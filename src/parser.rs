use crate::model::{LogEntry, LogLevel};
use chrono::DateTime;
use once_cell::sync::Lazy;
use regex::Regex;
use std::io::{self, BufRead};

/// Regex für das TYPO3 Log-Format:
/// DATUM [LEVEL] request="REQUEST_ID" component="COMPONENT": NACHRICHT
///
/// Beispiel:
/// Thu, 02 Apr 2026 12:00:02 +0200 [ERROR] request="043d54b20b2e8" component="Vendor.Extension": Message
static LOG_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"^([A-Za-z]{3}, \d{2} [A-Za-z]{3} \d{4} \d{2}:\d{2}:\d{2} [+-]\d{4}) \[([A-Z]+)\] request="([^"]*)" component="([^"]*)": (.*)$"#
    ).expect("Invalid regex pattern")
});

/// Parst einen einzelnen Log-Eintrag aus einer Zeile
pub fn parse_log_line(line: &str, line_number: usize) -> Option<LogEntry> {
    let captures = LOG_REGEX.captures(line)?;

    // Zeitstempel parsen
    let timestamp_str = captures.get(1)?.as_str();
    let timestamp = DateTime::parse_from_str(timestamp_str, "%a, %d %b %Y %H:%M:%S %z").ok()?;

    // Log-Level parsen
    let level_str = captures.get(2)?.as_str();
    let level = LogLevel::from_str(level_str)?;

    // Request-ID
    let request_id = captures.get(3).map(|m| m.as_str().to_string());
    let request_id = if request_id.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        None
    } else {
        request_id
    };

    // Component
    let component = captures.get(4)?.as_str().to_string();

    // Nachricht
    let message = captures.get(5)?.as_str().to_string();

    Some(LogEntry {
        timestamp,
        level,
        request_id,
        component,
        message,
        extra_data: None,
        line_number,
    })
}

/// Streaming-Parser-Zustand. Erlaubt es, eine Log-Datei häppchenweise
/// (z.B. zeilenweise via `BufReader`) zu parsen, ohne den gesamten Inhalt
/// im Speicher halten zu müssen. Gleichzeitig kann der Zustand zwischen
/// inkrementellen Reload-Aufrufen erhalten bleiben.
pub struct StreamParser {
    /// Bisher konsumierte Zeilen (1-basierte nächste Zeilennummer = lines_consumed + 1)
    lines_consumed: usize,
    /// Aktuell offener Eintrag, dem noch Extra-Zeilen folgen können
    pending: Option<LogEntry>,
    /// Gesammelte Extra-Zeilen für `pending`
    extras: Vec<String>,
}

impl StreamParser {
    /// Erzeugt einen Parser, der nach `lines_offset` Zeilen weitermacht
    /// (0 für eine frische Datei). Wird für inkrementelle Reloads benötigt,
    /// damit die `line_number` in `LogEntry` mit der tatsächlichen Position
    /// in der Datei übereinstimmt.
    pub fn with_line_offset(lines_offset: usize) -> Self {
        Self {
            lines_consumed: lines_offset,
            pending: None,
            extras: Vec::new(),
        }
    }

    pub fn lines_consumed(&self) -> usize {
        self.lines_consumed
    }

    /// Verarbeitet eine einzelne Zeile (ohne Zeilenumbruch).
    /// Liefert ggf. einen abgeschlossenen Eintrag zurück, wenn diese
    /// Zeile einen neuen Eintrag startet und der vorherige damit fertig ist.
    pub fn feed_line(&mut self, line: &str) -> Option<LogEntry> {
        self.lines_consumed += 1;
        let line_number = self.lines_consumed;

        // Versuch: ist das eine neue Eintrags-Header-Zeile?
        if let Some(new_entry) = parse_log_line(line, line_number) {
            // Vorherigen Eintrag abschließen
            let finished = self.finalize_pending();
            self.pending = Some(new_entry);
            self.extras.clear();
            return finished;
        }

        // Keine Header-Zeile: gehört entweder zum aktuellen Eintrag (Extra-Daten)
        // oder ist eine verwaiste Zeile (kein offener Eintrag).
        if self.pending.is_some() {
            // Führende Leerzeilen überspringen, sonst sammeln
            if !line.trim().is_empty() || !self.extras.is_empty() {
                self.extras.push(line.to_string());
            }
        }
        // Verwaiste Zeile ohne offenen Eintrag → ignorieren (wie zuvor).
        None
    }

    /// Schließt den letzten offenen Eintrag ab und liefert ihn zurück.
    /// Sollte am Ende des Streams (EOF) aufgerufen werden.
    pub fn finish(mut self) -> Option<LogEntry> {
        self.finalize_pending()
    }

    fn finalize_pending(&mut self) -> Option<LogEntry> {
        let mut entry = self.pending.take()?;

        // Trailing leere Zeilen entfernen
        while self
            .extras
            .last()
            .map(|s| s.trim().is_empty())
            .unwrap_or(false)
        {
            self.extras.pop();
        }

        if !self.extras.is_empty() {
            entry.extra_data = Some(std::mem::take(&mut self.extras).join("\n"));
        } else {
            self.extras.clear();
        }

        Some(entry)
    }
}

/// Streaming-Variante: liest aus einem `BufRead` zeilenweise und parst
/// dabei sukzessive Einträge. Die Datei wird nicht komplett in den Speicher geladen.
///
/// `line_offset` gibt an, wie viele Zeilen vor dem aktuellen Reader-Anfang
/// bereits konsumiert wurden (0 für eine frische Datei). Dadurch bleibt
/// `LogEntry::line_number` über inkrementelle Reloads hinweg konsistent.
///
/// Liefert die geparsten Einträge sowie die Gesamtzahl der konsumierten Zeilen
/// (inkl. `line_offset`).
pub fn parse_log_stream<R: BufRead>(
    mut reader: R,
    line_offset: usize,
) -> io::Result<(Vec<LogEntry>, usize)> {
    let mut parser = StreamParser::with_line_offset(line_offset);
    let mut entries = Vec::new();
    let mut buf = String::new();

    loop {
        buf.clear();
        let n = reader.read_line(&mut buf)?;
        if n == 0 {
            break; // EOF
        }
        // `read_line` liefert den abschließenden Zeilenumbruch mit;
        // `lines()` (das vorher genutzt wurde) tut das nicht. Konsistenz herstellen.
        let trimmed = strip_line_ending(&buf);
        if let Some(entry) = parser.feed_line(trimmed) {
            entries.push(entry);
        }
    }

    let lines_consumed = parser.lines_consumed();
    if let Some(last) = parser.finish() {
        entries.push(last);
    }

    Ok((entries, lines_consumed))
}

fn strip_line_ending(s: &str) -> &str {
    if let Some(stripped) = s.strip_suffix('\n') {
        stripped.strip_suffix('\r').unwrap_or(stripped)
    } else {
        s
    }
}

/// Parst mehrere Zeilen und kombiniert mehrzeilige Einträge.
/// Nutzt intern den Streaming-Parser, akzeptiert aber bequem einen `&str`.
/// (Wird im Binary nicht verwendet — nur als Convenience für Tests / Aufrufer
/// mit bereits geladenem String.)
#[allow(dead_code)]
pub fn parse_log_content(content: &str) -> Vec<LogEntry> {
    let cursor = io::Cursor::new(content.as_bytes());
    parse_log_stream(cursor, 0)
        .map(|(entries, _)| entries)
        .unwrap_or_default()
}

/// Versucht JSON in der Nachricht zu erkennen und zu extrahieren
/// Gibt (Text vor JSON, formatiertes JSON) zurück
pub fn extract_json_from_message(message: &str) -> Option<(String, String)> {
    // Suche nach JSON-Start (Object oder Array)
    let start_obj = message.find('{');
    let start_arr = message.find('[');

    let start = match (start_obj, start_arr) {
        (Some(o), Some(a)) => Some(o.min(a)),
        (Some(o), None) => Some(o),
        (None, Some(a)) => Some(a),
        (None, None) => None,
    }?;

    let json_part = &message[start..];

    // Versuche zu parsen und formatieren
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_part) {
        let prefix = message[..start].trim_end_matches(|c| c == ' ' || c == '-' || c == ':');
        let formatted = serde_json::to_string_pretty(&json).unwrap_or_else(|_| json_part.to_string());
        return Some((prefix.to_string(), formatted));
    }

    // Falls das nicht funktioniert, versuche nur bis zum Ende der Zeile
    if let Some(end) = json_part.find('\n') {
        let json_line = &json_part[..end];
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_line) {
            let prefix = message[..start].trim_end_matches(|c| c == ' ' || c == '-' || c == ':');
            let formatted = serde_json::to_string_pretty(&json).unwrap_or_else(|_| json_line.to_string());
            return Some((prefix.to_string(), formatted));
        }
    }

    None
}

#[allow(dead_code)]
pub fn extract_all_json(text: &str) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' || chars[i] == '[' {
            let start = i;
            let open_char = chars[i];
            let close_char = if open_char == '{' { '}' } else { ']' };
            let mut depth = 1;
            let mut in_string = false;
            let mut escape_next = false;
            i += 1;

            while i < chars.len() && depth > 0 {
                let c = chars[i];

                if escape_next {
                    escape_next = false;
                } else if c == '\\' && in_string {
                    escape_next = true;
                } else if c == '"' {
                    in_string = !in_string;
                } else if !in_string {
                    if c == open_char {
                        depth += 1;
                    } else if c == close_char {
                        depth -= 1;
                    }
                }
                i += 1;
            }

            if depth == 0 {
                let json_str: String = chars[start..i].iter().collect();
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    let formatted = serde_json::to_string_pretty(&json)
                        .unwrap_or_else(|_| json_str.clone());
                    results.push((start, formatted));
                }
            }
        } else {
            i += 1;
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LogLevel;

    const TS: &str = "Thu, 02 Apr 2026";

    fn line(level: &str, msg: &str) -> String {
        format!(r#"{} 12:00:02 +0200 [{}] request="req1" component="Vendor.Ext": {}"#, TS, level, msg)
    }

    // --- parse_log_line: alle Log-Level ---

    #[test]
    fn test_parse_error_log() {
        let l = r#"Thu, 02 Apr 2026 12:00:02 +0200 [ERROR] request="043d54b20b2e8" component="Vendor.Extension.Service.ProductCrmRestService": Client error"#;
        let entry = parse_log_line(l, 1).expect("Should parse");
        assert_eq!(entry.level, LogLevel::Error);
        assert_eq!(entry.request_id, Some("043d54b20b2e8".to_string()));
        assert!(entry.component.contains("ProductCrmRestService"));
        assert_eq!(entry.message, "Client error");
    }

    #[test]
    fn test_parse_debug_log() {
        let l = r#"Thu, 02 Apr 2026 11:06:55 +0200 [DEBUG] request="a03b3f7c34daa" component="Vendor.Extension.Service.ProductCrmRestService": crmRequest - {"endpoint":"https://example.com"}"#;
        let entry = parse_log_line(l, 1).expect("Should parse");
        assert_eq!(entry.level, LogLevel::Debug);
        assert!(entry.message.contains("crmRequest"));
    }

    #[test]
    fn test_parse_warning_log() {
        let entry = parse_log_line(&line("WARNING", "Low disk space"), 1).expect("Should parse");
        assert_eq!(entry.level, LogLevel::Warning);
    }

    #[test]
    fn test_parse_info_log() {
        let entry = parse_log_line(&line("INFO", "Started"), 1).expect("Should parse");
        assert_eq!(entry.level, LogLevel::Info);
    }

    #[test]
    fn test_parse_notice_log() {
        let entry = parse_log_line(&line("NOTICE", "Cache cleared"), 1).expect("Should parse");
        assert_eq!(entry.level, LogLevel::Notice);
    }

    #[test]
    fn test_parse_critical_log() {
        let entry = parse_log_line(&line("CRITICAL", "DB unreachable"), 1).expect("Should parse");
        assert_eq!(entry.level, LogLevel::Critical);
    }

    #[test]
    fn test_parse_alert_log() {
        let entry = parse_log_line(&line("ALERT", "Disk full"), 1).expect("Should parse");
        assert_eq!(entry.level, LogLevel::Alert);
    }

    #[test]
    fn test_parse_emergency_log() {
        let entry = parse_log_line(&line("EMERGENCY", "System down"), 1).expect("Should parse");
        assert_eq!(entry.level, LogLevel::Emergency);
    }

    // --- parse_log_line: leere request_id ---

    #[test]
    fn test_parse_empty_request_id_becomes_none() {
        let l = r#"Thu, 02 Apr 2026 12:00:02 +0200 [INFO] request="" component="Vendor.Ext": msg"#;
        let entry = parse_log_line(l, 1).expect("Should parse");
        assert!(entry.request_id.is_none());
    }

    // --- parse_log_line: ungültige Zeilen ---

    #[test]
    fn test_parse_invalid_line_returns_none() {
        assert!(parse_log_line("this is not a log line", 1).is_none());
        assert!(parse_log_line("", 1).is_none());
        assert!(parse_log_line(r#"{"json":"only"}"#, 1).is_none());
    }

    #[test]
    fn test_parse_line_number_is_stored() {
        let entry = parse_log_line(&line("INFO", "msg"), 42).expect("Should parse");
        assert_eq!(entry.line_number, 42);
    }

    // --- parse_log_line: Multiline ---

    #[test]
    fn test_parse_multiline() {
        let content = r#"Thu, 02 Apr 2026 12:00:02 +0200 [ERROR] request="abc123" component="Test": Error occurred
{"error":"invalid_grant","error_message":"Invalid token"}

Thu, 02 Apr 2026 13:00:02 +0200 [ERROR] request="def456" component="Test": Another error"#;

        let entries = parse_log_content(content);

        assert_eq!(entries.len(), 2);
        assert!(entries[0].extra_data.is_some());
        assert!(entries[0].extra_data.as_ref().unwrap().contains("invalid_grant"));
        assert!(entries[1].extra_data.is_none());
    }

    #[test]
    fn test_parse_multiple_extra_lines() {
        let content = "Thu, 02 Apr 2026 12:00:00 +0200 [ERROR] request=\"r1\" component=\"C\": msg\nline2\nline3\n\nThu, 02 Apr 2026 13:00:00 +0200 [INFO] request=\"r2\" component=\"C\": second";
        let entries = parse_log_content(content);
        assert_eq!(entries.len(), 2);
        let extra = entries[0].extra_data.as_ref().unwrap();
        assert!(extra.contains("line2"));
        assert!(extra.contains("line3"));
    }

    #[test]
    fn test_parse_trailing_empty_lines_stripped_from_extra() {
        let content = "Thu, 02 Apr 2026 12:00:00 +0200 [ERROR] request=\"r1\" component=\"C\": msg\nextra\n\n";
        let entries = parse_log_content(content);
        assert_eq!(entries.len(), 1);
        let extra = entries[0].extra_data.as_ref().unwrap();
        assert!(!extra.ends_with('\n'));
    }

    // --- StreamParser direkt ---

    #[test]
    fn stream_parser_single_entry_via_finish() {
        let mut p = StreamParser::with_line_offset(0);
        assert!(p.feed_line(&line("ERROR", "msg")).is_none());
        let consumed = p.lines_consumed();
        let entry = p.finish().expect("Should yield entry");
        assert_eq!(entry.level, LogLevel::Error);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn stream_parser_orphan_lines_are_ignored() {
        let mut p = StreamParser::with_line_offset(0);
        // extra line before any header
        assert!(p.feed_line("orphan line").is_none());
        assert!(p.finish().is_none());
    }

    #[test]
    fn stream_parser_line_offset_is_respected() {
        let mut p = StreamParser::with_line_offset(10);
        p.feed_line(&line("INFO", "msg"));
        let entry = p.finish().unwrap();
        assert_eq!(entry.line_number, 11);
    }

    #[test]
    fn stream_parser_lines_consumed_tracks_all_lines() {
        let mut p = StreamParser::with_line_offset(0);
        p.feed_line(&line("ERROR", "first"));
        p.feed_line("extra data");
        p.feed_line(&line("INFO", "second"));
        let consumed = p.lines_consumed();
        p.finish();
        assert_eq!(consumed, 3);
    }

    // --- strip_line_ending (indirekt via parse_log_stream mit CRLF-Input) ---

    #[test]
    fn parse_log_stream_handles_crlf_line_endings() {
        let content = format!("{}\r\n", line("ERROR", "msg"));
        let cursor = std::io::Cursor::new(content.as_bytes());
        let (entries, _) = parse_log_stream(cursor, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "msg");
    }

    // --- extract_json_from_message ---

    #[test]
    fn extract_json_from_message_finds_json_object() {
        let msg = r#"Request failed: {"code":404,"reason":"not found"}"#;
        let (prefix, json) = extract_json_from_message(msg).expect("Should find JSON");
        assert_eq!(prefix, "Request failed");
        assert!(json.contains("404"));
    }

    #[test]
    fn extract_json_from_message_finds_json_array() {
        let msg = r#"items: [1,2,3]"#;
        let (prefix, json) = extract_json_from_message(msg).expect("Should find JSON");
        assert!(json.contains('1'));
        assert!(!prefix.is_empty());
    }

    #[test]
    fn extract_json_from_message_returns_none_without_json() {
        assert!(extract_json_from_message("plain error message").is_none());
    }

    #[test]
    fn extract_json_from_message_returns_none_for_invalid_json() {
        assert!(extract_json_from_message("prefix {not valid json}").is_none());
    }

    #[test]
    fn extract_json_from_message_formats_pretty() {
        let msg = r#"msg {"a":1,"b":2}"#;
        let (_, json) = extract_json_from_message(msg).unwrap();
        assert!(json.contains('\n'), "should be pretty-printed");
    }
}
