#!/usr/bin/env bash
# Downloads the Fusion Pixel 12px monospaced BDF release and unpacks the
# zh_TW face into source/. The archive is pinned by version and sha256.
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=2026.09.25
ZIP=fusion-pixel-font-12px-monospaced-bdf-v$VERSION.zip
SHA256=d75f5262f108757edb0f47ee8e3d2dfdfbecfd94558faf5ffb0c06dc5866fb3b
URL=https://github.com/TakWolf/fusion-pixel-font/releases/download/$VERSION/$ZIP

mkdir -p source
if [ ! -f "source/$ZIP" ]; then
  curl -fL -o "source/$ZIP.part" "$URL"
  mv "source/$ZIP.part" "source/$ZIP"
fi
echo "$SHA256  source/$ZIP" | sha256sum -c -
unzip -o -q -j "source/$ZIP" fusion-pixel-12px-monospaced-zh_tw.bdf -d source
echo "source/fusion-pixel-12px-monospaced-zh_tw.bdf"
