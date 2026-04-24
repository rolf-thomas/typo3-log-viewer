#!/bin/bash
# Erhöht die Version in Cargo.toml nach SemVer-Regeln.
#
# Nutzung:
#   ./bump-version.sh              Zeigt die aktuelle Version
#   ./bump-version.sh major        0.1.1 → 1.0.0
#   ./bump-version.sh minor        0.1.1 → 0.2.0
#   ./bump-version.sh patch        0.1.1 → 0.1.2
#   ./bump-version.sh 1.2.3        Setzt explizit auf 1.2.3

set -e

ROOT=$(cd "$(dirname "$0")" && pwd)
cd "$ROOT"

CARGO="Cargo.toml"

if [ ! -f "$CARGO" ]; then
  echo "Fehler: $CARGO nicht gefunden."
  exit 1
fi

# Aktuelle Version aus Cargo.toml
current=$(awk -F'"' '/^version/ { print $2; exit }' "$CARGO")

if [ -z "$current" ]; then
  echo "Fehler: Konnte version aus $CARGO nicht lesen."
  exit 1
fi

bump="$1"

# Ohne Argument: nur anzeigen
if [ -z "$bump" ]; then
  echo "Aktuelle Version: $current"
  echo ""
  echo "Nutzung: $0 {major|minor|patch|<x.y.z>}"
  exit 0
fi

# Aktuelle Version zerlegen
if ! [[ "$current" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
  echo "Fehler: Aktuelle Version '$current' entspricht nicht dem Format x.y.z"
  exit 1
fi
major="${BASH_REMATCH[1]}"
minor="${BASH_REMATCH[2]}"
patch="${BASH_REMATCH[3]}"

# Neue Version berechnen
case "$bump" in
  major)
    major=$((major + 1))
    minor=0
    patch=0
    ;;
  minor)
    minor=$((minor + 1))
    patch=0
    ;;
  patch)
    patch=$((patch + 1))
    ;;
  *)
    if [[ "$bump" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
      major="${BASH_REMATCH[1]}"
      minor="${BASH_REMATCH[2]}"
      patch="${BASH_REMATCH[3]}"
    else
      echo "Fehler: Ungültiges Bump-Argument '$bump'"
      echo "Erlaubt: major | minor | patch | <x.y.z>"
      exit 1
    fi
    ;;
esac

new="$major.$minor.$patch"

if [ "$new" = "$current" ]; then
  echo "Version ist bereits $current. Kein Bump nötig."
  exit 0
fi

# Cargo.toml patchen (erste version-Zeile, die nach [package] kommt)
sed -i '' "s/^version = \"$current\"/version = \"$new\"/" "$CARGO"

# Verifizieren
actual=$(awk -F'"' '/^version/ { print $2; exit }' "$CARGO")
if [ "$actual" != "$new" ]; then
  echo "Fehler: Version-Update fehlgeschlagen (erwartet $new, ist $actual)"
  exit 1
fi

# Cargo.lock auf neuen Stand bringen
if command -v cargo >/dev/null 2>&1; then
  cargo update --workspace --package typo3-log-viewer >/dev/null 2>&1 || true
fi

echo "Version: $current → $new"
echo ""
echo "Nächste Schritte:"
echo "  git add Cargo.toml Cargo.lock && git commit -m \"Bump version to $new\" && git push"
echo "  ./release-all.sh"
echo ""
echo "Optional – Tap-Repo automatisch pushen:"
echo "  TAP_PATH=~/Code/Tools/homebrew-tools ./release-all.sh"
