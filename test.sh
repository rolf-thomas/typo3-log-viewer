#!/bin/bash
set -e

echo "Führe Tests aus..."
cargo test "$@"

echo ""
echo "Fertig!"
