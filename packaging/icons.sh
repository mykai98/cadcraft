#!/usr/bin/env bash
# Regenerate original ZnunitAICAD icons with Python 3 and Pillow on any platform.
# Outputs are committed; normal builds and packaging need neither tool.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec python3 "$ROOT/packaging/icons.py"
