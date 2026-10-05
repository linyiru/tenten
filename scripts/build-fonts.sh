#!/usr/bin/env bash
# Builds build/TentenTC-Big5.ttf (the Big5 common set) and
# build/TentenTC-Full.ttf (every glyph in the source), plus a .woff2 of the
# Big5 font when fontTools with brotli is importable. WOFF2_FULL=1 also
# compresses the full font, which takes fontTools over ten minutes.
set -euo pipefail
cd "$(dirname "$0")/.."

BDF=source/fusion-pixel-12px-monospaced-zh_tw.bdf
[ -f "$BDF" ] || scripts/fetch-source.sh >/dev/null

cargo build --release --quiet
mkdir -p build
target/release/tenten "$BDF" build/TentenTC-Big5.ttf charsets/big5-common.txt
target/release/tenten "$BDF" build/TentenTC-Full.ttf

if python3 -c 'import fontTools, brotli' 2>/dev/null; then
  fonts=(build/TentenTC-Big5.ttf)
  [ "${WOFF2_FULL:-0}" = 1 ] && fonts+=(build/TentenTC-Full.ttf)
  for f in "${fonts[@]}"; do
    python3 -c 'import sys; from fontTools.ttLib import TTFont; t = TTFont(sys.argv[1]); t.flavor = "woff2"; t.save(sys.argv[2])' "$f" "${f%.ttf}.woff2"
  done
else
  echo "fontTools with brotli not found: skipped .woff2" >&2
fi
