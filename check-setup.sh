#!/bin/bash
# Prüft alle Voraussetzungen für den Build von typo3-log-viewer
# und bietet an, fehlende Komponenten zu installieren.

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

ok()   { echo -e "${GREEN}  ✓${NC} $1"; }
warn() { echo -e "${YELLOW}  !${NC} $1"; }
fail() { echo -e "${RED}  ✗${NC} $1"; }

ask_install() {
  # $1 = Beschreibung, $2 = Installationsbefehl
  echo ""
  printf "    Jetzt installieren? [j/N] "
  read -r answer
  if [[ "$answer" =~ ^[jJyY]$ ]]; then
    echo "    Führe aus: $2"
    eval "$2"
    return 0
  fi
  return 1
}

echo ""
echo "========================================"
echo " typo3-log-viewer – Setup-Prüfung"
echo "========================================"
echo ""

MISSING=0

# ── 1. Homebrew ────────────────────────────────────────────────────────────────
echo "[ Homebrew ]"
if command -v brew >/dev/null 2>&1; then
  ok "Homebrew $(brew --version | head -1 | awk '{print $2}')"
else
  fail "Homebrew nicht gefunden"
  warn "Homebrew ist für mehrere weitere Installationen nötig."
  ask_install "Homebrew" \
    '/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"' \
    || MISSING=$((MISSING + 1))
fi
echo ""

# ── 2. Xcode Command Line Tools ────────────────────────────────────────────────
echo "[ Xcode Command Line Tools ]"
if xcode-select -p >/dev/null 2>&1; then
  ok "CLT installiert ($(xcode-select -p))"
else
  fail "Xcode Command Line Tools fehlen"
  ask_install "Xcode CLT" "xcode-select --install" \
    || MISSING=$((MISSING + 1))
fi
echo ""

# ── 3. Rust / rustup ──────────────────────────────────────────────────────────
echo "[ Rust ]"
REQUIRED="1.75"
REQ_MAJOR=$(echo "$REQUIRED" | cut -d. -f1)
REQ_MINOR=$(echo "$REQUIRED" | cut -d. -f2)

# Prüfen ob rustup im PATH ist (nicht nur irgendein rustc)
RUSTUP_OK=0
if command -v rustup >/dev/null 2>&1; then
  RUSTUP_OK=1
fi

# Prüfen ob rustc neu genug ist — bevorzuge ~/.cargo/bin/rustc
RUST_OK=0
RUSTC_BIN="${HOME}/.cargo/bin/rustc"
if [ -x "$RUSTC_BIN" ]; then
  RUST_VERSION=$("$RUSTC_BIN" --version | awk '{print $2}')
elif command -v rustc >/dev/null 2>&1; then
  RUST_VERSION=$(rustc --version | awk '{print $2}')
else
  RUST_VERSION=""
fi
if [ -n "$RUST_VERSION" ]; then
  RUST_MAJOR=$(echo "$RUST_VERSION" | cut -d. -f1)
  RUST_MINOR=$(echo "$RUST_VERSION" | cut -d. -f2)
  if [ "$RUST_MAJOR" -gt "$REQ_MAJOR" ] || \
     ([ "$RUST_MAJOR" -eq "$REQ_MAJOR" ] && [ "$RUST_MINOR" -ge "$REQ_MINOR" ]); then
    RUST_OK=1
  fi
fi

if [ "$RUSTUP_OK" -eq 1 ] && [ "$RUST_OK" -eq 1 ]; then
  ok "rustc $RUST_VERSION (>= $REQUIRED)"
  ok "rustup $(rustup --version 2>/dev/null | awk '{print $2}')"
elif [ "$RUSTUP_OK" -eq 0 ] && [ "$RUST_OK" -eq 1 ]; then
  # rustc vorhanden, aber nicht via rustup (z.B. Homebrew oder System-Paket)
  warn "rustc $RUST_VERSION gefunden, aber NICHT über rustup installiert"
  warn "System-Rust kann für Cross-Targets und cargo-zigbuild Probleme verursachen."
  warn "Empfehlung: rustup installieren und als primäres Rust verwenden:"
  warn "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  MISSING=$((MISSING + 1))
elif [ "$RUSTUP_OK" -eq 1 ] && [ "$RUST_OK" -eq 0 ]; then
  warn "rustc $RUST_VERSION gefunden, aber >= $REQUIRED benötigt"
  ask_install "Rust aktualisieren" "rustup update stable" \
    || MISSING=$((MISSING + 1))
else
  fail "Rust/rustup nicht gefunden"
  # Wenn bereits ein System-Rust vorhanden ist, braucht der Installer --yes
  # und RUSTUP_INIT_SKIP_PATH_CHECK=yes damit er nicht abbricht.
  if command -v rustc >/dev/null 2>&1; then
    warn "System-Rust unter $(command -v rustc) gefunden — Installer mit --yes aufrufen"
    RUSTUP_INSTALL="RUSTUP_INIT_SKIP_PATH_CHECK=yes curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --yes"
  else
    RUSTUP_INSTALL="curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  fi
  if ask_install "Rust (via rustup.rs)" "$RUSTUP_INSTALL"; then
    warn "Starte eine neue Shell-Sitzung (oder 'source ~/.cargo/env') und führe dieses Script erneut aus."
  else
    MISSING=$((MISSING + 1))
  fi
fi
echo ""

# ── 4. cargo (kommt mit Rust, aber prüfen) ────────────────────────────────────
echo "[ cargo ]"
# Bevorzuge das rustup-verwaltete cargo, falls vorhanden
CARGO_BIN="${HOME}/.cargo/bin/cargo"
if [ -x "$CARGO_BIN" ]; then
  CARGO_CMD="$CARGO_BIN"
elif command -v cargo >/dev/null 2>&1; then
  CARGO_CMD="cargo"
else
  CARGO_CMD=""
fi

if [ -n "$CARGO_CMD" ]; then
  CARGO_VERSION=$("$CARGO_CMD" --version | awk '{print $2}')
  CARGO_PATH=$(command -v "$CARGO_CMD" 2>/dev/null || echo "$CARGO_CMD")
  CARGO_MAJOR=$(echo "$CARGO_VERSION" | cut -d. -f1)
  CARGO_MINOR=$(echo "$CARGO_VERSION" | cut -d. -f2)
  if [ "$CARGO_MAJOR" -gt "$REQ_MAJOR" ] || \
     ([ "$CARGO_MAJOR" -eq "$REQ_MAJOR" ] && [ "$CARGO_MINOR" -ge "$REQ_MINOR" ]); then
    ok "cargo $CARGO_VERSION ($CARGO_CMD)"
    # Wenn ein älteres System-cargo im PATH vor dem rustup-cargo liegt, Hinweis geben
    SYSTEM_CARGO=$(command -v cargo 2>/dev/null || true)
    if [ "$SYSTEM_CARGO" != "$CARGO_BIN" ] && [ -x "$CARGO_BIN" ]; then
      warn "~/.cargo/bin steht nicht am Anfang des PATH"
      warn "Füge folgendes in ~/.zshrc oder ~/.bash_profile ein:"
      warn "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
    fi
  else
    fail "cargo $CARGO_VERSION ist zu alt (>= $REQUIRED benötigt)"
    if [ -x "$CARGO_BIN" ]; then
      warn "Neueres cargo unter $CARGO_BIN vorhanden, aber nicht im PATH"
      warn "Füge folgendes in ~/.zshrc oder ~/.bash_profile ein:"
      warn "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
    else
      warn "Bitte 'rustup update stable' ausführen"
    fi
    MISSING=$((MISSING + 1))
  fi
else
  fail "cargo nicht gefunden (Rust-Installation unvollständig?)"
  MISSING=$((MISSING + 1))
fi
echo ""

# ── 5. Linux-Cross-Build: Zig + cargo-zigbuild ────────────────────────────────
echo "[ Linux Cross-Build (optional, für ./build-linux.sh) ]"
ZIG_OK=0
ZIGBUILD_OK=0

if command -v zig >/dev/null 2>&1; then
  ok "zig $(zig version)"
  ZIG_OK=1
else
  warn "zig nicht installiert (nötig für Linux-Cross-Build ohne Docker)"
  ask_install "Zig via Homebrew" "brew install zig" && ZIG_OK=1 || true
fi

if command -v cargo-zigbuild >/dev/null 2>&1; then
  ok "cargo-zigbuild vorhanden"
  ZIGBUILD_OK=1
else
  warn "cargo-zigbuild nicht installiert"
  if [ "$ZIG_OK" -eq 1 ]; then
    # cargo-zigbuild >= 0.20 setzt Rust-Edition 2024 voraus (Cargo >= 1.85).
    # Version 0.19.x läuft noch mit älterem Cargo (Edition 2021).
    if [ "$RUST_OK" -eq 1 ] && [ "$RUSTUP_OK" -eq 1 ]; then
      CARGO_MINOR_NUM=$(cargo --version 2>/dev/null | awk '{print $2}' | cut -d. -f2)
      if [ "${CARGO_MINOR_NUM:-0}" -ge 85 ]; then
        ZIGBUILD_INSTALL="cargo install cargo-zigbuild"
      else
        ZIGBUILD_INSTALL="cargo install cargo-zigbuild@0.19.8"
        warn "Cargo < 1.85 erkannt — installiere cargo-zigbuild 0.19.8 (Edition-2021-kompatibel)"
      fi
    else
      ZIGBUILD_INSTALL="cargo install cargo-zigbuild@0.19.8"
    fi
    ask_install "cargo-zigbuild" "$ZIGBUILD_INSTALL" && ZIGBUILD_OK=1 || true
  else
    warn "cargo-zigbuild wird erst installiert, wenn zig verfügbar ist"
  fi
fi

if [ "$ZIG_OK" -eq 1 ] && [ "$ZIGBUILD_OK" -eq 1 ]; then
  # Rust-Targets prüfen
  INSTALLED_TARGETS=$(rustup target list --installed 2>/dev/null)
  for TARGET in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-unknown-linux-musl; do
    if echo "$INSTALLED_TARGETS" | grep -q "^$TARGET$"; then
      ok "rustup target: $TARGET"
    else
      warn "rustup target fehlt: $TARGET"
      ask_install "Target $TARGET hinzufügen" "rustup target add $TARGET" || true
    fi
  done
fi
echo ""

# ── 6. GitHub CLI (für Release-Workflow) ──────────────────────────────────────
echo "[ GitHub CLI (optional, für Release-Workflow) ]"
if command -v gh >/dev/null 2>&1; then
  ok "gh $(gh --version | head -1 | awk '{print $3}')"
  if gh auth status >/dev/null 2>&1; then
    ok "gh: eingeloggt ($(gh auth status 2>&1 | grep 'Logged in' | awk '{print $NF}'))"
  else
    warn "gh: nicht eingeloggt — 'gh auth login' ausführen"
  fi
else
  warn "GitHub CLI (gh) nicht installiert"
  ask_install "GitHub CLI via Homebrew" "brew install gh" || true
fi
echo ""

# ── Ergebnis ──────────────────────────────────────────────────────────────────
echo "========================================"
if [ "$MISSING" -eq 0 ]; then
  echo -e "${GREEN}Alle Pflicht-Voraussetzungen erfüllt.${NC}"
  echo ""
  echo "Nächste Schritte:"
  echo "  ./build.sh                  # macOS-Binary bauen"
  echo "  ./build-linux.sh            # Linux-Binaries bauen (falls Zig ok)"
  echo "  ./release.sh                # Release-Tarballs erzeugen"
else
  echo -e "${RED}$MISSING Pflicht-Komponente(n) fehlen noch.${NC}"
  echo "Script erneut ausführen nach der Installation."
fi
echo "========================================"
echo ""
