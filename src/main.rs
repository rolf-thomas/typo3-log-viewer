mod clipboard;
mod loader;
mod model;
mod parser;
mod ui;
mod updater;

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
use updater::{InstallMethod, UpdateInfo, UpdateState};

fn print_version() {
    println!("{}", env!("CARGO_PKG_VERSION"));
}

fn print_usage() {
    eprintln!("TYPO3 Log Viewer v{} - Interaktive Darstellung von TYPO3-Logdateien", env!("CARGO_PKG_VERSION"));
    eprintln!();
    eprintln!("Verwendung:");
    eprintln!("  typo3-log-viewer <datei.log>     Öffnet eine spezifische Log-Datei");
    eprintln!("  typo3-log-viewer <verzeichnis>   Listet alle .log-Dateien im Verzeichnis");
    eprintln!("  typo3-log-viewer                 Nutzt ./var/log/ falls vorhanden");
    eprintln!();
    eprintln!("Navigation:");
    eprintln!("  ↑/↓, j/k        Nach oben / unten");
    eprintln!("  PgUp/PgDown     Seite hoch / runter  (macOS: Fn+↑/↓)");
    eprintln!("  Home/g, End/G   Zum Anfang / Ende    (macOS: Fn+←/→)");
    eprintln!("  Enter           Details anzeigen");
    eprintln!();
    eprintln!("Detail-Ansicht:");
    eprintln!("  ←/→, h/l        Vorheriger / nächster Eintrag");
    eprintln!("  ↑/↓, j/k        Scrollen");
    eprintln!("  e               Exception-Details ein-/ausklappen");
    eprintln!("  c               Inhalt in Zwischenablage kopieren");
    eprintln!("  ESC/Enter       Zurück zur Liste");
    eprintln!();
    eprintln!("Filter:");
    eprintln!("  /               Textsuche (Nachricht + Component)");
    eprintln!("  1–4             Level-Filter (1=Error, 2=Warning, 3=Info, 4=Debug)");
    eprintln!("  f               Request-Fokus (alle Einträge dieser Request-ID)");
    eprintln!("  s               Selbe Lognachricht anzeigen");
    eprintln!("  d               Datumsfilter-Menü (Heute, Monat, Bereich)");
    eprintln!("  0 / ESC         Filter zurücksetzen");
    eprintln!();
    eprintln!("Allgemein:");
    eprintln!("  t               Zeitkorrektur: dann + / − (je 1h) oder 0 (Reset)");
    eprintln!("  Shift+↑/↓       Zeilen markieren (zusammenhängend)");
    eprintln!("  Backspace       Löschen: Zeile / Markierung / Selektion / ganze Datei");
    eprintln!("  ?               Hilfe (im TUI)");
    eprintln!("  q / ESC         Beenden");
}

/// Datei-Info für die Auswahl
struct FileInfo {
    path: PathBuf,
    name: String,
    size: String,
    is_empty: bool,
}

/// Interaktive Dateiauswahl mit TUI
fn select_file_interactive(
    files: &[PathBuf],
    preselect: Option<usize>,
    update_state: &UpdateState,
) -> io::Result<Option<PathBuf>> {
    // Datei-Infos sammeln
    let file_infos: Vec<FileInfo> = files
        .iter()
        .map(|path| {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let metadata = std::fs::metadata(path).ok();
            let bytes = metadata.as_ref().map(|m| m.len());
            let size = bytes
                .map(format_file_size)
                .unwrap_or_else(|| "?".to_string());
            let is_empty = bytes == Some(0);
            FileInfo {
                path: path.clone(),
                name,
                size,
                is_empty,
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
            render_file_selector(f, &file_infos, &mut list_state, update_state);
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
                            if file_infos[idx].is_empty {
                                continue;
                            }
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
fn render_file_selector(
    f: &mut Frame,
    files: &[FileInfo],
    list_state: &mut ListState,
    update_state: &UpdateState,
) {
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
    let update_available = updater::current(update_state).is_some();
    let right = if update_available {
        format!(" v{} * ", env!("CARGO_PKG_VERSION"))
    } else {
        format!(" v{} ", env!("CARGO_PKG_VERSION"))
    };

    let width = chunks[1].width as usize;
    let pad = width.saturating_sub(left.len() + right.len());

    let bg = Style::default().bg(Color::DarkGray).fg(Color::White);
    let help_line = Line::from(vec![
        Span::styled(left.to_string(), bg),
        Span::styled(" ".repeat(pad), bg),
        Span::styled(
            right,
            if update_available {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                bg
            },
        ),
    ]);

    let help = Paragraph::new(help_line).style(bg);
    f.render_widget(help, chunks[1]);
}

/// Setzt einen Panic-Hook, der das Terminal vor der Panic-Ausgabe zurücksetzt,
/// damit die Shell nach einem Crash nicht im Alternate-Screen / Raw-Mode hängt.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));
}

fn main() {
    install_panic_hook();

    // Update-Check im Hintergrund starten — Ergebnis fließt in Statusleisten
    // und in den Exit-Hinweis ein.
    let (update_state, _update_handle) = updater::start_check();

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
            match select_file_interactive(&files, last_selected_index, &update_state) {
                Ok(Some(f)) => {
                    last_selected_index = files.iter().position(|p| p == &f);
                    f
                }
                Ok(None) => {
                    print_update_notice(&update_state);
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("Fehler bei der Dateiauswahl: {}", e);
                    process::exit(1);
                }
            }
        };

        // Datei laden
        let result = match load_log_file(&file_to_open) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Fehler beim Laden der Datei: {}", e);
                process::exit(1);
            }
        };

        match run_tui(result, has_file_selector, update_state.clone()) {
            Ok(AppExit::Back) => continue,
            Ok(AppExit::Quit) => break,
            Err(e) => {
                eprintln!("Fehler: {}", e);
                process::exit(1);
            }
        }
    }

    print_update_notice(&update_state);
}

/// Schreibt nach Beenden einen Hinweis in die Shell, wenn eine neuere Version
/// verfügbar ist. Format und Inhalt hängen von der erkannten Installationsart ab.
fn print_update_notice(state: &UpdateState) {
    let Some(info) = updater::current(state) else {
        return;
    };
    let UpdateInfo {
        latest_version,
        install_method,
    } = info;
    let current = env!("CARGO_PKG_VERSION");

    // ANSI: gelb für Header, fett für Versionssprung.
    let yellow = "\x1b[33m";
    let bold = "\x1b[1m";
    let dim = "\x1b[2m";
    let reset = "\x1b[0m";

    eprintln!();
    eprintln!(
        "{yellow}{bold}★ Update verfügbar:{reset} typo3-log-viewer {dim}v{current}{reset} → {bold}v{latest_version}{reset}"
    );
    match install_method {
        InstallMethod::Homebrew => {
            eprintln!("  Update via Homebrew:");
            eprintln!("    {}", InstallMethod::Homebrew.update_command());
        }
        InstallMethod::Manual => {
            eprintln!("  {}", InstallMethod::Manual.update_command());
            eprintln!(
                "  {dim}Auf macOS ggf. anschließend: xattr -d com.apple.quarantine typo3-log-viewer{reset}"
            );
        }
    }
    eprintln!();
}

fn run_tui(
    result: loader::LoadResult,
    has_file_selector: bool,
    update_state: UpdateState,
) -> io::Result<AppExit> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App erstellen und ausführen
    let mut app = App::new(result);
    app.has_file_selector = has_file_selector;
    app.update_state = Some(update_state);
    let res = run_app(&mut terminal, app);

    // Terminal cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}
