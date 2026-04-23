#!/bin/bash
# Erzeugt Release-Tarballs aus vorhandenen Binaries und gibt die
# Homebrew-Formel-Snippets mit URLs und SHA256-Checksummen aus.
#
# Voraussetzungen:
#   ./build.sh         (macOS arm64 gebaut + signiert)
#   ./build-linux.sh   (optional, für Linux-Binaries)
#   Intel-Mac-Build:   cargo build --release --target x86_64-apple-darwin
#
# Argumente:
#   $1  Version (z.B. 0.1.0) — wird aus Cargo.toml gelesen, wenn nicht angegeben
#
# Output: dist/
#   typo3-log-viewer-<version>-<target>.tar.gz
#   checksums.txt
#   formula-snippet.rb

set -e

ROOT=$(cd "$(dirname "$0")" && pwd)
cd "$ROOT"

VERSION="${1:-$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)}"
DIST="$ROOT/dist"
rm -rf "$DIST"
mkdir -p "$DIST"

NAME="typo3-log-viewer"

# Tabelle: Target → menschenfreundlicher Suffix
declare -a BUILDS=(
  "target/release|macos-arm64"
  "target/x86_64-apple-darwin/release|macos-x86_64"
  "target/x86_64-unknown-linux-gnu/release|linux-x86_64"
  "target/aarch64-unknown-linux-gnu/release|linux-arm64"
  "target/x86_64-unknown-linux-musl/release|linux-x86_64-musl"
)

declare -a INCLUDED_SUFFIXES=()
declare -a INCLUDED_HASHES=()

for BUILD in "${BUILDS[@]}"; do
  SRC_DIR="${BUILD%|*}"
  SUFFIX="${BUILD#*|}"
  BIN="$SRC_DIR/$NAME"

  if [ ! -f "$BIN" ]; then
    echo "Übersprungen: $BIN (nicht gebaut)"
    continue
  fi

  TARBALL="$NAME-$VERSION-$SUFFIX.tar.gz"
  tar -czf "$DIST/$TARBALL" -C "$SRC_DIR" "$NAME"

  HASH=$(shasum -a 256 "$DIST/$TARBALL" | cut -d' ' -f1)
  echo "$HASH  $TARBALL" >> "$DIST/checksums.txt"

  INCLUDED_SUFFIXES+=("$SUFFIX")
  INCLUDED_HASHES+=("$HASH")

  echo "Gepackt: $TARBALL ($(ls -lh "$DIST/$TARBALL" | awk '{print $5}'))"
done

if [ ${#INCLUDED_SUFFIXES[@]} -eq 0 ]; then
  echo "Fehler: Keine Binaries gefunden. Führe ./build.sh und ggf. ./build-linux.sh aus."
  exit 1
fi

# Formel-Snippet erzeugen
SNIPPET="$DIST/formula-snippet.rb"

# URL-Basis als Platzhalter — wird in der Formula ersetzt
URL_BASE="https://github.com/rolf-thomas/typo3-log-viewer/releases/download/v$VERSION"

cat > "$SNIPPET" <<EOF
# --- Snippet für Formula/typo3-log-viewer.rb (Version $VERSION) ---

version "$VERSION"

EOF

for i in "${!INCLUDED_SUFFIXES[@]}"; do
  SUFFIX="${INCLUDED_SUFFIXES[$i]}"
  HASH="${INCLUDED_HASHES[$i]}"
  TARBALL="$NAME-$VERSION-$SUFFIX.tar.gz"
  cat >> "$SNIPPET" <<EOF
# $SUFFIX
url "$URL_BASE/$TARBALL"
sha256 "$HASH"

EOF
done

echo ""
echo "=========================================="
echo "Release $VERSION fertig."
echo "=========================================="
echo "Tarballs: $DIST/"
echo "Checksums: $DIST/checksums.txt"
echo "Formel-Snippet: $DIST/formula-snippet.rb"
echo ""
echo "Nächste Schritte:"
echo "  1. gh release create v$VERSION --title \"v$VERSION\" dist/*.tar.gz"
echo "  2. Formula/$NAME.rb aktualisieren (Version + SHA256 aus formula-snippet.rb)"
echo "  3. Formel-Änderung ins Tap-Repo pushen"
