use std::io::{self, Write};
use std::process::{Command, Stdio};

/// Versucht, einen Text in die System-Zwischenablage zu kopieren.
///
/// Nutzt plattformabhängig externe Tools, statt eine native Clipboard-Crate
/// als Dependency aufzunehmen:
/// - macOS:   `pbcopy`
/// - Linux:   `wl-copy` (Wayland), Fallback `xclip -selection clipboard`
/// - Windows: `clip.exe`
///
/// Liefert einen sprechenden Fehler, wenn kein Tool gefunden / ausführbar ist.
pub fn copy_to_clipboard(text: &str) -> io::Result<()> {
    let candidates = clipboard_commands();

    let mut last_error: Option<io::Error> = None;
    for (program, args) in candidates {
        match try_copy(program, args, text) {
            Ok(()) => return Ok(()),
            Err(e) => last_error = Some(e),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Kein unterstütztes Clipboard-Tool gefunden",
        )
    }))
}

fn clipboard_commands() -> Vec<(&'static str, &'static [&'static str])> {
    if cfg!(target_os = "macos") {
        vec![("pbcopy", &[])]
    } else if cfg!(target_os = "windows") {
        vec![("clip.exe", &[]), ("clip", &[])]
    } else {
        // Linux / BSD: zuerst Wayland, dann X11
        vec![
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ]
    }
}

fn try_copy(program: &str, args: &[&str], text: &str) -> io::Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| io::Error::other("stdin nicht verfügbar"))?;
        stdin.write_all(text.as_bytes())?;
    }

    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{} beendete sich mit Status {}",
            program, status
        )))
    }
}
