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
    /// Soll die App beendet werden?
    pub should_quit: bool,
    /// Soll zur Dateiauswahl zurückgekehrt werden?
    pub should_go_back: bool,
    /// Dateiauswahl war verfügbar (Verzeichnis mit mehreren Dateien)
    pub has_file_selector: bool,
    /// Zwischenspeicher für Von-Datum bei der Bereichseingabe
    date_from_input: String,
}

impl App {
    pub fn new(result: LoadResult) -> Self {
        let filtered_indices: Vec<usize> = (0..result.entries.len()).collect();

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
            should_quit: false,
            should_go_back: false,
            has_file_selector: false,
            date_from_input: String::new(),
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
            let current = self.list_state.selected().unwrap_or(0);
            if current >= self.filtered_indices.len() {
                self.list_state.select(Some(self.filtered_indices.len() - 1));
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
        if let Some(selected) = self.list_state.selected() {
            if selected > 0 {
                self.list_state.select(Some(selected - 1));
            }
        }
    }

    /// Navigation: nach unten
    pub fn move_down(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected < self.filtered_indices.len().saturating_sub(1) {
                self.list_state.select(Some(selected + 1));
            }
        }
    }

    /// Navigation: Seite hoch
    pub fn page_up(&mut self, page_size: usize) {
        if let Some(selected) = self.list_state.selected() {
            let new_selected = selected.saturating_sub(page_size);
            self.list_state.select(Some(new_selected));
        }
    }

    /// Navigation: Seite runter
    pub fn page_down(&mut self, page_size: usize) {
        if let Some(selected) = self.list_state.selected() {
            let new_selected = (selected + page_size).min(self.filtered_indices.len().saturating_sub(1));
            self.list_state.select(Some(new_selected));
        }
    }

    /// Navigation: zum Anfang
    pub fn go_to_start(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    /// Navigation: zum Ende
    pub fn go_to_end(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(self.filtered_indices.len() - 1));
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

        // Datei neu laden und parsen
        let content = std::fs::read_to_string(&self.file_path)?;
        self.entries = crate::parser::parse_log_content(&content);
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

/// Rendert die Listen-Ansicht
fn render_list(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .map(|&idx| {
            let entry = &app.entries[idx];
            let level_style = Style::default().fg(level_color(entry.level));

            // Berechne verfügbare Breite für Nachricht
            let timestamp = entry.short_timestamp();
            let level = format!("[{}]", entry.level);
            let prefix_len = timestamp.len() + level.len() + 3; // +3 für Leerzeichen
            let msg_width = (area.width as usize).saturating_sub(prefix_len + 4);

            let line = Line::from(vec![
                Span::styled(timestamp, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(level, level_style.add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::raw(entry.truncated_message(msg_width)),
            ]);

            ListItem::new(line)
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
                .bg(Color::DarkGray)
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
        Line::from(Span::styled(
            "Nachricht:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
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

        for json_line in json_formatted.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", json_line),
                Style::default().fg(Color::Cyan),
            )));
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
            let pretty = serde_json::to_string_pretty(&json).unwrap_or(formatted.clone());
            for data_line in pretty.lines() {
                lines.push(Line::from(Span::styled(
                    format!("  {}", data_line),
                    Style::default().fg(Color::Green),
                )));
            }
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
            " {}/{} | ↑↓:Scrollen | ←→:Eintrag wechseln | ESC/Enter:Zurück | q:Quit",
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
                    KeyCode::Left | KeyCode::Char('h') => {
                        app.move_up();
                        app.detail_scroll = 0;
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        app.move_down();
                        app.detail_scroll = 0;
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        app.go_to_start();
                        app.detail_scroll = 0;
                    }
                    KeyCode::End => {
                        app.go_to_end();
                        app.detail_scroll = 0;
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
                    app.page_up(20);
                }
                KeyCode::PageDown => {
                    app.page_down(20);
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
