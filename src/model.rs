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

    /// Kürzt die Nachricht auf eine maximale Länge
    pub fn truncated_message(&self, max_len: usize) -> String {
        if self.message.len() <= max_len {
            self.message.clone()
        } else {
            format!("{}...", &self.message[..max_len.saturating_sub(3)])
        }
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
            || self.date_from.is_some()
            || self.date_to.is_some()
    }

    pub fn clear(&mut self) {
        self.min_level = None;
        self.search_text = None;
        self.component_filter = None;
        self.request_id = None;
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
