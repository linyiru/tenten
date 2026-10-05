#!/usr/bin/env bash
# Renders build/film/tenten.mp4 (1920x1080, 60 fps) and its score,
# build/film/tenten.wav. Arguments pass through to `film`, for example
#   scripts/build-film.sh --still 2,9.5,15   # review stills, no video
#   scripts/build-film.sh --size 960x540     # a fast preview
# Needs ffmpeg (with libx264) on PATH for the video; stills do not.
set -euo pipefail
cd "$(dirname "$0")/.."

BDF=source/fusion-pixel-12px-monospaced-zh_tw.bdf
[ -f "$BDF" ] || scripts/fetch-source.sh >/dev/null

cargo build --release --quiet --bin film
target/release/film --bdf "$BDF" "$@"
