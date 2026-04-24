# TYPO3 Log Viewer — Entwicklerdokumentation

## Voraussetzungen auf einem neuen macOS-System

Für den vollständigen Build werden folgende Komponenten benötigt:

| Komponente | Pflicht | Zweck |
|---|---|---|
| Xcode Command Line Tools | ja | C-Linker, `codesign` |
| [Homebrew](https://brew.sh/) | empfohlen | Paketverwaltung |
| [Rust >= 1.75](https://rustup.rs/) | ja | Compiler + cargo |
| [Zig](https://ziglang.org/) + cargo-zigbuild | optional | Linux-Cross-Build ohne Docker |
| [GitHub CLI (`gh`)](https://cli.github.com/) | optional | Release-Workflow |

Das mitgelieferte Script prüft alle Voraussetzungen und bietet an, fehlende Komponenten direkt zu installieren:

```bash
./check-setup.sh
```

## Bauen aus dem Quellcode

```bash
git clone https://github.com/rolf-thomas/typo3-log-viewer.git
cd typo3-log-viewer
./check-setup.sh   # Voraussetzungen prüfen
./build.sh         # Release-Binary bauen und signieren (macOS arm64)
```

Die fertige Binary liegt unter `target/release/typo3-log-viewer`.

```bash
# Debug-Build
cargo build

# Tests ausführen
cargo test
```

## Projektstruktur

```
src/
├── main.rs          # CLI, Argument-Handling, interaktive Dateiauswahl
├── model.rs         # Datenstrukturen: LogEntry, LogLevel, LogFilter
├── parser.rs        # Log-Parser (Regex + JSON-Extraktion mit Klammer-Matching)
├── loader.rs        # Datei-Laden und Verzeichnis-Scan
└── ui/
    ├── mod.rs       # UI-Modul
    └── app.rs       # Ratatui-App, Views, Event-Handling

Formula/
└── typo3-log-viewer.rb   # Homebrew-Formel

build.sh             # macOS arm64 Release-Build + ad-hoc codesign
build-linux.sh       # Linux Cross-Build (Zig preferred, Docker/cross als Fallback)
release.sh           # Erzeugt dist/*.tar.gz + SHA256 für Homebrew
bump-version.sh      # Versionsnummer hochsetzen (major/minor/patch)
update-formula.sh    # Homebrew-Formel mit neuen SHA256-Werten patchen
release-all.sh       # Kombinierter Release-Workflow (baut alles + GitHub Release)
check-setup.sh       # Prüft Entwicklungsvoraussetzungen, bietet Installation an
```

## Technologie-Stack

| Komponente | Technologie |
|---|---|
| Sprache | Rust (>= 1.75) |
| TUI | ratatui 0.29 |
| Terminal | crossterm 0.28 |
| Log-Parsing | regex + serde_json |
| Cross-Compilation | cargo-zigbuild (Zig) |
| Code-Signierung | `codesign` (ad-hoc, kein Apple Developer Account) |
| Distribution | Homebrew Tap via GitHub Releases |

## Linux-Build

macOS-Binaries funktionieren **nicht** direkt auf Linux. Zwei Optionen:

### Variante A: Native auf Linux

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install build-essential pkg-config   # Debian/Ubuntu
cargo build --release
```

### Variante B: Cross-Compilation vom Mac (empfohlen)

```bash
brew install zig
cargo install cargo-zigbuild
./build-linux.sh x86_64-unknown-linux-musl aarch64-unknown-linux-gnu
```

Das Script `build-linux.sh` wählt automatisch zwischen `cargo-zigbuild` (Zig) und `cross` (Docker).

Targets:

| Target | Plattform |
|--------|-----------|
| `x86_64-unknown-linux-musl` | Linux x86_64, statisch gelinkt (empfohlen) |
| `aarch64-unknown-linux-gnu` | ARM64 Linux (Raspberry Pi, ARM-Server) |
| `x86_64-unknown-linux-gnu` | Linux x86_64, dynamisch (glibc) |

## Release-Workflow

```bash
# 1. Version bumpen
./bump-version.sh minor   # 0.2.0 → 0.3.0

# 2. Committen
git add Cargo.toml Cargo.lock
git commit -m "Bump version to 0.3.0"
git push

# 3. Alles bauen, Tarballs erzeugen, GitHub Release anlegen, Formel patchen
./release-all.sh

# 4. Formel ins Tap-Repo pushen (optional: automatisch via TAP_PATH)
TAP_PATH=~/Code/Tools/homebrew-tools ./release-all.sh
```

`release-all.sh` erledigt automatisch:
- macOS arm64 + Intel Build + Signierung
- Linux x86_64 (musl) + ARM64 Cross-Build
- Tarballs + SHA256 in `dist/`
- Homebrew-Formel patchen (`update-formula.sh`)
- GitHub Release mit allen Tarballs anlegen

## Homebrew-Tap-Setup (einmalig)

```bash
gh repo create rolf-thomas/homebrew-tools --public
```

Struktur im Tap-Repo:
```
homebrew-tools/
└── Formula/
    └── typo3-log-viewer.rb
```

GitHub-Repo: `rolf-thomas/typo3-log-viewer`
Homebrew-Tap-Repo: `rolf-thomas/homebrew-tools`

## macOS Gatekeeper

Die Binary ist nur ad-hoc signiert (kein Apple Developer Account). Beim Empfänger einmalig:

```bash
xattr -d com.apple.quarantine /pfad/zu/typo3-log-viewer
```

Oder: **Systemeinstellungen → Datenschutz & Sicherheit → "Trotzdem öffnen"**

Wer die Warnung komplett vermeiden will, baut lokal mit `./build.sh`.
