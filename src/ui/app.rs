use crate::loader::{format_file_size, LoadResult};
use crate::model::{message_prefix, parse_date_input, LogEntry, LogFilter, LogLevel};
use crate::parser::extract_json_from_message;
use chrono::{Datelike, Local, NaiveDate};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};
use std::collections::HashSet;
use std::io;
use std::path::PathBuf;

/// Aktuelle Ansicht
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    List,
    Detail,
    #[allow(dead_code)]
    Filter,
    Help,
    DateMenu,
}

/// Input-Modus für Filter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    None,
    Search,
    #[allow(dead_code)]
    Level,
    #[allow(dead_code)]
    Component,
    DateFrom,
    DateTo,
}

/// Haupt-App-Zustand
pub struct App {
    /// Alle geladenen Log-Einträge
    pub entries: Vec<LogEntry>,
    /// Gefilterte Indizes
    pub filtered_indices: Vec<usize>,
    /// Aktiver Filter
    pub filter: LogFilter,
    /// Dateiinformationen
    pub file_path: PathBuf,
    pub file_size: u64,
    /// Listen-Zustand
    pub list_state: ListState,
    /// Aktuelle Ansicht
    pub view: AppView,
    /// Filter-Modus
    pub filter_mode: FilterMode,
    /// Filter-Eingabe
    pub filter_input: String,
    /// Scroll-Position in Detail-Ansicht
    pub detail_scroll: u16,
    /// Anzahl sichtbarer Zeilen in der Listenansicht (zuletzt gerendert)
    pub visible_rows: usize,
    /// "exception"-Key in JSON-Detailansicht aufgeklappt
    pub show_exception: bool,
    /// Soll die App beendet werden?
    pub should_quit: bool,
    /// Soll zur Dateiauswahl zurückgekehrt werden?
    pub should_go_back: bool,
    /// Dateiauswahl war verfügbar (Verzeichnis mit mehreren Dateien)
    pub has_file_selector: bool,
    /// Zwischenspeicher für Von-Datum bei der Bereichseingabe
    date_from_input: String,
    /// Zeilennummern von Einträgen, die seit dem Start neu hinzugekommen sind
    pub new_line_numbers: HashSet<usize>,
    /// Bereits gesehene Zeilennummern (für Diff-Berechnung beim Reload)
    seen_line_numbers: HashSet<usize>,
    /// Tail-Modus aktiv: bei neuen Einträgen wird automatisch nachgescrollt,
    /// solange die markierte Zeile nicht am oberen Fensterrand angekommen ist.
    /// Wird durch manuelle Navigation deaktiviert und durch End reaktiviert.
    auto_tail: bool,
}

impl App {
    pub fn new(result: LoadResult) -> Self {
        let filtered_indices: Vec<usize> = (0..result.entries.len()).collect();
        let seen_line_numbers: HashSet<usize> =
            result.entries.iter().map(|e| e.line_number).collect();

        let mut app = App {
            entries: result.entries,
            filtered_indices,
            filter: LogFilter::default(),
            file_path: result.file_path,
            file_size: result.file_size,
            list_state: ListState::default(),
            view: AppView::List,
            filter_mode: FilterMode::None,
            filter_input: String::new(),
            detail_scroll: 0,
            visible_rows: 20,
            show_exception: false,
            should_quit: false,
            should_go_back: false,
            has_file_selector: false,
            date_from_input: String::new(),
            new_line_numbers: HashSet::new(),
            seen_line_numbers,
            auto_tail: true,
        };

        // Wähle den letzten (neuesten) Eintrag
        if !app.filtered_indices.is_empty() {
            app.list_state.select(Some(app.filtered_indices.len() - 1));
        }

        app
    }

    /// Wendet den Filter an und aktualisiert filtered_indices
    pub fn apply_filter(&mut self) {
        self.filtered_indices = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| self.filter.matches(entry))
            .map(|(i, _)| i)
            .collect();

        // Selektion anpassen
        if self.filtered_indices.is_empty() {
            self.list_state.select(None);
        } else {
            match self.list_state.selected() {
                None => {
                    self.list_state.select(Some(0));
                }
                Some(current) if current >= self.filtered_indices.len() => {
                    self.list_state.select(Some(self.filtered_indices.len() - 1));
                }
                _ => {}
            }
        }
    }

    /// Gibt den aktuell ausgewählten Log-Eintrag zurück
    pub fn selected_entry(&self) -> Option<&LogEntry> {
        self.list_state
            .selected()
            .and_then(|i| self.filtered_indices.get(i))
            .and_then(|&idx| self.entries.get(idx))
    }

    /// Navigation: nach oben
    pub fn move_up(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let selected = self.list_state.selected().unwrap_or(0);
        self.list_state.select(Some(selected.saturating_sub(1)));
        self.auto_tail = false;
    }

    /// Navigation: nach unten
    pub fn move_down(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let selected = self.list_state.selected().unwrap_or(0);
        let next = (selected + 1).min(self.filtered_indices.len() - 1);
        self.list_state.select(Some(next));
        self.auto_tail = next + 1 == self.filtered_indices.len();
    }

    /// Navigation: Seite hoch
    pub fn page_up(&mut self, page_size: usize) {
        if let Some(selected) = self.list_state.selected() {
            let new_selected = selected.saturating_sub(page_size);
            self.list_state.select(Some(new_selected));
            self.auto_tail = false;
        }
    }

    /// Navigation: Seite runter
    pub fn page_down(&mut self, page_size: usize) {
        if let Some(selected) = self.list_state.selected() {
            let new_selected = (selected + page_size).min(self.filtered_indices.len().saturating_sub(1));
            self.list_state.select(Some(new_selected));
            self.auto_tail = new_selected + 1 == self.filtered_indices.len();
        }
    }

    /// Navigation: zum Anfang
    pub fn go_to_start(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
            self.auto_tail = false;
        }
    }

    /// Navigation: zum Ende
    pub fn go_to_end(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(self.filtered_indices.len() - 1));
            self.auto_tail = true;
        }
    }

    /// Setzt Level-Filter
    pub fn set_level_filter(&mut self, level: Option<LogLevel>) {
        self.filter.min_level = level;
        self.apply_filter();
    }

    /// Setzt Textsuche
    pub fn set_search_filter(&mut self, search: Option<String>) {
        self.filter.search_text = search;
        self.apply_filter();
    }

    /// Löscht alle Filter
    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.apply_filter();
    }

    /// Setzt einen Datumsbereich-Filter
    pub fn set_date_range(&mut self, from: Option<NaiveDate>, to: Option<NaiveDate>) {
        self.filter.date_from = from;
        self.filter.date_to = to;
        self.apply_filter();
    }

    /// Schnellfilter: heute
    pub fn filter_today(&mut self) {
        let today = Local::now().date_naive();
        self.set_date_range(Some(today), Some(today));
    }

    /// Schnellfilter: letzter Kalendermonat
    pub fn filter_last_month(&mut self) {
        let today = Local::now().date_naive();
        let first_this_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        let last_month_end = first_this_month.pred_opt().unwrap();
        let last_month_start = NaiveDate::from_ymd_opt(last_month_end.year(), last_month_end.month(), 1).unwrap();
        self.set_date_range(Some(last_month_start), Some(last_month_end));
    }

    /// Schnellfilter: letzte N Monate (rollierend ab heute)
    pub fn filter_last_months(&mut self, months: u32) {
        let today = Local::now().date_naive();
        let from = subtract_months(today, months);
        self.set_date_range(Some(from), Some(today));
    }

    /// Fokussiert auf gleiche Nachrichten (selber Präfix vor JSON)
    pub fn set_message_focus(&mut self) {
        if let Some(entry) = self.selected_entry() {
            let prefix = message_prefix(&entry.message);
            if !prefix.is_empty() {
                self.filter.message_prefix = Some(prefix);
                self.apply_filter();
                self.list_state.select(Some(0));
                self.auto_tail = false;
            }
        }
    }

    /// Fokussiert auf die Request-ID des aktuell gewählten Eintrags
    pub fn set_request_focus(&mut self) {
        if let Some(req_id) = self.selected_entry().and_then(|e| e.request_id.clone()) {
            self.filter.request_id = Some(req_id);
            self.apply_filter();
            // Selektion auf ersten Eintrag des Requests setzen
            self.list_state.select(Some(0));
            self.auto_tail = false;
        }
    }

    /// Prüft, ob sich die Datei geändert hat, und lädt sie neu.
    /// Die aktuelle Selektion wird per line_number wiederhergestellt,
    /// sodass neue Einträge die Auswahl nicht verändern.
    pub fn reload_if_changed(&mut self) -> io::Result<bool> {
        let metadata = match std::fs::metadata(&self.file_path) {
            Ok(m) => m,
            Err(_) => return Ok(false), // Datei kurzzeitig nicht verfügbar — nächster Tick
        };
        let current_size = metadata.len();

        if current_size == self.file_size {
            return Ok(false);
        }

        // Aktuelle Auswahl per stabilem Kriterium (line_number) merken
        let selected_line_number = self.selected_entry().map(|e| e.line_number);

        // Tail-Modus und vorherige Listenlänge merken, um danach
        // ggf. weiterzuscrollen
        let was_tailing = self.auto_tail;
        let prev_filtered_len = self.filtered_indices.len();
        let file_shrunk = current_size < self.file_size;

        // Datei neu laden und parsen
        let content = std::fs::read_to_string(&self.file_path)?;
        let new_entries = crate::parser::parse_log_content(&content);

        // Neue Einträge ermitteln: bei Logrotation (Datei geschrumpft)
        // alten Stand verwerfen, damit erneut auftretende Zeilennummern
        // nicht fälschlich als bekannt durchrutschen.
        if file_shrunk {
            self.seen_line_numbers.clear();
            self.new_line_numbers.clear();
            for entry in &new_entries {
                self.seen_line_numbers.insert(entry.line_number);
            }
        } else {
            for entry in &new_entries {
                if !self.seen_line_numbers.contains(&entry.line_number) {
                    self.seen_line_numbers.insert(entry.line_number);
                    self.new_line_numbers.insert(entry.line_number);
                }
            }
        }

        self.entries = new_entries;
        self.file_size = current_size;
        self.apply_filter();

        // Auswahl wiederherstellen
        if let Some(line_num) = selected_line_number {
            if let Some(pos) = self
                .filtered_indices
                .iter()
                .position(|&i| self.entries[i].line_number == line_num)
            {
                self.list_state.select(Some(pos));
            }
            // Falls die alte Zeile nicht mehr existiert (z.B. Rotation),
            // behält apply_filter eine sinnvolle Position bei.
        }

        // Solange wir im Tail-Modus sind, scrollen wir bei neuen Einträgen
        // automatisch weiter. Der Offset wächst um die Anzahl neuer Zeilen,
        // aber höchstens bis die markierte Zeile ganz oben im Fenster steht.
        let new_filtered_len = self.filtered_indices.len();
        if was_tailing && new_filtered_len > prev_filtered_len {
            let added = new_filtered_len - prev_filtered_len;
            if let Some(sel) = self.list_state.selected() {
                let current_offset = self.list_state.offset();
                let target_offset = (current_offset + added).min(sel);
                *self.list_state.offset_mut() = target_offset;
            }
        }

        Ok(true)
    }
}

/// Subtrahiert N Monate von einem Datum (bleibt im gültigen Bereich)
fn subtract_months(date: NaiveDate, months: u32) -> NaiveDate {
    let total_months = date.year() as i32 * 12 + date.month() as i32 - 1 - months as i32;
    let year = total_months / 12;
    let month = (total_months % 12 + 1) as u32;
    let day = date.day().min(days_in_month(year, month));
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn days_in_month(year: i32, month: u32) -> u32 {
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .unwrap()
    .pred_opt()
    .unwrap()
    .day()
}

/// Farbe für Log-Level
fn level_color(level: LogLevel) -> Color {
    match level {
        LogLevel::Emergency | LogLevel::Alert | LogLevel::Critical => Color::Magenta,
        LogLevel::Error => Color::Red,
        LogLevel::Warning => Color::Yellow,
        LogLevel::Notice | LogLevel::Info => Color::Green,
        LogLevel::Debug => Color::DarkGray,
    }
}

/// Teilt eine Nachricht am " - {" oder " - [" Trenner in (Haupttext, Option<Rest>)
fn split_message_at_json(message: &str) -> (&str, Option<&str>) {
    for sep in [" - {", " - ["] {
        if let Some(pos) = message.find(sep) {
            return (&message[..pos], Some(&message[pos..]));
        }
    }
    (message, None)
}

/// Rendert die Listen-Ansicht
fn render_list(f: &mut Frame, app: &mut App, area: Rect) {
    // Sichtbare Zeilen: Listenhöhe abzüglich der beiden Rahmenzeilen
    app.visible_rows = (area.height as usize).saturating_sub(2).max(1);

    let level_col_width = app
        .filtered_indices
        .iter()
        .map(|&idx| app.entries[idx].level.as_str().len() + 2)
        .max()
        .unwrap_or(7);

    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .map(|&idx| {
            let entry = &app.entries[idx];
            let level_style = Style::default().fg(level_color(entry.level));

            // Berechne verfügbare Breite für Nachricht
            // 2 Trennzeichen " │ " = je 3 Zeichen
            let timestamp = entry.short_timestamp();
            let level_str = format!("[{}]", entry.level);
            let level_padded = format!("{:<width$}", level_str, width = level_col_width);
            let prefix_len = timestamp.len() + level_col_width + 6; // +6 für zwei " │ "
            let msg_width = (area.width as usize).saturating_sub(prefix_len + 4);

            let (main_text, dim_rest) = split_message_at_json(&entry.message);

            let main_truncated = if main_text.len() <= msg_width {
                main_text.to_string()
            } else {
                format!("{}...", &main_text[..msg_width.saturating_sub(3)])
            };

            let sep = Style::default().fg(Color::DarkGray);
            let mut spans = vec![
                Span::styled(timestamp, Style::default().fg(Color::Cyan)),
                Span::styled(" │ ", sep),
                Span::styled(level_padded, level_style.add_modifier(Modifier::BOLD)),
                Span::styled(" │ ", sep),
                Span::raw(main_truncated.clone()),
            ];

            if let Some(rest) = dim_rest {
                let remaining = msg_width.saturating_sub(main_truncated.len());
                if remaining > 4 {
                    let rest_truncated = if rest.len() <= remaining {
                        rest.to_string()
                    } else {
                        format!("{}...", &rest[..remaining.saturating_sub(3)])
                    };
                    spans.push(Span::styled(
                        rest_truncated,
                        Style::default().fg(Color::DarkGray),
                    ));
                }
            }

            let mut item = ListItem::new(Line::from(spans));
            if app.new_line_numbers.contains(&entry.line_number) {
                item = item.style(Style::default().bg(Color::Rgb(20, 90, 40)));
            }
            item
        })
        .collect();

    // Status-Text für Titel
    let filter_info = if let Some(req_id) = &app.filter.request_id {
        format!(
            " [Request-Fokus: {} — {} Einträge]",
            req_id,
            app.filtered_indices.len()
        )
    } else if let Some(prefix) = &app.filter.message_prefix {
        let short = if prefix.len() > 40 { format!("{}…", &prefix[..40]) } else { prefix.clone() };
        format!(
            " [Gleiche: \"{}\" — {} Einträge]",
            short,
            app.filtered_indices.len()
        )
    } else if app.filter.is_active() {
        let mut parts = Vec::new();
        if let Some(label) = app.filter.date_label() {
            parts.push(label);
        }
        if app.filter.min_level.is_some() || app.filter.search_text.is_some() {
            parts.push("weiterer Filter".to_string());
        }
        let desc = if parts.is_empty() {
            String::new()
        } else {
            format!(": {}", parts.join(", "))
        };
        format!(
            " [Filter{} — {} von {} Einträgen]",
            desc,
            app.filtered_indices.len(),
            app.entries.len()
        )
    } else {
        String::new()
    };

    let title = format!(
        " {} ({}, {} Einträge){} ",
        app.file_path.file_name().unwrap_or_default().to_string_lossy(),
        format_file_size(app.file_size),
        app.entries.len(),
        filter_info
    );

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::White)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(50, 70, 110))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.list_state);

    // Scrollbar
    if !app.filtered_indices.is_empty() {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        let mut scrollbar_state = ScrollbarState::new(app.filtered_indices.len())
            .position(app.list_state.selected().unwrap_or(0));

        f.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

/// Rendert einen JSON-Wert als formatierte Lines.
/// String-Werte mit \n werden aufgefächert. "exception"-Keys werden
/// standardmäßig eingeklappt und nur bei show_exception aufgeklappt.
fn render_json_value(
    value: &serde_json::Value,
    indent: usize,
    color: Color,
    show_exception: bool,
    lines: &mut Vec<Line<'static>>,
) {
    let pad = "  ".repeat(indent);
    match value {
        serde_json::Value::Object(map) => {
            lines.push(Line::from(Span::styled(
                format!("{}{{", pad),
                Style::default().fg(color),
            )));
            let len = map.len();
            for (i, (key, val)) in map.iter().enumerate() {
                let comma = if i + 1 < len { "," } else { "" };
                // "exception"-Key: eingeklappt anzeigen wenn nicht explizit aufgeklappt
                if key == "exception" && !show_exception {
                    let hint = Style::default().fg(Color::DarkGray);
                    lines.push(Line::from(Span::styled(
                        format!("{}  \"exception\": [ausgeblendet — e zum Einblenden]{}", pad, comma),
                        hint,
                    )));
                    continue;
                }
                match val {
                    serde_json::Value::String(s) if s.contains('\n') => {
                        lines.push(Line::from(Span::styled(
                            format!("{}  \"{}\": ▼", pad, key),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        )));
                        for text_line in s.lines() {
                            lines.push(Line::from(Span::styled(
                                format!("{}    {}", pad, text_line),
                                Style::default().fg(color),
                            )));
                        }
                        if !comma.is_empty() {
                            lines.push(Line::from(Span::styled(
                                format!("{}  {}", pad, comma),
                                Style::default().fg(color),
                            )));
                        }
                    }
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        lines.push(Line::from(Span::styled(
                            format!("{}  \"{}\":", pad, key),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        )));
                        render_json_value(val, indent + 1, color, show_exception, lines);
                        if !comma.is_empty() {
                            if let Some(last) = lines.last_mut() {
                                let s = last.spans.iter().map(|s| s.content.as_ref()).collect::<String>();
                                *last = Line::from(Span::styled(
                                    format!("{}{}", s, comma),
                                    Style::default().fg(color),
                                ));
                            }
                        }
                    }
                    _ => {
                        let val_str = match val {
                            serde_json::Value::String(s) => format!("\"{}\"", s),
                            other => other.to_string(),
                        };
                        lines.push(Line::from(Span::styled(
                            format!("{}  \"{}\": {}{}", pad, key, val_str, comma),
                            Style::default().fg(color),
                        )));
                    }
                }
            }
            lines.push(Line::from(Span::styled(
                format!("{}}}", pad),
                Style::default().fg(color),
            )));
        }
        serde_json::Value::Array(arr) => {
            lines.push(Line::from(Span::styled(
                format!("{}[", pad),
                Style::default().fg(color),
            )));
            for val in arr {
                render_json_value(val, indent + 1, color, show_exception, lines);
            }
            lines.push(Line::from(Span::styled(
                format!("{}]", pad),
                Style::default().fg(color),
            )));
        }
        serde_json::Value::String(s) if s.contains('\n') => {
            for text_line in s.lines() {
                lines.push(Line::from(Span::styled(
                    format!("{}  {}", pad, text_line),
                    Style::default().fg(color),
                )));
            }
        }
        other => {
            lines.push(Line::from(Span::styled(
                format!("{}  {}", pad, other),
                Style::default().fg(color),
            )));
        }
    }
}

/// Rendert die Detail-Ansicht
fn render_detail(f: &mut Frame, app: &App, area: Rect) {
    let entry = match app.selected_entry() {
        Some(e) => e,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Kein Eintrag ausgewählt ");
            f.render_widget(block, area);
            return;
        }
    };

    let level_style = Style::default()
        .fg(level_color(entry.level))
        .add_modifier(Modifier::BOLD);

    // Werte vorab extrahieren um Borrow-Probleme zu vermeiden
    let timestamp = entry.full_timestamp();
    let level_str = entry.level.as_str();
    let request_id = entry.request_id.clone().unwrap_or_else(|| "-".to_string());
    let component = entry.component.clone();
    let line_number = entry.line_number.to_string();
    let message = entry.message.clone();
    let extra_data = entry.formatted_extra_data();

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Zeitpunkt:  ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(timestamp),
        ]),
        Line::from(vec![
            Span::styled("Level:      ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(level_str, level_style),
        ]),
        Line::from(vec![
            Span::styled("Request:    ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(request_id),
        ]),
        Line::from(vec![
            Span::styled("Component:  ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(component),
        ]),
        Line::from(vec![
            Span::styled("Zeile:      ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(line_number),
        ]),
        Line::from(""),
    ];

    // Prüfe ob JSON in der Nachricht enthalten ist
    if let Some((text_part, json_formatted)) = extract_json_from_message(&message) {
        // Text vor dem JSON anzeigen
        if !text_part.is_empty() {
            for text_line in text_part.lines() {
                lines.push(Line::from(text_line.to_string()));
            }
            lines.push(Line::from(""));
        }

        // JSON formatiert anzeigen
        lines.push(Line::from(Span::styled(
            "JSON:",
            Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan),
        )));
        lines.push(Line::from(""));

        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&json_formatted) {
            render_json_value(&json_val, 1, Color::Cyan, app.show_exception, &mut lines);
        } else {
            for json_line in json_formatted.lines() {
                lines.push(Line::from(Span::styled(
                    format!("  {}", json_line),
                    Style::default().fg(Color::Cyan),
                )));
            }
        }
    } else {
        // Normale Nachricht ohne JSON
        for msg_line in message.lines() {
            lines.push(Line::from(msg_line.to_string()));
        }
    }

    // Extra-Daten (mehrzeilige Zusatzdaten aus dem Log)
    if let Some(formatted) = extra_data {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Zusätzliche Daten:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        // Prüfe ob extra_data JSON ist
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&formatted) {
            render_json_value(&json, 1, Color::Green, app.show_exception, &mut lines);
        } else {
            for data_line in formatted.lines() {
                lines.push(Line::from(Span::styled(
                    data_line.to_string(),
                    Style::default().fg(Color::Green),
                )));
            }
        }
    }

    let text = Text::from(lines);

    let detail = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Log Details [←→: Eintrag wechseln | ESC: Zurück] ")
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll, 0));

    f.render_widget(detail, area);
}

/// Rendert die Hilfe
fn render_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(Span::styled(
            "TYPO3 Log Viewer - Tastenkürzel",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("Navigation:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  ↑/k        Nach oben"),
        Line::from("  ↓/j        Nach unten"),
        Line::from("  PgUp       Seite hoch"),
        Line::from("  PgDown     Seite runter"),
        Line::from("  Home/g     Zum Anfang"),
        Line::from("  End/G      Zum Ende"),
        Line::from("  Enter      Details anzeigen"),
        Line::from(""),
        Line::from(Span::styled("Filter:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  f          Request-Fokus (alle Einträge dieser Request-ID)"),
        Line::from("  s          Selbe Lognachricht anzeigen"),
        Line::from("  d          Datumsfilter-Menü"),
        Line::from("  /          Textsuche"),
        Line::from("  1-4        Level-Filter (1=Error, 2=Warning, 3=Info, 4=Debug)"),
        Line::from("  0/ESC      Filter zurücksetzen"),
        Line::from(""),
        Line::from(Span::styled("Allgemein:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  ?          Diese Hilfe"),
        Line::from("  q/ESC      Beenden / Zurück"),
        Line::from(""),
        Line::from(Span::styled(
            "Drücke eine beliebige Taste zum Schließen...",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Hilfe ")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    // Zentriertes Popup
    let popup_area = centered_rect(60, 70, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(help, popup_area);
}

/// Rendert das Datumsfilter-Menü
fn render_date_menu(f: &mut Frame, app: &App, area: Rect) {
    let today = Local::now().date_naive();
    let first_this = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let last_month_end = first_this.pred_opt().unwrap();

    let current = app.filter.date_label()
        .map(|l| format!("Aktuell: {}", l))
        .unwrap_or_else(|| "Kein Datumsfilter aktiv".to_string());

    let lines = vec![
        Line::from(Span::styled(" Datumsfilter", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled(format!(" {}", current), Style::default().fg(Color::Cyan))),
        Line::from(""),
        Line::from(Span::styled(" Schnellfilter:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(format!("  [1]  Heute  ({})", today.format("%d.%m.%Y"))),
        Line::from(format!("  [2]  Letzter Monat  ({})", last_month_end.format("%m/%Y"))),
        Line::from(format!("  [3]  Letzte 6 Monate  (ab {})", subtract_months(today, 6).format("%d.%m.%Y"))),
        Line::from(format!("  [4]  Letzte 12 Monate  (ab {})", subtract_months(today, 12).format("%d.%m.%Y"))),
        Line::from(""),
        Line::from(Span::styled(" Eigener Bereich:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  [5]  Datumsbereich eingeben  (TT.MM.JJJJ)"),
        Line::from(""),
        Line::from("  [0]  Datumsfilter zurücksetzen"),
        Line::from(""),
        Line::from(Span::styled(
            "  ESC  Schließen",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let popup = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Datum ")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    let popup_area = centered_rect(50, 60, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(popup, popup_area);
}

/// Rendert die Filter-Eingabe
fn render_filter_input(f: &mut Frame, app: &App, area: Rect) {
    let title = match app.filter_mode {
        FilterMode::Search => " Suche: ",
        FilterMode::Level => " Level-Filter (1-4, 0=alle): ",
        FilterMode::Component => " Component-Filter: ",
        FilterMode::DateFrom => " Von-Datum (TT.MM.JJJJ): ",
        FilterMode::DateTo => " Bis-Datum (TT.MM.JJJJ, leer = nur ein Tag): ",
        FilterMode::None => return,
    };

    let input = Paragraph::new(app.filter_input.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().fg(Color::White));

    let input_area = Rect {
        x: area.x,
        y: area.y + area.height - 3,
        width: area.width,
        height: 3,
    };

    f.render_widget(input, input_area);
}

/// Rendert die Statusleiste
fn render_statusbar(f: &mut Frame, app: &App, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");

    let left = if app.filter_mode != FilterMode::None {
        "Enter: Bestätigen | ESC: Abbrechen".to_string()
    } else if app.view == AppView::Detail {
        let pos = app.list_state.selected().map(|s| s + 1).unwrap_or(0);
        let total = app.filtered_indices.len();
        format!(
            " {}/{} | ↑↓:Scrollen | ←→:Eintrag wechseln | e:Exception ein/aus | ESC/Enter:Zurück | q:Quit",
            pos, total
        )
    } else {
        let pos = app.list_state.selected().map(|s| s + 1).unwrap_or(0);
        let total = app.filtered_indices.len();
        format!(
            " {}/{} | ↑↓:Nav | Enter:Details | f:Fokus | s:Selbe | d:Datum | /:Suche | 1-4:Level | 0:Reset | ?:Hilfe | q:Quit",
            pos, total
        )
    };

    let right = format!(" v{} ", version);
    let width = area.width as usize;
    let pad = width.saturating_sub(left.len() + right.len());
    let status = format!("{}{}{}", left, " ".repeat(pad), right);

    let statusbar = Paragraph::new(status)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));

    f.render_widget(statusbar, area);
}

/// Berechnet einen zentrierten Bereich
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Rendert die gesamte UI
pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    let main_area = chunks[0];
    let status_area = chunks[1];

    match app.view {
        AppView::List => {
            render_list(f, app, main_area);
        }
        AppView::Detail => {
            render_detail(f, app, main_area);
        }
        AppView::Help => {
            render_list(f, app, main_area);
            render_help(f, main_area);
        }
        AppView::Filter => {
            render_list(f, app, main_area);
        }
        AppView::DateMenu => {
            render_list(f, app, main_area);
            render_date_menu(f, app, main_area);
        }
    }

    // Filter-Eingabe (überlagert Statusbar)
    if app.filter_mode != FilterMode::None {
        render_filter_input(f, app, f.area());
    } else {
        render_statusbar(f, app, status_area);
    }
}

/// Verarbeitet Tastatureingaben
pub fn handle_input(app: &mut App) -> io::Result<()> {
    if event::poll(std::time::Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            // Filter-Modus
            if app.filter_mode != FilterMode::None {
                match key.code {
                    KeyCode::Esc => {
                        app.filter_mode = FilterMode::None;
                        app.filter_input.clear();
                        app.date_from_input.clear();
                    }
                    KeyCode::Enter => {
                        match app.filter_mode {
                            FilterMode::Search => {
                                let search = if app.filter_input.is_empty() {
                                    None
                                } else {
                                    Some(app.filter_input.clone())
                                };
                                app.set_search_filter(search);
                                app.filter_mode = FilterMode::None;
                                app.filter_input.clear();
                            }
                            FilterMode::Level => {
                                let level = match app.filter_input.as_str() {
                                    "1" => Some(LogLevel::Error),
                                    "2" => Some(LogLevel::Warning),
                                    "3" => Some(LogLevel::Info),
                                    "4" => Some(LogLevel::Debug),
                                    _ => None,
                                };
                                app.set_level_filter(level);
                                app.filter_mode = FilterMode::None;
                                app.filter_input.clear();
                            }
                            FilterMode::DateFrom => {
                                // Von-Datum gespeichert, jetzt Bis-Datum abfragen
                                app.date_from_input = app.filter_input.clone();
                                app.filter_input.clear();
                                app.filter_mode = FilterMode::DateTo;
                            }
                            FilterMode::DateTo => {
                                let from = parse_date_input(&app.date_from_input);
                                let to = if app.filter_input.is_empty() {
                                    from // nur ein Tag
                                } else {
                                    parse_date_input(&app.filter_input)
                                };
                                app.set_date_range(from, to);
                                app.filter_mode = FilterMode::None;
                                app.filter_input.clear();
                                app.date_from_input.clear();
                            }
                            _ => {
                                app.filter_mode = FilterMode::None;
                                app.filter_input.clear();
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        app.filter_input.pop();
                    }
                    KeyCode::Char(c) => {
                        app.filter_input.push(c);
                    }
                    _ => {}
                }
                return Ok(());
            }

            // Hilfe-Ansicht
            if app.view == AppView::Help {
                app.view = AppView::List;
                return Ok(());
            }

            // Datumsmenü
            if app.view == AppView::DateMenu {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('d') => {
                        app.view = AppView::List;
                    }
                    KeyCode::Char('1') => {
                        app.filter_today();
                        app.view = AppView::List;
                    }
                    KeyCode::Char('2') => {
                        app.filter_last_month();
                        app.view = AppView::List;
                    }
                    KeyCode::Char('3') => {
                        app.filter_last_months(6);
                        app.view = AppView::List;
                    }
                    KeyCode::Char('4') => {
                        app.filter_last_months(12);
                        app.view = AppView::List;
                    }
                    KeyCode::Char('5') => {
                        app.view = AppView::List;
                        app.filter_mode = FilterMode::DateFrom;
                        app.filter_input.clear();
                    }
                    KeyCode::Char('0') => {
                        app.filter.date_from = None;
                        app.filter.date_to = None;
                        app.apply_filter();
                        app.view = AppView::List;
                    }
                    _ => {}
                }
                return Ok(());
            }

            // Detail-Ansicht
            if app.view == AppView::Detail {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                        app.view = AppView::List;
                        app.detail_scroll = 0;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.detail_scroll = app.detail_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.detail_scroll = app.detail_scroll.saturating_add(1);
                    }
                    KeyCode::PageUp => {
                        app.detail_scroll = app.detail_scroll.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        app.detail_scroll = app.detail_scroll.saturating_add(10);
                    }
                    KeyCode::Char('e') => {
                        app.show_exception = !app.show_exception;
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        app.move_up();
                        app.detail_scroll = 0;
                        app.show_exception = false;
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        app.move_down();
                        app.detail_scroll = 0;
                        app.show_exception = false;
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        app.go_to_start();
                        app.detail_scroll = 0;
                        app.show_exception = false;
                    }
                    KeyCode::End => {
                        app.go_to_end();
                        app.detail_scroll = 0;
                        app.show_exception = false;
                    }
                    KeyCode::Char('G') => {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            app.go_to_end();
                            app.detail_scroll = 0;
                        }
                    }
                    _ => {}
                }
                return Ok(());
            }

            // Listen-Ansicht
            match key.code {
                KeyCode::Char('q') => {
                    app.should_quit = true;
                }
                KeyCode::Esc => {
                    if app.filter.is_active() {
                        app.clear_filter();
                    } else if app.has_file_selector {
                        app.should_go_back = true;
                    } else {
                        app.should_quit = true;
                    }
                }
                KeyCode::Char('f') => {
                    app.set_request_focus();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    app.move_up();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.move_down();
                }
                KeyCode::PageUp => {
                    app.page_up(app.visible_rows);
                }
                KeyCode::PageDown => {
                    app.page_down(app.visible_rows);
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    app.go_to_start();
                }
                KeyCode::Char('s') => {
                    app.set_message_focus();
                }
                KeyCode::End => {
                    app.go_to_end();
                }
                KeyCode::Char('G') => {
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        app.go_to_end();
                    }
                }
                KeyCode::Enter => {
                    if app.selected_entry().is_some() {
                        app.view = AppView::Detail;
                    }
                }
                KeyCode::Char('/') => {
                    app.filter_mode = FilterMode::Search;
                }
                KeyCode::Char('1') => {
                    app.set_level_filter(Some(LogLevel::Error));
                }
                KeyCode::Char('2') => {
                    app.set_level_filter(Some(LogLevel::Warning));
                }
                KeyCode::Char('3') => {
                    app.set_level_filter(Some(LogLevel::Info));
                }
                KeyCode::Char('4') => {
                    app.set_level_filter(Some(LogLevel::Debug));
                }
                KeyCode::Char('0') => {
                    app.clear_filter();
                }
                KeyCode::Char('d') => {
                    app.view = AppView::DateMenu;
                }
                KeyCode::Char('?') => {
                    app.view = AppView::Help;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

pub enum AppExit {
    Quit,
    Back,
}

/// Startet die App
pub fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    mut app: App,
) -> io::Result<AppExit> {
    let mut last_reload_check = std::time::Instant::now();
    let reload_interval = std::time::Duration::from_millis(500);

    loop {
        terminal.draw(|f| render(f, &mut app))?;
        handle_input(&mut app)?;

        // Periodisch prüfen, ob die Datei sich geändert hat (Live-Tail)
        if last_reload_check.elapsed() >= reload_interval {
            let _ = app.reload_if_changed();
            last_reload_check = std::time::Instant::now();
        }

        if app.should_go_back {
            return Ok(AppExit::Back);
        }
        if app.should_quit {
            return Ok(AppExit::Quit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;
    use std::fs;
    use std::path::Path;

    fn make_entry(line_number: usize) -> LogEntry {
        LogEntry {
            timestamp: DateTime::parse_from_rfc3339("2026-04-02T12:00:00+02:00").unwrap(),
            level: LogLevel::Info,
            request_id: Some(format!("req{}", line_number)),
            component: "Test".to_string(),
            message: format!("Message {}", line_number),
            extra_data: None,
            line_number,
        }
    }

    fn make_app(n: usize) -> App {
        let entries: Vec<LogEntry> = (1..=n).map(make_entry).collect();
        App::new(LoadResult {
            entries,
            file_path: PathBuf::from("/dev/null"),
            file_size: 0,
        })
    }

    fn unique_temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "typo3logvtest_{}_{}_{}.log",
            std::process::id(),
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        path
    }

    fn write_log_lines(path: &Path, count: usize) {
        let mut content = String::new();
        for i in 1..=count {
            content.push_str(&format!(
                "Thu, 02 Apr 2026 12:{:02}:00 +0200 [INFO] request=\"r{}\" component=\"Test\": Entry {}\n",
                i % 60, i, i
            ));
        }
        fs::write(path, &content).unwrap();
    }

    fn load_app_from(path: &Path) -> App {
        let result = crate::loader::load_log_file(path).unwrap();
        App::new(result)
    }

    // -- App::new ---------------------------------------------------------

    #[test]
    fn new_selects_last_entry_and_enables_auto_tail() {
        let app = make_app(5);
        assert_eq!(app.list_state.selected(), Some(4));
        assert!(app.auto_tail);
    }

    #[test]
    fn new_seeds_seen_line_numbers_so_existing_entries_are_not_marked_new() {
        let app = make_app(5);
        assert!(app.new_line_numbers.is_empty());
        assert_eq!(app.seen_line_numbers.len(), 5);
        for ln in 1..=5 {
            assert!(app.seen_line_numbers.contains(&ln));
        }
    }

    #[test]
    fn new_with_empty_entries_has_no_selection() {
        let app = App::new(LoadResult {
            entries: vec![],
            file_path: PathBuf::from("/dev/null"),
            file_size: 0,
        });
        assert_eq!(app.list_state.selected(), None);
        assert!(app.seen_line_numbers.is_empty());
    }

    // -- Navigation: auto_tail-Flag ---------------------------------------

    #[test]
    fn move_up_disables_auto_tail() {
        let mut app = make_app(5);
        app.move_up();
        assert_eq!(app.list_state.selected(), Some(3));
        assert!(!app.auto_tail);
    }

    #[test]
    fn move_up_at_top_clamps_and_stays_disabled() {
        let mut app = make_app(5);
        app.list_state.select(Some(0));
        app.move_up();
        assert_eq!(app.list_state.selected(), Some(0));
        assert!(!app.auto_tail);
    }

    #[test]
    fn move_up_on_empty_list_is_noop() {
        let mut app = make_app(0);
        app.move_up();
        assert_eq!(app.list_state.selected(), None);
    }

    #[test]
    fn move_down_to_last_enables_auto_tail() {
        let mut app = make_app(5);
        app.list_state.select(Some(3));
        app.auto_tail = false;
        app.move_down();
        assert_eq!(app.list_state.selected(), Some(4));
        assert!(app.auto_tail);
    }

    #[test]
    fn move_down_in_middle_disables_auto_tail() {
        let mut app = make_app(5);
        app.list_state.select(Some(1));
        app.auto_tail = true;
        app.move_down();
        assert_eq!(app.list_state.selected(), Some(2));
        assert!(!app.auto_tail);
    }

    #[test]
    fn move_down_at_last_keeps_auto_tail() {
        let mut app = make_app(5);
        app.list_state.select(Some(4));
        app.auto_tail = true;
        app.move_down();
        assert_eq!(app.list_state.selected(), Some(4));
        assert!(app.auto_tail);
    }

    #[test]
    fn page_up_disables_auto_tail() {
        let mut app = make_app(20);
        app.page_up(5);
        assert_eq!(app.list_state.selected(), Some(14));
        assert!(!app.auto_tail);
    }

    #[test]
    fn page_up_clamps_at_zero() {
        let mut app = make_app(20);
        app.list_state.select(Some(3));
        app.page_up(10);
        assert_eq!(app.list_state.selected(), Some(0));
        assert!(!app.auto_tail);
    }

    #[test]
    fn page_down_to_last_enables_auto_tail() {
        let mut app = make_app(20);
        app.list_state.select(Some(15));
        app.auto_tail = false;
        app.page_down(10);
        assert_eq!(app.list_state.selected(), Some(19));
        assert!(app.auto_tail);
    }

    #[test]
    fn page_down_in_middle_disables_auto_tail() {
        let mut app = make_app(20);
        app.list_state.select(Some(0));
        app.auto_tail = true;
        app.page_down(5);
        assert_eq!(app.list_state.selected(), Some(5));
        assert!(!app.auto_tail);
    }

    #[test]
    fn go_to_start_disables_auto_tail() {
        let mut app = make_app(5);
        assert!(app.auto_tail);
        app.go_to_start();
        assert_eq!(app.list_state.selected(), Some(0));
        assert!(!app.auto_tail);
    }

    #[test]
    fn go_to_end_enables_auto_tail() {
        let mut app = make_app(5);
        app.go_to_start();
        assert!(!app.auto_tail);
        app.go_to_end();
        assert_eq!(app.list_state.selected(), Some(4));
        assert!(app.auto_tail);
    }

    #[test]
    fn go_to_start_on_empty_list_is_noop() {
        let mut app = make_app(0);
        app.go_to_start();
        assert_eq!(app.list_state.selected(), None);
    }

    #[test]
    fn go_to_end_on_empty_list_is_noop() {
        let mut app = make_app(0);
        app.go_to_end();
        assert_eq!(app.list_state.selected(), None);
    }

    #[test]
    fn set_message_focus_disables_auto_tail() {
        let mut app = make_app(3);
        app.auto_tail = true;
        app.set_message_focus();
        assert!(!app.auto_tail);
        assert_eq!(app.list_state.selected(), Some(0));
    }

    #[test]
    fn set_request_focus_disables_auto_tail() {
        let mut app = make_app(3);
        app.auto_tail = true;
        app.set_request_focus();
        assert!(!app.auto_tail);
        assert_eq!(app.list_state.selected(), Some(0));
    }

    // -- reload_if_changed ------------------------------------------------

    #[test]
    fn reload_returns_false_when_size_unchanged() {
        let path = unique_temp_path("nochange");
        write_log_lines(&path, 3);
        let mut app = load_app_from(&path);
        let changed = app.reload_if_changed().unwrap();
        assert!(!changed);
        assert!(app.new_line_numbers.is_empty());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn reload_marks_appended_entries_as_new() {
        let path = unique_temp_path("appended");
        write_log_lines(&path, 4);
        let mut app = load_app_from(&path);
        assert!(app.new_line_numbers.is_empty());

        write_log_lines(&path, 7);
        let changed = app.reload_if_changed().unwrap();

        assert!(changed);
        assert_eq!(app.entries.len(), 7);
        assert_eq!(app.new_line_numbers.len(), 3);
        for ln in 5..=7 {
            assert!(app.new_line_numbers.contains(&ln), "line {} should be new", ln);
        }
        for ln in 1..=4 {
            assert!(!app.new_line_numbers.contains(&ln), "line {} should not be new", ln);
        }
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn reload_accumulates_new_entries_across_multiple_reloads() {
        let path = unique_temp_path("accumulate");
        write_log_lines(&path, 2);
        let mut app = load_app_from(&path);

        write_log_lines(&path, 4);
        app.reload_if_changed().unwrap();
        write_log_lines(&path, 6);
        app.reload_if_changed().unwrap();

        // line_numbers 3..=6 should all be marked as new
        assert_eq!(app.new_line_numbers.len(), 4);
        for ln in 3..=6 {
            assert!(app.new_line_numbers.contains(&ln));
        }
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn reload_resets_new_marker_on_log_rotation() {
        let path = unique_temp_path("rotation");
        write_log_lines(&path, 10);
        let mut app = load_app_from(&path);
        let prev_size = app.file_size;

        // Datei schrumpft (simulierte Rotation)
        write_log_lines(&path, 2);
        let new_size = fs::metadata(&path).unwrap().len();
        assert!(new_size < prev_size);

        app.reload_if_changed().unwrap();

        assert!(app.new_line_numbers.is_empty());
        assert_eq!(app.seen_line_numbers.len(), 2);
        assert_eq!(app.entries.len(), 2);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn reload_in_tail_mode_advances_offset_by_added_count() {
        let path = unique_temp_path("scroll_tail");
        write_log_lines(&path, 5);
        let mut app = load_app_from(&path);
        app.visible_rows = 5;
        *app.list_state.offset_mut() = 0;
        assert_eq!(app.list_state.selected(), Some(4));
        assert!(app.auto_tail);

        // 2 neue Einträge → Offset wandert von 0 auf 2
        write_log_lines(&path, 7);
        app.reload_if_changed().unwrap();

        assert_eq!(app.list_state.selected(), Some(4));
        assert_eq!(app.list_state.offset(), 2);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn reload_caps_offset_at_selected_position() {
        let path = unique_temp_path("scroll_cap");
        write_log_lines(&path, 5);
        let mut app = load_app_from(&path);
        app.visible_rows = 5;
        *app.list_state.offset_mut() = 0;
        assert_eq!(app.list_state.selected(), Some(4));

        // Viele neue Einträge auf einen Schlag: Offset würde 0 + 8 = 8
        // ergeben, soll aber bei sel=4 abgeschnitten werden.
        write_log_lines(&path, 13);
        app.reload_if_changed().unwrap();

        assert_eq!(app.list_state.selected(), Some(4));
        assert_eq!(app.list_state.offset(), 4);

        // Weitere Reloads dürfen den Offset nicht weiter anheben
        write_log_lines(&path, 25);
        app.reload_if_changed().unwrap();
        assert_eq!(app.list_state.offset(), 4);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn reload_does_not_scroll_when_auto_tail_disabled() {
        let path = unique_temp_path("no_scroll");
        write_log_lines(&path, 10);
        let mut app = load_app_from(&path);
        app.visible_rows = 5;

        app.go_to_start();
        assert!(!app.auto_tail);
        *app.list_state.offset_mut() = 0;

        write_log_lines(&path, 15);
        app.reload_if_changed().unwrap();

        assert_eq!(app.list_state.selected(), Some(0));
        assert_eq!(app.list_state.offset(), 0);
        // Trotzdem werden neue Einträge als "neu" markiert
        assert_eq!(app.new_line_numbers.len(), 5);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn reload_preserves_selection_via_line_number() {
        let path = unique_temp_path("preserve_sel");
        write_log_lines(&path, 10);
        let mut app = load_app_from(&path);
        app.list_state.select(Some(3)); // line_number 4
        app.auto_tail = false;

        write_log_lines(&path, 15);
        app.reload_if_changed().unwrap();

        // Eintrag mit line_number 4 ist weiterhin an Position 3
        assert_eq!(app.list_state.selected(), Some(3));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn reload_recovers_when_file_temporarily_missing() {
        // Existiert die Datei nicht, soll reload_if_changed nicht panicen
        let path = unique_temp_path("missing");
        write_log_lines(&path, 3);
        let mut app = load_app_from(&path);
        fs::remove_file(&path).unwrap();
        let changed = app.reload_if_changed().unwrap();
        assert!(!changed);
    }
}
