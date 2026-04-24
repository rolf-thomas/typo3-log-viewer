#!/bin/bash
# Patcht Formula/typo3-log-viewer.rb automatisch mit:
#   - Version aus Cargo.toml ([package].version)
#   - SHA256-Werten aus dist/checksums.txt
#
# Voraussetzungen:
#   - ./release.sh wurde ausgeführt (dist/ existiert mit Tarballs + checksums.txt)
#   - Die Tarball-Namen folgen typo3-log-viewer-<version>-<suffix>.tar.gz
#
# Nutzung: ./update-formula.sh

set -e

ROOT=$(cd "$(dirname "$0")" && pwd)
cd "$ROOT"

FORMULA="Formula/typo3-log-viewer.rb"
CHECKSUMS="dist/checksums.txt"
CARGO="Cargo.toml"

# --- Voraussetzungen prüfen ---

if [ ! -f "$FORMULA" ]; then
  echo "Fehler: $FORMULA nicht gefunden."
  exit 1
fi

if [ ! -f "$CHECKSUMS" ]; then
  echo "Fehler: $CHECKSUMS nicht gefunden."
  echo "Führe zuerst ./release.sh aus."
  exit 1
fi

# --- Version aus Cargo.toml ---

VERSION=$(awk -F'"' '/^version/ { print $2; exit }' "$CARGO")

if [ -z "$VERSION" ]; then
  echo "Fehler: Konnte version aus $CARGO nicht lesen."
  exit 1
fi

# --- SHA256-Werte aus checksums.txt extrahieren ---

get_hash() {
  local suffix="$1"
  awk -v pattern="typo3-log-viewer-$VERSION-$suffix.tar.gz" \
    '$2 == pattern { print $1 }' "$CHECKSUMS"
}

SHA_MACOS_ARM64=$(get_hash "macos-arm64")
SHA_MACOS_X86_64=$(get_hash "macos-x86_64")
SHA_LINUX_ARM64=$(get_hash "linux-arm64")
SHA_LINUX_MUSL=$(get_hash "linux-x86_64-musl")

# Sanity-Check: Alle Hashes vorhanden?
missing=0
for pair in \
  "macos-arm64:$SHA_MACOS_ARM64" \
  "macos-x86_64:$SHA_MACOS_X86_64" \
  "linux-arm64:$SHA_LINUX_ARM64" \
  "linux-x86_64-musl:$SHA_LINUX_MUSL"; do
  suffix="${pair%:*}"
  hash="${pair#*:}"
  if [ -z "$hash" ]; then
    echo "Fehler: SHA256 für $suffix fehlt (Tarball für Version $VERSION nicht in $CHECKSUMS gefunden)"
    missing=1
  fi
done

if [ $missing -ne 0 ]; then
  echo ""
  echo "Inhalt von $CHECKSUMS:"
  cat "$CHECKSUMS"
  exit 1
fi

# --- Formel patchen ---

TMP=$(mktemp)

awk -v version="$VERSION" \
    -v sha_macos_arm64="$SHA_MACOS_ARM64" \
    -v sha_macos_x86_64="$SHA_MACOS_X86_64" \
    -v sha_linux_arm64="$SHA_LINUX_ARM64" \
    -v sha_linux_musl="$SHA_LINUX_MUSL" '
  # Version-Zeile ersetzen
  /^  version "[^"]*"/ {
    sub(/version "[^"]*"/, "version \"" version "\"")
    print
    next
  }
  # url für macOS arm64 → nächste Zeile (sha256) patchen
  /macos-arm64\.tar\.gz/ {
    print
    if ((getline line) > 0) {
      sub(/sha256 "[^"]*"/, "sha256 \"" sha_macos_arm64 "\"", line)
      print line
    }
    next
  }
  /macos-x86_64\.tar\.gz/ {
    print
    if ((getline line) > 0) {
      sub(/sha256 "[^"]*"/, "sha256 \"" sha_macos_x86_64 "\"", line)
      print line
    }
    next
  }
  /linux-arm64\.tar\.gz/ {
    print
    if ((getline line) > 0) {
      sub(/sha256 "[^"]*"/, "sha256 \"" sha_linux_arm64 "\"", line)
      print line
    }
    next
  }
  /linux-x86_64-musl\.tar\.gz/ {
    print
    if ((getline line) > 0) {
      sub(/sha256 "[^"]*"/, "sha256 \"" sha_linux_musl "\"", line)
      print line
    }
    next
  }
  { print }
' "$FORMULA" > "$TMP"

mv "$TMP" "$FORMULA"

# --- Ergebnis ---

echo "Formel aktualisiert auf Version $VERSION"
echo ""
echo "Aktuelle Werte in $FORMULA:"
grep -n -E '^  version |sha256 "' "$FORMULA"
