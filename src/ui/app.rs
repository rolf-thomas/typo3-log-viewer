use crate::loader::{format_file_size, LoadResult};
use crate::model::{LogEntry, LogFilter, LogLevel};
use crate::parser::extract_json_from_message;
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
    Filter,
    Help,
}

/// Input-Modus für Filter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    None,
    Search,
    Level,
    Component,
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
    let filter_info = if app.filter.is_active() {
        format!(
            " [Filter aktiv: {} von {} Einträgen]",
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
                .title(" Log Details [ESC: Zurück] ")
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
        Line::from("  /          Textsuche"),
        Line::from("  1-4        Level-Filter (1=Error, 2=Warning, 3=Info, 4=Debug)"),
        Line::from("  0          Filter zurücksetzen"),
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

/// Rendert die Filter-Eingabe
fn render_filter_input(f: &mut Frame, app: &App, area: Rect) {
    let title = match app.filter_mode {
        FilterMode::Search => " Suche: ",
        FilterMode::Level => " Level-Filter (1-4, 0=alle): ",
        FilterMode::Component => " Component-Filter: ",
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
    let status = if app.filter_mode != FilterMode::None {
        "Enter: Bestätigen | ESC: Abbrechen".to_string()
    } else {
        let pos = app.list_state.selected().map(|s| s + 1).unwrap_or(0);
        let total = app.filtered_indices.len();
        format!(
            " {}/{} | ↑↓:Nav | Enter:Details | /:Suche | 1-4:Level | 0:Reset | ?:Hilfe | q:Quit ",
            pos, total
        )
    };

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
                            }
                            _ => {}
                        }
                        app.filter_mode = FilterMode::None;
                        app.filter_input.clear();
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
                    } else {
                        app.should_quit = true;
                    }
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
                KeyCode::Char('?') => {
                    app.view = AppView::Help;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/// Startet die App
pub fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    mut app: App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| render(f, &mut app))?;
        handle_input(&mut app)?;

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
