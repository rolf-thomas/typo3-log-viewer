use crate::model::LogEntry;
use crate::parser::{parse_log_line, parse_log_stream};
use chrono::{Duration, Local};
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Ergebnis des Ladevorgangs
pub struct LoadResult {
    pub entries: Vec<LogEntry>,
    pub file_path: PathBuf,
    pub file_size: u64,
    /// Gesamtzahl der bisher konsumierten Zeilen.
    /// Wird für inkrementelle Reloads benötigt, damit die `line_number`
    /// neuer Einträge weiterhin der tatsächlichen Position in der Datei entspricht.
    pub lines_read: usize,
}

/// Lädt eine Log-Datei und parst deren Inhalt zeilenweise (streaming),
/// ohne den gesamten Dateiinhalt vorab in den Speicher zu laden.
pub fn load_log_file(path: &Path) -> io::Result<LoadResult> {
    let file = File::open(path)?;
    let file_size = file.metadata()?.len();

    let reader = BufReader::new(file);
    let (entries, lines_read) = parse_log_stream(reader, 0)?;

    Ok(LoadResult {
        entries,
        file_path: path.to_path_buf(),
        file_size,
        lines_read,
    })
}

/// Prüft, ob ein Dateiname zu einer (ggf. rotierten) Log-Datei gehört.
/// Erkennt sowohl `*.log` als auch rotierte Varianten wie `*.log.20260310163451`.
fn is_log_filename(name: &str) -> bool {
    if let Some(idx) = name.rfind(".log") {
        let after = &name[idx + ".log".len()..];
        // entweder genau ".log" am Ende ...
        if after.is_empty() {
            return true;
        }
        // ... oder ".log" gefolgt von einem Rotations-Suffix wie ".20260310163451"
        if after.starts_with('.') && after.len() > 1 {
            return true;
        }
    }
    false
}

/// Findet alle Log-Dateien in einem Verzeichnis (inkl. rotierter Dateien)
pub fn find_log_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut log_files = Vec::new();

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if is_log_filename(name) {
                        log_files.push(path);
                    }
                }
            }
        }
    }

    // Sortiere nach Dateiname
    log_files.sort();

    Ok(log_files)
}

/// Prüft, ob die letzte (= neuste) Zeile einer Log-Datei einem gültigen
/// Log-Eintrag entspricht, dessen Zeitstempel innerhalb der letzten 24 Stunden
/// liegt. Wird genutzt, um in der Dateiauswahl "aktuelle" Dateien grün zu
/// markieren.
///
/// Da Einträge mehrzeilig sein können (JSON-Folgezeilen), wird das Datei-Ende
/// gelesen und rückwärts nach der letzten Zeile gesucht, die als Header-Zeile
/// (mit Zeitstempel + Level) geparst werden kann.
pub fn is_log_file_recent(path: &Path) -> bool {
    let now = Local::now().fixed_offset();
    match last_log_timestamp(path) {
        Some(ts) => now.signed_duration_since(ts) <= Duration::days(1),
        None => false,
    }
}

/// Liefert den Zeitstempel des letzten (neusten) Log-Eintrags einer Datei,
/// indem nur das Datei-Ende gelesen wird (kein vollständiges Einlesen).
fn last_log_timestamp(path: &Path) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    /// Anzahl der Bytes, die vom Datei-Ende gelesen werden. Großzügig genug,
    /// um auch nach mehreren JSON-Folgezeilen die letzte Header-Zeile zu finden.
    const TAIL_BYTES: u64 = 64 * 1024;

    let mut file = File::open(path).ok()?;
    let file_size = file.metadata().ok()?.len();
    if file_size == 0 {
        return None;
    }

    let start = file_size.saturating_sub(TAIL_BYTES);
    file.seek(SeekFrom::Start(start)).ok()?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf).ok()?;

    // Verlustfrei genug: ungültige UTF-8-Bytes werden ersetzt, die für uns
    // relevanten Header-Zeilen (ASCII) bleiben intakt.
    let text = String::from_utf8_lossy(&buf);

    // Rückwärts nach der letzten parsebaren Header-Zeile suchen.
    text.lines()
        .rev()
        .find_map(|line| parse_log_line(line, 0))
        .map(|entry| entry.timestamp)
}

/// Formatiert eine Dateigröße human-readable
pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_log_filename() {
        assert!(is_log_filename("typo3.log"));
        assert!(is_log_filename("typo3_31214afefa.log"));
        assert!(is_log_filename("typo3_31214afefa.log.20260310163451"));
        assert!(is_log_filename("nginx.log.1"));
        assert!(!is_log_filename("readme.txt"));
        assert!(!is_log_filename("typo3.log."));
        assert!(!is_log_filename("notalog"));
        assert!(!is_log_filename(".logfile"));
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
        assert_eq!(format_file_size(2621440), "2.5 MB");
    }

    use std::io::Write;

    fn temp_log(name: &str, content: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("t3lv_{}_{}.log", std::process::id(), name));
        let mut f = File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    fn line_at(ts: &str) -> String {
        format!(
            r#"{} [INFO] request="r1" component="Vendor.Ext": message"#,
            ts
        )
    }

    #[test]
    fn recent_file_with_current_last_line_is_recent() {
        // Letzte Zeile trägt einen Zeitstempel "jetzt" (lokale Zeit).
        let now = Local::now();
        let ts = now.format("%a, %d %b %Y %H:%M:%S %z").to_string();
        let content = format!(
            "{}\n{}\n",
            line_at("Thu, 02 Apr 2020 12:00:00 +0200"),
            line_at(&ts)
        );
        let p = temp_log("recent", &content);
        assert!(is_log_file_recent(&p));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn old_file_is_not_recent() {
        let content = format!("{}\n", line_at("Thu, 02 Apr 2020 12:00:00 +0200"));
        let p = temp_log("old", &content);
        assert!(!is_log_file_recent(&p));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn recent_with_trailing_json_lines_still_detected() {
        // Letzte physische Zeile ist JSON – die Header-Zeile davor zählt.
        let now = Local::now();
        let ts = now.format("%a, %d %b %Y %H:%M:%S %z").to_string();
        let content = format!(
            "{}\n{{\"error\":\"boom\"}}\n\n",
            line_at(&ts)
        );
        let p = temp_log("json_tail", &content);
        assert!(is_log_file_recent(&p));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn empty_file_is_not_recent() {
        let p = temp_log("empty", "");
        assert!(!is_log_file_recent(&p));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn non_log_content_is_not_recent() {
        let p = temp_log("garbage", "just some text\nnot a log\n");
        assert!(!is_log_file_recent(&p));
        let _ = fs::remove_file(&p);
    }
}
