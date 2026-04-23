mod loader;
mod model;
mod parser;
mod ui;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use loader::{find_log_files, load_log_file};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, stdout};
use std::path::Path;
use std::process;
use ui::{run_app, App};

fn print_usage() {
    eprintln!("TYPO3 Log Viewer - Interaktive Darstellung von TYPO3-Logdateien");
    eprintln!();
    eprintln!("Verwendung:");
    eprintln!("  typo3-log-viewer <datei.log>     Öffnet eine spezifische Log-Datei");
    eprintln!("  typo3-log-viewer <verzeichnis>   Listet alle .log-Dateien im Verzeichnis");
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

fn select_file_interactive(files: &[std::path::PathBuf]) -> Option<std::path::PathBuf> {
    eprintln!("Gefundene Log-Dateien:");
    eprintln!();

    for (i, file) in files.iter().enumerate() {
        let name = file.file_name().unwrap_or_default().to_string_lossy();
        let size = std::fs::metadata(file)
            .map(|m| loader::format_file_size(m.len()))
            .unwrap_or_else(|_| "?".to_string());
        eprintln!("  [{}] {} ({})", i + 1, name, size);
    }

    eprintln!();
    eprint!("Datei auswählen (1-{}): ", files.len());

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        if let Ok(num) = input.trim().parse::<usize>() {
            if num >= 1 && num <= files.len() {
                return Some(files[num - 1].clone());
            }
        }
    }

    None
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let path_arg = &args[1];

    // Hilfe anzeigen
    if path_arg == "-h" || path_arg == "--help" {
        print_usage();
        return;
    }

    let path = Path::new(path_arg);

    // Prüfe ob Pfad existiert
    if !path.exists() {
        eprintln!("Fehler: Pfad '{}' existiert nicht.", path_arg);
        process::exit(1);
    }

    // Datei zum Öffnen bestimmen
    let file_to_open = if path.is_dir() {
        match find_log_files(path) {
            Ok(files) if files.is_empty() => {
                eprintln!("Keine .log-Dateien in '{}' gefunden.", path_arg);
                process::exit(1);
            }
            Ok(files) if files.len() == 1 => files[0].clone(),
            Ok(files) => match select_file_interactive(&files) {
                Some(f) => f,
                None => {
                    eprintln!("Keine gültige Auswahl.");
                    process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("Fehler beim Lesen des Verzeichnisses: {}", e);
                process::exit(1);
            }
        }
    } else {
        path.to_path_buf()
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

    // Terminal UI starten
    if let Err(e) = run_tui(result) {
        eprintln!("Fehler: {}", e);
        process::exit(1);
    }
}

fn run_tui(result: loader::LoadResult) -> io::Result<()> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App erstellen und ausführen
    let app = App::new(result);
    let res = run_app(&mut terminal, app);

    // Terminal cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}
