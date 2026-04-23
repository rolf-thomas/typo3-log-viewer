#!/bin/bash
set -e

BINARY_PATH="target/release/typo3-log-viewer"

echo "Baue Release-Version..."
cargo build --release

echo "Signiere Binary (ad-hoc)..."
codesign --force --deep -s - "$BINARY_PATH"

echo "Entferne Quarantäne-Attribut..."
xattr -cr "$BINARY_PATH" 2>/dev/null || true

echo ""
echo "Fertig! Binary: $BINARY_PATH"
ls -lh "$BINARY_PATH"
