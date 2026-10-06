#!/usr/bin/env bash
# Builds four locales, each from its own Fusion Pixel face:
#   build/TentenTC-Big5.ttf     zh_tw    charsets/big5-common.txt
#   build/TentenSC-GB2312.ttf   zh_hans  charsets/gb2312.txt
#   build/TentenJP-JIS.ttf      ja       charsets/jisx0208.txt
#   build/TentenKR-KSX.ttf      ko       charsets/ksx1001.txt
# plus build/Tenten<L>-Full.ttf with every glyph of the face, and a .woff2
# of each subset font when fontTools with brotli is importable or uvx can
# fetch it. WOFF2_FULL=1 also compresses the full fonts, which takes
# fontTools over ten minutes each.
set -euo pipefail
cd "$(dirname "$0")/.."

LOCALES=(
  "TC zh_tw Big5 big5-common"
  "SC zh_hans GB2312 gb2312"
  "JP ja JIS jisx0208"
  "KR ko KSX ksx1001"
)

[ -f source/fusion-pixel-12px-monospaced-ko.bdf ] || scripts/fetch-source.sh >/dev/null

cargo build --release --quiet --bin tenten
mkdir -p build
woff=()
for spec in "${LOCALES[@]}"; do
  read -r loc face set charset <<<"$spec"
  bdf=source/fusion-pixel-12px-monospaced-$face.bdf
  echo "Tenten$loc-$set: $(target/release/tenten --locale "$loc" "$bdf" "build/Tenten$loc-$set.ttf" "charsets/$charset.txt")"
  echo "Tenten$loc-Full: $(target/release/tenten --locale "$loc" "$bdf" "build/Tenten$loc-Full.ttf")"
  woff+=("build/Tenten$loc-$set.ttf")
  [ "${WOFF2_FULL:-0}" = 1 ] && woff+=("build/Tenten$loc-Full.ttf")
done

if python3 -c 'import fontTools, brotli' 2>/dev/null; then
  py=(python3)
elif command -v uvx >/dev/null && uvx --quiet --from fonttools --with brotli python -c '' 2>/dev/null; then
  py=(uvx --quiet --from fonttools --with brotli python)
else
  echo "fontTools with brotli not found: skipped .woff2" >&2
  exit 0
fi
for f in "${woff[@]}"; do
  "${py[@]}" -c 'import sys; from fontTools.ttLib import TTFont; t = TTFont(sys.argv[1]); t.flavor = "woff2"; t.save(sys.argv[2])' "$f" "${f%.ttf}.woff2" &
done
wait
ls -l build/*.woff2
