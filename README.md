# TYPO3 Log Viewer

Interaktiver Kommandozeilen-Viewer für TYPO3-Logdateien mit Listen- und Detailansicht.

## Features

- Schnelles Parsen auch großer TYPO3-Logdateien
- Interaktive Listenansicht mit Zeitpunkt, Level und Beschreibung
- Detailansicht mit Request-ID, Component und formatierten JSON-Daten
- Filterung nach Log-Level und Volltextsuche
- Farbcodierte Log-Levels (Error=rot, Warning=gelb, Info=grün, Debug=grau)
- Interaktive Dateiauswahl bei mehreren Log-Dateien im Verzeichnis

## Installation

### Voraussetzungen auf einem neuen macOS-System

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

### Bauen aus dem Quellcode

```bash
git clone <repo-url>
cd typo3-log-viewer
./check-setup.sh   # Voraussetzungen prüfen
./build.sh         # Release-Binary bauen und signieren
```

Das Build-Script kompiliert die Release-Version und signiert die Binary ad-hoc, um macOS Gatekeeper-Warnungen zu vermeiden.

Die fertige Binary liegt unter `target/release/typo3-log-viewer`.

### Manuelle Installation

```bash
cp target/release/typo3-log-viewer /usr/local/bin/
```

## Nutzung

### Einzelne Log-Datei öffnen

```bash
typo3-log-viewer var/log/wh_connector.log
```

### Verzeichnis mit interaktiver Dateiauswahl

```bash
typo3-log-viewer var/log/
```

Zeigt eine auswählbare Liste aller `.log`-Dateien im Verzeichnis.

## Tastenkürzel

### Listenansicht

| Taste | Funktion |
|-------|----------|
| ↑/↓, j/k | Navigation |
| PgUp/PgDown | Seitenweises Scrollen |
| Home/g | Zum Anfang |
| End/G | Zum Ende |
| Enter | Detailansicht öffnen |
| `/` | Textsuche |
| 1-4 | Level-Filter (1=Error, 2=Warning, 3=Info, 4=Debug) |
| 0 | Filter zurücksetzen |
| ? | Hilfe |
| q, ESC | Beenden |

### Detailansicht

| Taste | Funktion |
|-------|----------|
| ↑/↓, j/k | Scrollen |
| PgUp/PgDown | Seitenweises Scrollen |
| ESC, Enter | Zurück zur Liste |

### Dateiauswahl

| Taste | Funktion |
|-------|----------|
| ↑/↓, j/k | Navigation |
| Enter | Datei öffnen |
| q, ESC | Abbrechen |

## Distribution auf andere Macs

Da die Binary nur ad-hoc signiert ist (ohne Apple Developer Account), zeigt macOS beim ersten Start auf einem fremden Mac eine Warnung:

> "Apple konnte nicht überprüfen, ob frei von Schadsoftware ist..."

### Lösung für den Empfänger

Einmalig im Terminal ausführen:

```bash
xattr -d com.apple.quarantine /pfad/zu/typo3-log-viewer
```

Alternativ über die GUI:

1. **Systemeinstellungen** → **Datenschutz & Sicherheit**
2. Unter "Sicherheit" erscheint die blockierte App
3. **"Trotzdem öffnen"** klicken

### Empfehlung für dauerhafte Distribution

Selbst kompilieren auf dem Ziel-Mac verhindert die Warnung komplett:

```bash
git clone <repo-url>
cd typo3-log-viewer
./build.sh
```

## Linux / Debian

Das Tool läuft auch unter Linux (Debian, Ubuntu, Fedora, Arch, etc.). macOS-Binaries funktionieren **nicht** direkt — es muss eine Linux-Binary gebaut werden. Es gibt zwei Wege:

### Variante A: Native Build auf Debian (einfachste Lösung)

Auf dem Debian-System Rust installieren und bauen:

```bash
# Rust installieren
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Projekt bauen
git clone <repo-url>
cd typo3-log-viewer
cargo build --release

# Binary liegt dann unter:
./target/release/typo3-log-viewer
```

Systemabhängigkeiten auf Debian/Ubuntu (falls noch nicht vorhanden):

```bash
sudo apt install build-essential pkg-config
```

### Variante B: Cross-Compilation vom Mac aus

Für Distribution mehrerer Architekturen ohne Zugriff auf ein Debian-System.

Das Script `build-linux.sh` erkennt automatisch, welches Backend verfügbar ist. Es gibt zwei Optionen — **Zig ist empfohlen** (kein Docker nötig, schneller, kleinerer Footprint):

#### Option B1: Zig als Cross-Linker (empfohlen)

```bash
brew install zig
cargo install cargo-zigbuild
./build-linux.sh
```

Vorteile:
- Keine Docker-Installation nötig
- Schnellere Builds (direkt, ohne Container)
- Funktioniert auch in CI/CD ohne Privileged-Mode

#### Option B2: cross mit Docker

Voraussetzungen:
- [Docker Desktop](https://www.docker.com/products/docker-desktop/) installiert und gestartet
- `cross`-Tool:
  ```bash
  cargo install cross --git https://github.com/cross-rs/cross
  ```

```bash
./build-linux.sh
```

Das Script baut für zwei Targets:

| Target | Plattform |
|--------|-----------|
| `x86_64-unknown-linux-gnu` | Standard-PCs, Server, Intel/AMD |
| `aarch64-unknown-linux-gnu` | ARM-Geräte, Raspberry Pi 4/5, ARM-Server |

Ergebnis:

```
target/x86_64-unknown-linux-gnu/release/typo3-log-viewer
target/aarch64-unknown-linux-gnu/release/typo3-log-viewer
```

Einzelnes Target bauen:

```bash
./build-linux.sh x86_64-unknown-linux-gnu
```

### Kompatibilität

Die gebauten Linux-Binaries sind **dynamisch gegen glibc gelinkt**. Sie funktionieren auf Debian/Ubuntu-Versionen, deren glibc mindestens so aktuell ist wie die des Build-Systems. `cross` verwendet standardmäßig ein ausreichend altes Base-Image (Ubuntu mit glibc 2.17/2.27), sodass aktuelle Debian-Systeme (Bullseye/Bookworm und neuer) problemlos unterstützt werden.

Für maximale Portabilität (z.B. Alpine Linux, statisch gelinkte Binaries) kann `x86_64-unknown-linux-musl` verwendet werden:

```bash
rustup target add x86_64-unknown-linux-musl
./build-linux.sh x86_64-unknown-linux-musl
```

### Installation auf Debian

```bash
# Binary kopieren
sudo cp typo3-log-viewer /usr/local/bin/
sudo chmod +x /usr/local/bin/typo3-log-viewer

# Aufruf
typo3-log-viewer /var/log/typo3/
```

## Homebrew-Distribution

Das Tool kann über einen eigenen Homebrew-Tap verteilt werden — für macOS (arm64 + Intel) und Linux (über Linuxbrew). Sowohl öffentliche als auch **private Taps** werden unterstützt.

### Einmaliges Setup: Tap-Repository anlegen

Homebrew-Taps müssen den Namen `homebrew-<name>` tragen. Beispiel:

```bash
gh repo create rolf-thomas/homebrew-tools --public
```

Struktur im Tap-Repo:

```
homebrew-tools/
└── Formula/
    └── typo3-log-viewer.rb
```

Kopiere `Formula/typo3-log-viewer.rb` aus diesem Projekt ins Tap-Repo. Die URLs sind bereits auf `rolf-thomas` gesetzt.

### Release-Workflow

Für jedes neue Release:

```bash
# 1. Alle Binaries bauen
./build.sh                              # macOS arm64 (signiert)
cargo build --release --target x86_64-apple-darwin   # macOS Intel
codesign --force --deep -s - target/x86_64-apple-darwin/release/typo3-log-viewer
./build-linux.sh x86_64-unknown-linux-musl aarch64-unknown-linux-gnu

# 2. Release-Tarballs + SHA256 erzeugen
./release.sh

# 3. GitHub Release anlegen und Tarballs hochladen
gh release create v0.1.0 --title "v0.1.0" dist/*.tar.gz

# 4. Formel aktualisieren (Version + SHA256 aus dist/formula-snippet.rb)
# 5. Im Tap-Repo committen und pushen
```

### Installation durch Nutzer

```bash
brew tap rolf-thomas/tools
brew install typo3-log-viewer
```

Update mit `brew update && brew upgrade typo3-log-viewer`.

> Hinweis: Falls du später auf einen privaten Tap wechseln möchtest, brauchst du einen `HOMEBREW_GITHUB_API_TOKEN` (mit Scope `repo`) oder SSH-Zugriff auf das Repo.

### Selbst-gehosteter Tap (Alternative)

Falls du später auf GitLab/Gitea wechselst:

```bash
brew tap rolf-thomas/tools https://gitlab.example.com/rolf-thomas/homebrew-tools.git
```

Die Release-Tarballs können auch auf einem beliebigen HTTPS-Server liegen (S3, interner Webserver). In der Formel wird einfach die URL-Basis angepasst.

## Unterstütztes Log-Format

Der Viewer erwartet das Standard-TYPO3-Logformat:

```
DATUM [LEVEL] request="REQUEST_ID" component="COMPONENT": NACHRICHT
```

Beispiel:

```
Thu, 02 Apr 2026 12:00:02 +0200 [ERROR] request="043d54b20b2e8" component="WeberHaus.WhConnector.Service.SugarCrmRestService": Client error: ...
```

Mehrzeilige Einträge (z. B. JSON-Folgezeilen) werden automatisch erkannt und dem zugehörigen Log-Eintrag zugeordnet.

## Projektstruktur

```
src/
├── main.rs          # CLI, Argument-Handling, Dateiauswahl
├── model.rs         # Datenstrukturen (LogEntry, LogLevel, LogFilter)
├── parser.rs        # Log-Parser mit Regex und JSON-Extraktion
├── loader.rs        # Datei-Laden und Verzeichnis-Scan
└── ui/
    ├── mod.rs       # UI-Modul
    └── app.rs       # Ratatui-Anwendung, Views, Event-Handling
```

## Technologie-Stack

- **[Rust](https://www.rust-lang.org/)** – Systemsprache für Performance und Sicherheit
- **[ratatui](https://ratatui.rs/)** – Terminal-UI-Framework
- **[crossterm](https://github.com/crossterm-rs/crossterm)** – Terminal-Abstraktion
- **[chrono](https://github.com/chronotope/chrono)** – Datumsparsing
- **[regex](https://github.com/rust-lang/regex)** – Log-Format-Parsing
- **[serde_json](https://github.com/serde-rs/json)** – JSON-Formatierung

## Entwicklung

```bash
# Debug-Build
cargo build

# Tests ausführen
cargo test

# Release-Build mit Signierung
./build.sh
```
