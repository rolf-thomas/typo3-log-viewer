//! Update-Check gegen GitHub Releases.
//!
//! Läuft in einem Hintergrund-Thread bei jedem Start und stellt einen Stern
//! plus Shell-Hinweis nach Beenden bereit, wenn eine neuere Version verfügbar ist.

use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/rolf-thomas/typo3-log-viewer/releases/latest";
const RELEASES_PAGE_URL: &str = "https://github.com/rolf-thomas/typo3-log-viewer/releases/latest";
const HTTP_TIMEOUT: Duration = Duration::from_secs(3);

/// Wie das Binary installiert wurde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMethod {
    /// Über Homebrew (macOS oder Linuxbrew)
    Homebrew,
    /// Manuell heruntergeladen oder selbst gebaut
    Manual,
}

impl InstallMethod {
    /// Konkrete Update-Anweisung als String für die Shell-Ausgabe.
    pub fn update_command(self) -> String {
        match self {
            InstallMethod::Homebrew => {
                "brew update && brew upgrade typo3-log-viewer".to_string()
            }
            InstallMethod::Manual => {
                format!("Neuste Version herunterladen: {}", RELEASES_PAGE_URL)
            }
        }
    }
}

/// Ergebnis eines Update-Checks.
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub install_method: InstallMethod,
}

/// Shared State, in den der Hintergrund-Thread das Ergebnis schreibt.
pub type UpdateState = Arc<Mutex<Option<UpdateInfo>>>;

/// Startet den asynchronen Update-Check.
///
/// Rückgabe ist ein gemeinsam genutzter State, der nach erfolgreichem
/// Check `Some(UpdateInfo)` enthält, wenn die latest-Version > der
/// laufenden Version ist. Bei Fehlern, identischer Version oder Timeout
/// bleibt der State `None`.
pub fn start_check() -> (UpdateState, JoinHandle<()>) {
    let state: UpdateState = Arc::new(Mutex::new(None));
    let state_clone = Arc::clone(&state);

    let handle = thread::spawn(move || {
        let current_version = env!("CARGO_PKG_VERSION");
        let install_method = detect_install_method();

        // Test-Hook: erlaubt das Erzwingen einer "neusten" Version ohne Netz.
        // Nutzung: TYPO3_LOG_VIEWER_FAKE_LATEST=9.9.9 ./typo3-log-viewer …
        let latest = if let Ok(fake) = std::env::var("TYPO3_LOG_VIEWER_FAKE_LATEST") {
            if fake.trim().is_empty() {
                None
            } else {
                Some(normalize_version(&fake))
            }
        } else {
            fetch_latest_version()
        };

        if let Some(latest) = latest {
            if is_newer(&latest, current_version) {
                let mut guard = state_clone.lock().unwrap();
                *guard = Some(UpdateInfo {
                    latest_version: latest,
                    install_method,
                });
            }
        }
    });

    (state, handle)
}

/// Liest den aktuellen Stand des Update-Checks (non-blocking).
pub fn current(state: &UpdateState) -> Option<UpdateInfo> {
    state.lock().ok().and_then(|g| g.clone())
}

/// Erkennt anhand des Pfads der laufenden Binary, wie sie installiert wurde.
fn detect_install_method() -> InstallMethod {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return InstallMethod::Manual,
    };
    let path_str = exe.to_string_lossy();

    // Typische Brew-Präfixe: macOS arm64, macOS Intel, Linuxbrew
    const BREW_PREFIXES: &[&str] = &[
        "/opt/homebrew/",
        "/usr/local/Cellar/",
        "/usr/local/opt/",
        "/home/linuxbrew/.linuxbrew/",
    ];

    for prefix in BREW_PREFIXES {
        if path_str.starts_with(prefix) {
            return InstallMethod::Homebrew;
        }
    }

    // Cellar kann auch unter macOS bei alternativer Brew-Konfiguration anders liegen;
    // wir prüfen daher zusätzlich auf das übliche Pfadsegment.
    if path_str.contains("/Cellar/typo3-log-viewer/") {
        return InstallMethod::Homebrew;
    }

    InstallMethod::Manual
}

/// Holt die neuste Version von der GitHub Releases API.
/// Gibt `None` zurück bei jedem Fehler (Netz, Parse, Timeout).
fn fetch_latest_version() -> Option<String> {
    let agent = ureq::AgentBuilder::new().timeout(HTTP_TIMEOUT).build();

    let response = agent
        .get(GITHUB_RELEASES_URL)
        .set(
            "User-Agent",
            concat!("typo3-log-viewer/", env!("CARGO_PKG_VERSION")),
        )
        .set("Accept", "application/vnd.github+json")
        .call()
        .ok()?;

    let json: serde_json::Value = response.into_json().ok()?;
    let tag = json.get("tag_name")?.as_str()?;
    Some(normalize_version(tag))
}

/// Entfernt ein führendes "v" aus einem Versions-Tag.
fn normalize_version(tag: &str) -> String {
    tag.trim().trim_start_matches('v').to_string()
}

/// Vergleicht zwei semver-ähnliche Versionsstrings (`a > b`?).
/// Behandelt fehlende oder nicht-numerische Komponenten konservativ.
fn is_newer(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> Vec<u32> {
        s.split(|c: char| c == '.' || c == '-')
            .map(|p| p.parse::<u32>().unwrap_or(0))
            .collect()
    };
    let av = parse(a);
    let bv = parse(b);
    let len = av.len().max(bv.len());
    for i in 0..len {
        let ai = av.get(i).copied().unwrap_or(0);
        let bi = bv.get(i).copied().unwrap_or(0);
        if ai > bi {
            return true;
        }
        if ai < bi {
            return false;
        }
    }
    false
}

// --- Tests --------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert!(is_newer("0.10.0", "0.9.1"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.10.0", "0.10.0"));
        assert!(!is_newer("0.9.0", "0.10.0"));
        assert!(is_newer("0.10.1", "0.10.0"));
    }

    #[test]
    fn version_normalize() {
        assert_eq!(normalize_version("v0.10.0"), "0.10.0");
        assert_eq!(normalize_version("0.10.0"), "0.10.0");
        assert_eq!(normalize_version("  v1.2.3  "), "1.2.3");
    }
}
