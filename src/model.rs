use chrono::{DateTime, FixedOffset, NaiveDate};
use std::fmt;

/// Log-Level entsprechend TYPO3 Logging
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Info,
    Debug,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "EMERGENCY" => Some(LogLevel::Emergency),
            "ALERT" => Some(LogLevel::Alert),
            "CRITICAL" => Some(LogLevel::Critical),
            "ERROR" => Some(LogLevel::Error),
            "WARNING" => Some(LogLevel::Warning),
            "NOTICE" => Some(LogLevel::Notice),
            "INFO" => Some(LogLevel::Info),
            "DEBUG" => Some(LogLevel::Debug),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Emergency => "EMERGENCY",
            LogLevel::Alert => "ALERT",
            LogLevel::Critical => "CRITICAL",
            LogLevel::Error => "ERROR",
            LogLevel::Warning => "WARNING",
            LogLevel::Notice => "NOTICE",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
        }
    }

    /// Numerischer Wert für Filterung (niedriger = wichtiger)
    pub fn severity(&self) -> u8 {
        match self {
            LogLevel::Emergency => 0,
            LogLevel::Alert => 1,
            LogLevel::Critical => 2,
            LogLevel::Error => 3,
            LogLevel::Warning => 4,
            LogLevel::Notice => 5,
            LogLevel::Info => 6,
            LogLevel::Debug => 7,
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Ein einzelner Log-Eintrag
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Zeitstempel des Log-Eintrags
    pub timestamp: DateTime<FixedOffset>,
    /// Log-Level
    pub level: LogLevel,
    /// Request-ID (optional)
    pub request_id: Option<String>,
    /// Component/Modul
    pub component: String,
    /// Die eigentliche Nachricht
    pub message: String,
    /// Zusätzliche Daten (z.B. JSON)
    pub extra_data: Option<String>,
    /// Originale Zeilennummer in der Datei
    pub line_number: usize,
}

impl LogEntry {
    /// Formatiert den Zeitstempel kurz für die Listen-Ansicht
    pub fn short_timestamp(&self) -> String {
        self.timestamp.format("%d.%m.%y %H:%M").to_string()
    }

    /// Formatiert den Zeitstempel vollständig für die Detail-Ansicht
    pub fn full_timestamp(&self) -> String {
        self.timestamp.format("%a, %d %b %Y %H:%M:%S %z").to_string()
    }

    /// Formatiert extra_data als hübsches JSON falls möglich
    pub fn formatted_extra_data(&self) -> Option<String> {
        self.extra_data.as_ref().map(|data| {
            // Versuche JSON zu parsen und hübsch zu formatieren
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                serde_json::to_string_pretty(&json).unwrap_or_else(|_| data.clone())
            } else {
                data.clone()
            }
        })
    }
}

/// Filter-Optionen für die Log-Anzeige
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    /// Minimales Log-Level (None = alle)
    pub min_level: Option<LogLevel>,
    /// Textsuche in Nachricht
    pub search_text: Option<String>,
    /// Filter nach Component
    pub component_filter: Option<String>,
    /// Fokus auf eine einzelne Request-ID
    pub request_id: Option<String>,
    /// Fokus auf gleiche Nachricht (Präfix vor JSON)
    pub message_prefix: Option<String>,
    /// Datum von (inkl. 00:00:00)
    pub date_from: Option<NaiveDate>,
    /// Datum bis (inkl. 23:59:59)
    pub date_to: Option<NaiveDate>,
}

impl LogFilter {
    pub fn matches(&self, entry: &LogEntry) -> bool {
        // Level-Filter
        if let Some(min_level) = &self.min_level {
            if entry.level.severity() > min_level.severity() {
                return false;
            }
        }

        // Text-Suche
        if let Some(search) = &self.search_text {
            let search_lower = search.to_lowercase();
            let message_lower = entry.message.to_lowercase();
            let component_lower = entry.component.to_lowercase();

            if !message_lower.contains(&search_lower)
                && !component_lower.contains(&search_lower) {
                return false;
            }
        }

        // Component-Filter
        if let Some(comp) = &self.component_filter {
            if !entry.component.to_lowercase().contains(&comp.to_lowercase()) {
                return false;
            }
        }

        // Request-Fokus
        if let Some(req_id) = &self.request_id {
            match &entry.request_id {
                Some(id) => {
                    if id != req_id {
                        return false;
                    }
                }
                None => return false,
            }
        }

        // Nachrichten-Präfix-Fokus
        if let Some(prefix) = &self.message_prefix {
            if message_prefix(&entry.message) != *prefix {
                return false;
            }
        }

        // Datumsbereich
        let entry_date = entry.timestamp.date_naive();
        if let Some(from) = self.date_from {
            if entry_date < from {
                return false;
            }
        }
        if let Some(to) = self.date_to {
            if entry_date > to {
                return false;
            }
        }

        true
    }

    pub fn is_active(&self) -> bool {
        self.min_level.is_some()
            || self.search_text.is_some()
            || self.component_filter.is_some()
            || self.request_id.is_some()
            || self.message_prefix.is_some()
            || self.date_from.is_some()
            || self.date_to.is_some()
    }

    pub fn clear(&mut self) {
        self.min_level = None;
        self.search_text = None;
        self.component_filter = None;
        self.request_id = None;
        self.message_prefix = None;
        self.date_from = None;
        self.date_to = None;
    }

    /// Beschreibung des aktiven Datumsfilters für die Anzeige
    pub fn date_label(&self) -> Option<String> {
        match (self.date_from, self.date_to) {
            (Some(f), Some(t)) if f == t => Some(format!("{}", f.format("%d.%m.%Y"))),
            (Some(f), Some(t)) => Some(format!("{} – {}", f.format("%d.%m.%Y"), t.format("%d.%m.%Y"))),
            (Some(f), None) => Some(format!("ab {}", f.format("%d.%m.%Y"))),
            (None, Some(t)) => Some(format!("bis {}", t.format("%d.%m.%Y"))),
            (None, None) => None,
        }
    }
}

/// Extrahiert den stabilen Nachricht-Präfix (vor dem ersten JSON-Block)
/// z.B. "Matched /auth/callback route" aus "Matched /auth/callback route - {…}"
pub fn message_prefix(message: &str) -> String {
    // Trenne am ersten " - {" oder " - [" (JSON folgt)
    for sep in [" - {", " - ["] {
        if let Some(pos) = message.find(sep) {
            return message[..pos].trim().to_string();
        }
    }
    // Kein JSON-Separator: ganze Nachricht (getrimmt)
    message.trim().to_string()
}

/// Parst ein Datum im Format TT.MM.JJJJ oder JJJJ-MM-TT
pub fn parse_date_input(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    // TT.MM.JJJJ
    if let Ok(d) = NaiveDate::parse_from_str(s, "%d.%m.%Y") {
        return Some(d);
    }
    // JJJJ-MM-TT
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    fn make_entry(level: LogLevel, message: &str, component: &str, request_id: Option<&str>, timestamp: &str) -> LogEntry {
        LogEntry {
            timestamp: DateTime::parse_from_str(timestamp, "%a, %d %b %Y %H:%M:%S %z").unwrap(),
            level,
            request_id: request_id.map(|s| s.to_string()),
            component: component.to_string(),
            message: message.to_string(),
            extra_data: None,
            line_number: 1,
        }
    }

    const TS: &str = "Thu, 02 Apr 2026 12:00:00 +0200";

    // --- LogLevel ---

    #[test]
    fn log_level_from_str_all_variants() {
        assert_eq!(LogLevel::from_str("EMERGENCY"), Some(LogLevel::Emergency));
        assert_eq!(LogLevel::from_str("ALERT"),     Some(LogLevel::Alert));
        assert_eq!(LogLevel::from_str("CRITICAL"),  Some(LogLevel::Critical));
        assert_eq!(LogLevel::from_str("ERROR"),     Some(LogLevel::Error));
        assert_eq!(LogLevel::from_str("WARNING"),   Some(LogLevel::Warning));
        assert_eq!(LogLevel::from_str("NOTICE"),    Some(LogLevel::Notice));
        assert_eq!(LogLevel::from_str("INFO"),      Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("DEBUG"),     Some(LogLevel::Debug));
    }

    #[test]
    fn log_level_from_str_case_insensitive() {
        assert_eq!(LogLevel::from_str("error"), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_str("Warning"), Some(LogLevel::Warning));
    }

    #[test]
    fn log_level_from_str_unknown_returns_none() {
        assert_eq!(LogLevel::from_str("VERBOSE"), None);
        assert_eq!(LogLevel::from_str(""), None);
    }

    #[test]
    fn log_level_severity_order() {
        assert!(LogLevel::Emergency.severity() < LogLevel::Alert.severity());
        assert!(LogLevel::Alert.severity()     < LogLevel::Critical.severity());
        assert!(LogLevel::Critical.severity()  < LogLevel::Error.severity());
        assert!(LogLevel::Error.severity()     < LogLevel::Warning.severity());
        assert!(LogLevel::Warning.severity()   < LogLevel::Notice.severity());
        assert!(LogLevel::Notice.severity()    < LogLevel::Info.severity());
        assert!(LogLevel::Info.severity()      < LogLevel::Debug.severity());
    }

    // --- LogEntry helpers ---

    #[test]
    fn short_timestamp_format() {
        let entry = make_entry(LogLevel::Info, "msg", "Comp", None, TS);
        assert_eq!(entry.short_timestamp(), "02.04.26 12:00");
    }

    #[test]
    fn full_timestamp_format() {
        let entry = make_entry(LogLevel::Info, "msg", "Comp", None, TS);
        assert_eq!(entry.full_timestamp(), "Thu, 02 Apr 2026 12:00:00 +0200");
    }

    #[test]
    fn formatted_extra_data_pretty_prints_json() {
        let mut entry = make_entry(LogLevel::Info, "msg", "Comp", None, TS);
        entry.extra_data = Some(r#"{"key":"value"}"#.to_string());
        let formatted = entry.formatted_extra_data().unwrap();
        assert!(formatted.contains('\n'), "should be pretty-printed");
        assert!(formatted.contains("\"key\""));
    }

    #[test]
    fn formatted_extra_data_returns_raw_when_not_json() {
        let mut entry = make_entry(LogLevel::Info, "msg", "Comp", None, TS);
        entry.extra_data = Some("plain text".to_string());
        assert_eq!(entry.formatted_extra_data().unwrap(), "plain text");
    }

    #[test]
    fn formatted_extra_data_none_when_no_extra() {
        let entry = make_entry(LogLevel::Info, "msg", "Comp", None, TS);
        assert!(entry.formatted_extra_data().is_none());
    }

    // --- LogFilter::matches ---

    #[test]
    fn filter_no_constraints_matches_everything() {
        let filter = LogFilter::default();
        let entry = make_entry(LogLevel::Debug, "any", "Any", None, TS);
        assert!(filter.matches(&entry));
    }

    #[test]
    fn filter_min_level_excludes_lower_severity() {
        let filter = LogFilter { min_level: Some(LogLevel::Warning), ..Default::default() };
        assert!(filter.matches(&make_entry(LogLevel::Error,   "m", "C", None, TS)));
        assert!(filter.matches(&make_entry(LogLevel::Warning, "m", "C", None, TS)));
        assert!(!filter.matches(&make_entry(LogLevel::Info,   "m", "C", None, TS)));
        assert!(!filter.matches(&make_entry(LogLevel::Debug,  "m", "C", None, TS)));
    }

    #[test]
    fn filter_search_text_matches_message_case_insensitive() {
        let filter = LogFilter { search_text: Some("TOKEN".to_string()), ..Default::default() };
        assert!(filter.matches(&make_entry(LogLevel::Info, "Invalid token received", "C", None, TS)));
        assert!(!filter.matches(&make_entry(LogLevel::Info, "No match here", "C", None, TS)));
    }

    #[test]
    fn filter_search_text_matches_component() {
        let filter = LogFilter { search_text: Some("authservice".to_string()), ..Default::default() };
        assert!(filter.matches(&make_entry(LogLevel::Info, "msg", "Vendor.AuthService", None, TS)));
        assert!(!filter.matches(&make_entry(LogLevel::Info, "msg", "Vendor.Other", None, TS)));
    }

    #[test]
    fn filter_component_filter_partial_match() {
        let filter = LogFilter { component_filter: Some("sugar".to_string()), ..Default::default() };
        assert!(filter.matches(&make_entry(LogLevel::Info, "m", "WeberHaus.SugarCrmService", None, TS)));
        assert!(!filter.matches(&make_entry(LogLevel::Info, "m", "WeberHaus.OtherService", None, TS)));
    }

    #[test]
    fn filter_request_id_matches_exact() {
        let filter = LogFilter { request_id: Some("abc123".to_string()), ..Default::default() };
        assert!(filter.matches(&make_entry(LogLevel::Info, "m", "C", Some("abc123"), TS)));
        assert!(!filter.matches(&make_entry(LogLevel::Info, "m", "C", Some("xyz999"), TS)));
        assert!(!filter.matches(&make_entry(LogLevel::Info, "m", "C", None, TS)));
    }

    #[test]
    fn filter_message_prefix_matches() {
        let filter = LogFilter { message_prefix: Some("crmRequest".to_string()), ..Default::default() };
        assert!(filter.matches(&make_entry(LogLevel::Info, "crmRequest - {\"key\":1}", "C", None, TS)));
        assert!(!filter.matches(&make_entry(LogLevel::Info, "otherMessage - {\"key\":1}", "C", None, TS)));
    }

    #[test]
    fn filter_date_from_excludes_earlier_entries() {
        use chrono::NaiveDate;
        let filter = LogFilter {
            date_from: Some(NaiveDate::from_ymd_opt(2026, 4, 3).unwrap()),
            ..Default::default()
        };
        // Entry is 2026-04-02 — should be excluded
        assert!(!filter.matches(&make_entry(LogLevel::Info, "m", "C", None, TS)));
        // Entry on the same day as from — should pass
        let entry_on_date = make_entry(LogLevel::Info, "m", "C", None, "Fri, 03 Apr 2026 08:00:00 +0200");
        assert!(filter.matches(&entry_on_date));
    }

    #[test]
    fn filter_date_to_excludes_later_entries() {
        use chrono::NaiveDate;
        let filter = LogFilter {
            date_to: Some(NaiveDate::from_ymd_opt(2026, 4, 1).unwrap()),
            ..Default::default()
        };
        assert!(!filter.matches(&make_entry(LogLevel::Info, "m", "C", None, TS)));
        let entry_before = make_entry(LogLevel::Info, "m", "C", None, "Wed, 01 Apr 2026 08:00:00 +0200");
        assert!(filter.matches(&entry_before));
    }

    #[test]
    fn filter_combined_level_and_search() {
        let filter = LogFilter {
            min_level: Some(LogLevel::Warning),
            search_text: Some("token".to_string()),
            ..Default::default()
        };
        assert!(filter.matches(&make_entry(LogLevel::Error, "bad token", "C", None, TS)));
        assert!(!filter.matches(&make_entry(LogLevel::Info,  "bad token", "C", None, TS)));
        assert!(!filter.matches(&make_entry(LogLevel::Error, "no match",  "C", None, TS)));
    }

    // --- LogFilter::is_active ---

    #[test]
    fn filter_is_active_default_is_false() {
        assert!(!LogFilter::default().is_active());
    }

    #[test]
    fn filter_is_active_true_when_any_field_set() {
        assert!(LogFilter { min_level: Some(LogLevel::Error), ..Default::default() }.is_active());
        assert!(LogFilter { search_text: Some("x".to_string()), ..Default::default() }.is_active());
        assert!(LogFilter { component_filter: Some("x".to_string()), ..Default::default() }.is_active());
        assert!(LogFilter { request_id: Some("x".to_string()), ..Default::default() }.is_active());
        assert!(LogFilter { message_prefix: Some("x".to_string()), ..Default::default() }.is_active());
    }

    // --- LogFilter::clear ---

    #[test]
    fn filter_clear_resets_all_fields() {
        let mut filter = LogFilter {
            min_level: Some(LogLevel::Error),
            search_text: Some("x".to_string()),
            component_filter: Some("y".to_string()),
            request_id: Some("z".to_string()),
            message_prefix: Some("p".to_string()),
            ..Default::default()
        };
        filter.clear();
        assert!(!filter.is_active());
    }

    // --- LogFilter::date_label ---

    #[test]
    fn date_label_none_when_no_dates() {
        assert!(LogFilter::default().date_label().is_none());
    }

    #[test]
    fn date_label_single_day() {
        use chrono::NaiveDate;
        let d = NaiveDate::from_ymd_opt(2026, 4, 2).unwrap();
        let filter = LogFilter { date_from: Some(d), date_to: Some(d), ..Default::default() };
        assert_eq!(filter.date_label().unwrap(), "02.04.2026");
    }

    #[test]
    fn date_label_range() {
        use chrono::NaiveDate;
        let from = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let to   = NaiveDate::from_ymd_opt(2026, 4, 5).unwrap();
        let filter = LogFilter { date_from: Some(from), date_to: Some(to), ..Default::default() };
        assert_eq!(filter.date_label().unwrap(), "01.04.2026 \u{2013} 05.04.2026");
    }

    #[test]
    fn date_label_only_from() {
        use chrono::NaiveDate;
        let d = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let filter = LogFilter { date_from: Some(d), ..Default::default() };
        assert_eq!(filter.date_label().unwrap(), "ab 01.04.2026");
    }

    #[test]
    fn date_label_only_to() {
        use chrono::NaiveDate;
        let d = NaiveDate::from_ymd_opt(2026, 4, 5).unwrap();
        let filter = LogFilter { date_to: Some(d), ..Default::default() };
        assert_eq!(filter.date_label().unwrap(), "bis 05.04.2026");
    }

    // --- message_prefix ---

    #[test]
    fn message_prefix_strips_json_object() {
        assert_eq!(message_prefix("crmRequest - {\"key\":1}"), "crmRequest");
    }

    #[test]
    fn message_prefix_strips_json_array() {
        assert_eq!(message_prefix("items - [{\"id\":1}]"), "items");
    }

    #[test]
    fn message_prefix_no_json_returns_full_message() {
        assert_eq!(message_prefix("Simple error message"), "Simple error message");
    }

    #[test]
    fn message_prefix_trims_whitespace() {
        assert_eq!(message_prefix("  trimmed  "), "trimmed");
    }

    // --- parse_date_input ---

    #[test]
    fn parse_date_input_german_format() {
        use chrono::NaiveDate;
        assert_eq!(
            parse_date_input("02.04.2026"),
            Some(NaiveDate::from_ymd_opt(2026, 4, 2).unwrap())
        );
    }

    #[test]
    fn parse_date_input_iso_format() {
        use chrono::NaiveDate;
        assert_eq!(
            parse_date_input("2026-04-02"),
            Some(NaiveDate::from_ymd_opt(2026, 4, 2).unwrap())
        );
    }

    #[test]
    fn parse_date_input_trims_whitespace() {
        use chrono::NaiveDate;
        assert_eq!(
            parse_date_input("  2026-04-02  "),
            Some(NaiveDate::from_ymd_opt(2026, 4, 2).unwrap())
        );
    }

    #[test]
    fn parse_date_input_invalid_returns_none() {
        assert!(parse_date_input("not-a-date").is_none());
        assert!(parse_date_input("").is_none());
    }
}
