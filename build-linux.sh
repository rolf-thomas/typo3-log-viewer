#!/bin/bash
# Cross-Compilation für Debian/Ubuntu/Linux
#
# Verwendet automatisch eines von zwei Backends:
#   1. cargo-zigbuild  (falls `zig` installiert ist — kein Docker nötig)
#   2. cross           (Docker Desktop muss laufen)
#
# Ausgabe:
#   target/<target>/release/typo3-log-viewer
#
# Standard-Targets: x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu
# Eigene Targets als Argumente übergeben, z.B.:
#   ./build-linux.sh x86_64-unknown-linux-musl

set -e

DEFAULT_TARGETS=("x86_64-unknown-linux-gnu" "aarch64-unknown-linux-gnu")

if [ $# -eq 0 ]; then
  TARGETS=("${DEFAULT_TARGETS[@]}")
else
  TARGETS=("$@")
fi

# Backend wählen
BACKEND=""
if command -v zig >/dev/null 2>&1 && command -v cargo-zigbuild >/dev/null 2>&1; then
  BACKEND="zigbuild"
  echo "Backend: cargo-zigbuild (Zig als Cross-Linker, kein Docker nötig)"
elif command -v cross >/dev/null 2>&1; then
  if ! docker info >/dev/null 2>&1; then
    echo "Fehler: 'cross' installiert, aber Docker-Daemon läuft nicht."
    echo ""
    echo "Entweder Docker Desktop starten oder cargo-zigbuild verwenden:"
    echo "  brew install zig"
    echo "  cargo install cargo-zigbuild"
    exit 1
  fi
  BACKEND="cross"
  echo "Backend: cross (Docker)"
else
  echo "Fehler: Weder cargo-zigbuild noch cross gefunden."
  echo ""
  echo "Empfohlen (Docker-frei):"
  echo "  brew install zig"
  echo "  cargo install cargo-zigbuild"
  echo ""
  echo "Alternative mit Docker:"
  echo "  cargo install cross --git https://github.com/cross-rs/cross"
  echo "  (Docker Desktop muss installiert und gestartet sein)"
  exit 1
fi

# Build-Funktion
build_target() {
  local target="$1"
  echo ""
  echo "=========================================="
  echo "Baue für Target: $target"
  echo "=========================================="

  # Target-Toolchain installieren (idempotent)
  rustup target add "$target" >/dev/null 2>&1 || true

  case "$BACKEND" in
    zigbuild)
      cargo zigbuild --release --target "$target"
      ;;
    cross)
      cross build --release --target "$target"
      ;;
  esac

  local binary="target/$target/release/typo3-log-viewer"
  if [ -f "$binary" ]; then
    echo ""
    echo "Fertig: $binary"
    ls -lh "$binary"
    file "$binary" 2>/dev/null || true
  else
    echo "Warnung: Erwartete Binary nicht gefunden: $binary"
  fi
}

for TARGET in "${TARGETS[@]}"; do
  build_target "$TARGET"
done

echo ""
echo "=========================================="
echo "Alle Builds abgeschlossen."
echo "=========================================="
