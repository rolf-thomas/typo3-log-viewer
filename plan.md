# TYPO3 Log Viewer - Entwicklungsplan

## Ziel
Entwicklung eines kompilierbaren macOS Kommandozeilenprogramms zur interaktiven Darstellung von TYPO3-Logdateien.

## Log-Format Analyse

Die TYPO3-Logdateien folgen diesem Format:
```
DATUM [LEVEL] request="REQUEST_ID" component="COMPONENT": NACHRICHT
```

Beispiel:
```
Thu, 02 Apr 2026 12:00:02 +0200 [ERROR] request="043d54b20b2e8" component="WeberHaus.WhConnector.Service.SugarCrmRestService": Client error: ...
```

### Extrahierte Felder:
- **Zeitpunkt**: RFC 2822 Format (z.B. "Thu, 02 Apr 2026 12:00:02 +0200")
- **Level**: ERROR, DEBUG, INFO, WARNING
- **Request-ID**: Eindeutige Anfrage-ID
- **Component**: Modulpfad (z.B. "TYPO3.CMS.Core.Resource.ResourceStorage")
- **Nachricht**: Beschreibung, kann JSON-Daten enthalten

### Besonderheiten:
- Mehrzeilige Einträge (JSON in Folgezeilen)
- Dateigrössen bis zu mehreren MB

---

## Technologie-Empfehlung: Rust mit Ratatui

### Begründung:
1. **Performance**: Rust ist extrem schnell, ideal für große Logdateien
2. **Kompilierbar**: Erzeugt eine einzelne, native Binary ohne Abhängigkeiten
3. **TUI-Framework**: `ratatui` ist das modernste Terminal-UI Framework
4. **Memory-Safe**: Keine Buffer-Overflows bei der Verarbeitung großer Dateien
5. **Cross-Platform**: Funktioniert auf macOS, Linux und Windows

### Abhängigkeiten:
- `ratatui` - Terminal UI Framework
- `crossterm` - Terminal-Abstraktion
- `chrono` - Datums-Parsing
- `regex` oder `nom` - Log-Parsing

---

## Architektur

```
src/
├── main.rs           # Einstiegspunkt, Argument-Parsing
├── parser.rs         # Log-Parsing Logik
├── model.rs          # Datenstrukturen (LogEntry, LogLevel)
├── ui/
│   ├── mod.rs        # UI-Modul
│   ├── app.rs        # App-State und Event-Handling
│   ├── list_view.rs  # Listen-Ansicht
│   └── detail_view.rs # Detail-Ansicht
└── loader.rs         # Async/Streaming Datei-Laden
```

---

## Features

### Phase 1: Basis-Funktionalität
1. **Log-Parsing**
   - Regex-basiertes Parsen des TYPO3-Formats
   - Unterstützung für mehrzeilige Einträge
   - Erkennung aller Log-Levels

2. **Listen-Ansicht**
   - Scrollbare Liste aller Log-Einträge
   - Anzeige: Zeitpunkt (kurz) + Beschreibung (gekürzt)
   - Farbcodierung nach Log-Level (ERROR=rot, WARNING=gelb, DEBUG=grau)

3. **Detail-Ansicht**
   - Bei Auswahl einer Zeile: vollständige Informationen
   - Zeitpunkt, Level, Request-ID, Component
   - Vollständige Nachricht mit formatiertem JSON

### Phase 2: Erweiterte Features
4. **Filterung**
   - Nach Log-Level filtern
   - Volltextsuche in Nachrichten
   - Nach Component filtern

5. **Navigation**
   - Pfeiltasten für Navigation
   - Page Up/Down für schnelles Scrollen
   - Sprung zu Anfang/Ende (Home/End)

6. **Performance-Optimierungen**
   - Lazy Loading für sehr große Dateien
   - Virtualisiertes Scrolling

### Phase 3: Zusätzliche Features
7. **Datei-Auswahl**
   - Automatische Erkennung von Log-Dateien im var/log/ Verzeichnis
   - Wechsel zwischen Dateien

8. **Export**
   - Gefilterte Logs exportieren
   - Request-ID basierte Gruppierung

---

## UI-Mockup

### Listen-Ansicht:
```
┌─ TYPO3 Log Viewer ─────────────────────────────────────────┐
│ [var/log/wh_connector.log] 1,234 Einträge                  │
├────────────────────────────────────────────────────────────┤
│ 02.04.26 12:00 [ERROR] Client error: POST .../oauth2/token │
│ 02.04.26 13:00 [ERROR] Client error: POST .../oauth2/token │
│>02.04.26 14:00 [ERROR] Guzzle ClientException - {"code"... │
│ 02.04.26 14:18 [DEBUG] kisRequest - {"endpoint":"https:... │
│ 02.04.26 14:33 [DEBUG] kisRequest - {"endpoint":"https:... │
├────────────────────────────────────────────────────────────┤
│ ↑↓ Navigate  Enter: Details  f: Filter  q: Quit           │
└────────────────────────────────────────────────────────────┘
```

### Detail-Ansicht:
```
┌─ Log Details ──────────────────────────────────────────────┐
│ Zeitpunkt:  Thu, 02 Apr 2026 14:00:02 +0200                │
│ Level:      ERROR                                          │
│ Request:    fd54bf1f3eccb                                  │
│ Component:  WeberHaus.WhConnector.Service.SugarCrmRest...  │
├────────────────────────────────────────────────────────────┤
│ Nachricht:                                                 │
│ Client error: `POST https://crm-live.weberhaus.de/...`     │
│ resulted in a `400 Bad Request Unknown` response:          │
│ {                                                          │
│   "error": "invalid_grant",                                │
│   "error_message": "Invalid refresh token"                 │
│ }                                                          │
├────────────────────────────────────────────────────────────┤
│ ESC: Zurück zur Liste                                      │
└────────────────────────────────────────────────────────────┘
```

---

## Implementierungsschritte

### Schritt 1: Projekt-Setup
- Rust-Projekt initialisieren mit `cargo init`
- Dependencies in Cargo.toml hinzufügen
- Grundstruktur der Dateien anlegen

### Schritt 2: Log-Parser
- LogEntry Struct definieren
- Regex für das TYPO3-Format erstellen
- Parser-Funktion implementieren
- Unit-Tests für verschiedene Log-Formate

### Schritt 3: Datei-Loader
- Funktion zum Einlesen einer Log-Datei
- Handling von mehrzeiligen Einträgen
- Performance-Tests mit großen Dateien

### Schritt 4: Basis-UI
- Ratatui App-Struktur aufsetzen
- Event-Loop implementieren
- Listen-Widget mit Scrolling

### Schritt 5: Detail-Ansicht
- Split-View oder Modal für Details
- JSON-Formatierung
- Navigation zwischen Views

### Schritt 6: Filterung & Suche
- Filter-Modus implementieren
- Level-Filter
- Textsuche

### Schritt 7: Polish & Build
- Tastenkürzel dokumentieren
- Release-Build optimieren
- README mit Installationsanleitung

---

## Kompilierung & Nutzung

```bash
# Debug-Build
cargo build

# Release-Build (optimiert)
cargo build --release

# Binary liegt dann in: target/release/typo3-log-viewer

# Nutzung
./typo3-log-viewer var/log/wh_connector.log
# oder
./typo3-log-viewer var/log/
```

---

## Alternative: Swift (falls bevorzugt)

Falls Swift bevorzugt wird:
- Nutze `Swift Argument Parser` für CLI
- `swift-tui` oder `Termbox` für Terminal-UI
- Kompiliert ebenfalls zu nativer macOS-Binary
- Bessere macOS-Integration möglich

Nachteil: Swift-TUI-Libraries sind weniger ausgereift als Rust's ratatui.
