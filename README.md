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

### Bauen aus dem Quellcode

Voraussetzung: [Rust](https://rustup.rs/) (Rust >= 1.75)

```bash
git clone <repo-url>
cd typo3-log-viewer
./build.sh
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
