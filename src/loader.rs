use crate::model::LogEntry;
use crate::parser::parse_log_content;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Ergebnis des Ladevorgangs
pub struct LoadResult {
    pub entries: Vec<LogEntry>,
    pub file_path: PathBuf,
    pub file_size: u64,
}

/// Lädt eine Log-Datei und parst deren Inhalt
pub fn load_log_file(path: &Path) -> io::Result<LoadResult> {
    let metadata = fs::metadata(path)?;
    let file_size = metadata.len();

    let content = fs::read_to_string(path)?;
    let entries = parse_log_content(&content);

    Ok(LoadResult {
        entries,
        file_path: path.to_path_buf(),
        file_size,
    })
}

/// Findet alle Log-Dateien in einem Verzeichnis
pub fn find_log_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut log_files = Vec::new();

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "log" {
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
    fn test_format_file_size() {
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
        assert_eq!(format_file_size(2621440), "2.5 MB");
    }
}
