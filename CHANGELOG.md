# Changelog

Alle Änderungen an diesem Projekt sind in dieser Datei dokumentiert.

## [0.12.1] – 2026-05-23

- Versionsnummer im README-Abschnitt „Binary direkt herunterladen" (Dateinamen und curl-Befehl) wird beim Versions-Bump automatisch mitgepflegt.

## [0.12.0] – 2026-05-23

- ESC im Listen-Modus führt zurück zur Datei-Auswahl, auch wenn die App mit einer einzelnen Logdatei als Argument gestartet wurde (Geschwisterdateien im selben Verzeichnis werden automatisch erkannt). Erst ESC in der Auswahl beendet das Programm.
- Release-Workflow: `release-all.sh` setzt jetzt einen passenden Abschnitt in `CHANGELOG.md` voraus, übernimmt ihn in den Release-Commit und nutzt ihn als Body des GitHub-Releases (inkl. Link auf das CHANGELOG am Tag-Commit). `bump-version.sh` erinnert daran.

## [0.11.0] – 2026-05-15

- Update-Prüfung im Hintergrund: Hinweis auf neue Version in den Statusleisten und beim Beenden inklusive Update-Befehl je nach Installationsart.

## [0.10.0] – 2026-05-13

- Mehrfach-Markierung von Zeilen über `Shift+↑`/`Shift+↓` mit Löschaktion für den markierten Bereich.
- Backspace öffnet ein Lösch-Menü mit den Optionen: aktuelle Zeile, markierter Bereich, gesamte gefilterte Auswahl oder komplette Logdatei.

## [0.9.1] – 2026-05-12

- Hervorhebung neuer Einträge wird beim Drücken von ESC zurückgesetzt, bevor zur Datei-Auswahl gewechselt wird.
- Release-Skript korrigiert; lokaler Git-Tag wird nun vor dem GitHub-Release gesetzt.
- README aktualisiert.

## [0.9.0] – 2026-05-10

- Manuelle Zeitkorrektur (`t` mit `+` / `−` / `0`) zum stundenweisen Verschieben der Zeitstempel.
- Listen-Ansicht zeigt jetzt auch Sekunden im Zeitstempel an.
- Detail-Ansicht verwendet ein lokalisiertes Zeitformat.
- Fix: `q` in der Detail-Ansicht beendet das Programm statt nur zur Liste zurückzukehren.
- `--help`-Text überarbeitet, unnötige Shell-Ausgaben beim Start entfernt.
- Unit-Tests für `model`- und `parser`-Module ergänzt.

## [0.8.0] – 2026-05-08

- Inhalt der Detail-Ansicht in die Zwischenablage kopieren (`c`).
- Backspace-Shortcut zum Leeren der gesamten Logdatei (Grundlage für das spätere Lösch-Menü).
- Speicherbedarf reduziert: Streaming-Parser und inkrementelles Tail-Reload statt vollständigem Neu-Einlesen bei wachsenden Dateien.

## [0.7.0] – 2026-05-07

- Fix: UTF-8-Panic in der Listen-Ansicht behoben; Terminal wird im Fehlerfall sauber wiederhergestellt (Panic-Hook).
- Leere Logdateien werden in der Datei-Auswahl als nicht öffenbar markiert.

## [0.6.0] – 2026-05-04

- Neue Einträge, die während der Laufzeit ankommen, werden mit grünem Hintergrund hervorgehoben.
- Zusätzliche Tests.

## [0.5.0] – 2026-04-30

- Datei-Auswahl listet auch rotierte Logdateien mit auf.
- Dynamisches Paging in der Listen-Ansicht (Seitenhöhe folgt der Terminalgröße).

## [0.4.0] – 2026-04-26

- JSON-Rendering verbessert; der `exception`-Key in JSON-Daten ist standardmäßig eingeklappt (`e` zum Aufklappen).
- Listen-Ansicht: ausgerichtete Spalten und dezente Trennlinien zwischen Zeitstempel, Level und Nachricht.
- Detail-Ansicht: Wechsel zwischen Log-Einträgen direkt im Detail (`←`/`→`).
- Schnellfilter „Heute" hinzugefügt.
- Optimierte Hintergrundfarbe der Selektion (kollidiert nicht mehr mit Textfarben).
- Fix: Listen-Ansicht ließ sich unter bestimmten Bedingungen nicht selektieren.
- Aufgeräumtes README.

## [0.3.0] – 2026-04-24

- Fokusfilter auf identische Lognachrichten (`s`).

## [0.2.0] – 2026-04-24

- Filter für Datumsbereiche (`d`).
- Request-Fokus (`f`): zeigt alle Einträge zur Request-ID des aktuellen Eintrags.
- Versionsanzeige in der Fußleiste sowie neue CLI-Option `-v` / `--version`.
- Beim Wechsel von Detail/Viewer zurück zur Datei-Auswahl wird die zuletzt geöffnete Datei vorgemerkt.
- Periodisches Reload, wenn sich die Logdatei während der Anzeige ändert (Live-Tail).
- Initial-Auswahl springt auf den neuesten Eintrag am Dateiende.
- Ohne Argument wird `./var/log/` als Standardpfad genutzt, sofern vorhanden.
- Check-Setup-Skript, README-Erweiterungen und verbessertes Release-Handling.

## [0.1.1] – 2026-04-23

- Erste Patch-Version nach dem Initial-Release (kleinere Verbesserungen am Setup).

## [0.1.0] – 2026-04-23

- Erstes Release: interaktiver TUI-Viewer für TYPO3-Logdateien.
- Listen- und Detail-Ansicht mit JSON-Parsing der Nachrichten.
- Build-Skripte für macOS und Linux/Debian.
- Homebrew-Tap-Anbindung.
