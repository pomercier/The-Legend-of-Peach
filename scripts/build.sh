#!/usr/bin/env bash
set -euo pipefail

# The Legend of Peach: A Mario to the Mushroom Kingdom
# Deterministic local ROM builder (requires user-provided legal backups).
# Original input ROMs are read-only and never modified.

usage() {
  cat <<'EOF'
Usage:
  ./scripts/build.sh <zelda_alttp_fr.sfc> <super_mario_world_eu.sfc> [output.sfc]
  ./scripts/build.sh --verify-only <zelda_alttp_fr.sfc> <super_mario_world_eu.sfc>

Defaults (when no positional paths are passed):
  Zelda base ROM : roms/zelda_alttp.sfc
  SMW donor ROM  : roms/super_mario_world.sfc
  Output ROM     : build/legend_of_peach_mario_kingdom.sfc

See docs/ROM_REQUIREMENTS.md for exact No-Intro regions and SHA-256 hashes.
EOF
}

VERIFY_ONLY=0
if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ "${1:-}" == "--verify-only" ]]; then
  VERIFY_ONLY=1
  shift
fi

ZELDA_ROM="${1:-roms/zelda_alttp.sfc}"
SMW_ROM="${2:-roms/super_mario_world.sfc}"
OUT_ROM="${3:-build/legend_of_peach_mario_kingdom.sfc}"

if [[ ! -f "$ZELDA_ROM" ]]; then
  echo "Error: Zelda base ROM not found at '$ZELDA_ROM'." >&2
  echo "See docs/ROM_REQUIREMENTS.md for required original ROM details." >&2
  exit 1
fi

if [[ ! -f "$SMW_ROM" ]]; then
  echo "Error: Super Mario World donor ROM not found at '$SMW_ROM'." >&2
  echo "See docs/ROM_REQUIREMENTS.md for required original ROM details." >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

echo "Building snes-cli (release)..."
cargo build --release --bin snes-cli

if [[ "$VERIFY_ONLY" -eq 1 ]]; then
  exec ./target/release/snes-cli --build-peach \
    --zelda "$ZELDA_ROM" \
    --smw "$SMW_ROM" \
    --verify-only
else
  mkdir -p "$(dirname "$OUT_ROM")"
  exec ./target/release/snes-cli --build-peach \
    --zelda "$ZELDA_ROM" \
    --smw "$SMW_ROM" \
    --out "$OUT_ROM"
fi
