#!/usr/bin/env bash
# Renders the README screenshot docs/assets/settings.png headlessly (egui_kittest + wgpu),
# with a backend that supports every action, as on Linux.
set -euo pipefail
cd "$(dirname "$0")/.."
KEYPAD_SHOTS="$PWD/docs/assets" cargo test -q -p keypad-ui --test shots
echo "wrote docs/assets/settings.png"
