#!/bin/bash
# Kompletter Release-Flow: baut alle Binaries, erzeugt Tarballs,
# patcht die Formel und legt den GitHub-Release an.
#
# Voraussetzungen:
#   - Version in Cargo.toml ist die Ziel-Version (siehe ./bump-version.sh)
#   - gh ist authentifiziert (gh auth status)
#   - Zig + cargo-zigbuild für Linux-Cross-Builds
#
# Optional:
#   TAP_PATH=/pfad/zu/homebrew-tools   # wenn gesetzt: Formel ins Tap kopieren + pushen
#
# Nutzung: ./release-all.sh

set -e

ROOT=$(cd "$(dirname "$0")" && pwd)
cd "$ROOT"

REPO="rolf-thomas/typo3-log-viewer"

banner() {
  echo ""
  echo "=========================================="
  echo " $1"
  echo "=========================================="
}

fail() {
  echo ""
  echo "FEHLER: $1" >&2
  exit 1
}

# --- Voraussetzungen prüfen ---

banner "Voraussetzungen prüfen"

[ -f Cargo.toml ] || fail "Cargo.toml nicht gefunden."
[ -x ./build.sh ] || fail "build.sh fehlt oder nicht ausführbar."
[ -x ./build-linux.sh ] || fail "build-linux.sh fehlt oder nicht ausführbar."
[ -x ./release.sh ] || fail "release.sh fehlt oder nicht ausführbar."
[ -x ./update-formula.sh ] || fail "update-formula.sh fehlt oder nicht ausführbar."

command -v gh >/dev/null 2>&1 || fail "gh CLI nicht installiert (brew install gh)."
gh auth status >/dev/null 2>&1 || fail "gh nicht authentifiziert (gh auth login)."

VERSION=$(awk -F'"' '/^version/ { print $2; exit }' Cargo.toml)
[ -n "$VERSION" ] || fail "Konnte version aus Cargo.toml nicht lesen."

TAG="v$VERSION"
echo "Ziel-Version: $VERSION (Tag: $TAG)"

# Prüfen, dass der Release noch nicht existiert
if gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1; then
  fail "Release $TAG existiert bereits auf $REPO. Erst löschen oder Version bumpen."
fi

# --- macOS arm64 Build (signiert) ---

banner "macOS arm64 Build"
./build.sh

# --- macOS x86_64 Build (signiert) ---

banner "macOS x86_64 Build"
cargo build --release --target x86_64-apple-darwin
codesign --force --deep -s - target/x86_64-apple-darwin/release/typo3-log-viewer
echo "Signiert."

# --- Linux Builds ---

banner "Linux Builds (musl + arm64)"
./build-linux.sh x86_64-unknown-linux-musl aarch64-unknown-linux-gnu

# --- Release-Tarballs erzeugen ---

banner "Release-Tarballs + Checksums"
./release.sh

# --- Formel patchen ---

banner "Formel patchen"
./update-formula.sh

# --- GitHub Release ---

banner "GitHub Release anlegen"
gh release create "$TAG" \
  --repo "$REPO" \
  --title "$TAG" \
  --generate-notes \
  dist/*.tar.gz

RELEASE_URL=$(gh release view "$TAG" --repo "$REPO" --json url -q .url)
echo "Release: $RELEASE_URL"

# --- Optional: Tap-Repo-Push ---

if [ -n "$TAP_PATH" ]; then
  banner "Tap-Repo aktualisieren"
  if [ ! -d "$TAP_PATH" ]; then
    echo "Warnung: TAP_PATH='$TAP_PATH' existiert nicht — Tap-Push übersprungen."
  else
    cp Formula/typo3-log-viewer.rb "$TAP_PATH/Formula/"
    (
      cd "$TAP_PATH"
      git add Formula/typo3-log-viewer.rb
      git commit -m "Bump typo3-log-viewer to $VERSION"
      git push
    )
    echo "Tap-Repo gepusht."
  fi
fi

# --- Fertig ---

banner "Release $TAG abgeschlossen"

echo "Assets:"
gh release view "$TAG" --repo "$REPO" --json assets -q '.assets[].name' | sed 's/^/  /'

echo ""
if [ -z "$TAP_PATH" ]; then
  echo "Nächster Schritt: Formel ins Tap-Repo kopieren und pushen:"
  echo "  TAP=~/Code/homebrew-tools"
  echo "  cp Formula/typo3-log-viewer.rb \"\$TAP/Formula/\""
  echo "  cd \"\$TAP\" && git add . && git commit -m \"Bump to $VERSION\" && git push"
  echo ""
  echo "Oder setze TAP_PATH und führe release-all.sh nochmal aus:"
  echo "  TAP_PATH=~/Code/homebrew-tools ./release-all.sh   # (nach Release delete)"
fi
