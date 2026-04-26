mod loader;
mod model;
mod parser;
mod ui;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use loader::{find_log_files, format_file_size, load_log_file};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io::{self, stdout};
use std::path::{Path, PathBuf};
use std::process;
use ui::{run_app, App, AppExit};

fn print_version() {
    println!("typo3-log-viewer {}", env!("CARGO_PKG_VERSION"));
}

fn print_usage() {
    eprintln!("TYPO3 Log Viewer - Interaktive Darstellung von TYPO3-Logdateien");
    eprintln!();
    eprintln!("Verwendung:");
    eprintln!("  typo3-log-viewer <datei.log>     Öffnet eine spezifische Log-Datei");
    eprintln!("  typo3-log-viewer <verzeichnis>   Listet alle .log-Dateien im Verzeichnis");
    eprintln!("  typo3-log-viewer                 Nutzt ./var/log/ falls vorhanden");
    eprintln!();
    eprintln!("Tastenkürzel:");
    eprintln!("  ↑/↓, j/k      Navigation");
    eprintln!("  PgUp/PgDown   Schnelles Scrollen");
    eprintln!("  Enter         Details anzeigen");
    eprintln!("  /             Textsuche");
    eprintln!("  1-4           Level-Filter (1=Error, 2=Warning, 3=Info, 4=Debug)");
    eprintln!("  0             Filter zurücksetzen");
    eprintln!("  ?             Hilfe");
    eprintln!("  q, ESC        Beenden");
}

/// Datei-Info für die Auswahl
struct FileInfo {
    path: PathBuf,
    name: String,
    size: String,
}

/// Interaktive Dateiauswahl mit TUI
fn select_file_interactive(files: &[PathBuf], preselect: Option<usize>) -> io::Result<Option<PathBuf>> {
    // Datei-Infos sammeln
    let file_infos: Vec<FileInfo> = files
        .iter()
        .map(|path| {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let size = std::fs::metadata(path)
                .map(|m| format_file_size(m.len()))
                .unwrap_or_else(|_| "?".to_string());
            FileInfo {
                path: path.clone(),
                name,
                size,
            }
        })
        .collect();

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut list_state = ListState::default();
    list_state.select(Some(preselect.unwrap_or(0)));

    let mut selected_file: Option<PathBuf> = None;

    loop {
        terminal.draw(|f| {
            render_file_selector(f, &file_infos, &mut list_state);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        break;
                    }
                    KeyCode::Enter => {
                        if let Some(idx) = list_state.selected() {
                            selected_file = Some(file_infos[idx].path.clone());
                        }
                        break;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if let Some(selected) = list_state.selected() {
                            if selected > 0 {
                                list_state.select(Some(selected - 1));
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if let Some(selected) = list_state.selected() {
                            if selected < file_infos.len() - 1 {
                                list_state.select(Some(selected + 1));
                            }
                        }
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        list_state.select(Some(0));
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        list_state.select(Some(file_infos.len() - 1));
                    }
                    _ => {}
                }
            }
        }
    }

    // Terminal cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(selected_file)
}

/// Rendert die Dateiauswahl
fn render_file_selector(f: &mut Frame, files: &[FileInfo], list_state: &mut ListState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    let items: Vec<ListItem> = files
        .iter()
        .map(|file| {
            let line = Line::from(vec![
                Span::styled(&file.name, Style::default().fg(Color::White)),
                Span::raw("  "),
                Span::styled(format!("({})", file.size), Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Log-Datei auswählen ({} Dateien) ", files.len()))
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(50, 70, 110))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, chunks[0], list_state);

    let left = " ↑↓/jk: Navigation | Enter: Auswählen | q/ESC: Abbrechen";
    let right = format!(" v{} ", env!("CARGO_PKG_VERSION"));
    let width = chunks[1].width as usize;
    let pad = width.saturating_sub(left.len() + right.len());
    let help_text = format!("{}{}{}", left, " ".repeat(pad), right);

    let help = Paragraph::new(help_text)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));

    f.render_widget(help, chunks[1]);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Hilfe / Version anzeigen
    if args.len() >= 2 && (args[1] == "-h" || args[1] == "--help") {
        print_usage();
        return;
    }
    if args.len() >= 2 && (args[1] == "-v" || args[1] == "--version") {
        print_version();
        return;
    }

    // Pfad aus Argumenten oder Fallback auf ./var/log/
    let path_arg: String = if args.len() >= 2 {
        args[1].clone()
    } else {
        let default = Path::new("./var/log");
        if default.is_dir() {
            eprintln!("Kein Pfad angegeben — verwende ./var/log/");
            "./var/log".to_string()
        } else {
            print_usage();
            process::exit(1);
        }
    };

    let path = Path::new(&path_arg);

    // Prüfe ob Pfad existiert
    if !path.exists() {
        eprintln!("Fehler: Pfad '{}' existiert nicht.", path_arg);
        process::exit(1);
    }

    // Dateien im Verzeichnis ermitteln (oder direkte Datei)
    let (files, has_file_selector) = if path.is_dir() {
        match find_log_files(path) {
            Ok(files) if files.is_empty() => {
                eprintln!("Keine .log-Dateien in '{}' gefunden.", path_arg);
                process::exit(1);
            }
            Ok(files) => {
                let has_selector = files.len() > 1;
                (files, has_selector)
            }
            Err(e) => {
                eprintln!("Fehler beim Lesen des Verzeichnisses: {}", e);
                process::exit(1);
            }
        }
    } else {
        (vec![path.to_path_buf()], false)
    };

    // Hauptschleife: Dateiauswahl → Viewer → ggf. zurück zur Auswahl
    let mut last_selected_index: Option<usize> = None;
    loop {
        let file_to_open = if files.len() == 1 {
            files[0].clone()
        } else {
            match select_file_interactive(&files, last_selected_index) {
                Ok(Some(f)) => {
                    last_selected_index = files.iter().position(|p| p == &f);
                    f
                }
                Ok(None) => process::exit(0),
                Err(e) => {
                    eprintln!("Fehler bei der Dateiauswahl: {}", e);
                    process::exit(1);
                }
            }
        };

        // Datei laden
        eprintln!("Lade {}...", file_to_open.display());
        let result = match load_log_file(&file_to_open) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Fehler beim Laden der Datei: {}", e);
                process::exit(1);
            }
        };

        if result.entries.is_empty() {
            eprintln!("Keine Log-Einträge in der Datei gefunden.");
            process::exit(1);
        }

        eprintln!("{} Einträge geladen.", result.entries.len());

        match run_tui(result, has_file_selector) {
            Ok(AppExit::Back) => continue,
            Ok(AppExit::Quit) => break,
            Err(e) => {
                eprintln!("Fehler: {}", e);
                process::exit(1);
            }
        }
    }
}

fn run_tui(result: loader::LoadResult, has_file_selector: bool) -> io::Result<AppExit> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App erstellen und ausführen
    let mut app = App::new(result);
    app.has_file_selector = has_file_selector;
    let res = run_app(&mut terminal, app);

    // Terminal cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}
