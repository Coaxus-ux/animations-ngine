#!/usr/bin/env bash
set -euo pipefail

if [ $# -lt 3 ] || [ $# -gt 4 ]; then
    echo "usage: tools/export_mp4.sh <sequence.kdl> <score.py> <out.mp4> [scale, default 2]" >&2
    echo "macOS only: frames come from pxe render, the score from tools/chiptune.py, AAC from afconvert and H.264 from AVFoundation." >&2
    exit 2
fi

sequence=$1
score=$2
out=$3
scale=${4:-2}
root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

if ! command -v cargo >/dev/null && [ -d /opt/homebrew/opt/rustup/bin ]; then
    export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
fi

cargo build --quiet --release --locked -p px-cli --manifest-path "$root/Cargo.toml"
pxe="$root/target/release/pxe"
frames=$(python3 "$root/tools/sequence_cues.py" "$sequence" --frames)
fps=$(python3 -c 'import sys; sys.path.insert(0, sys.argv[1]); import sequence_cues; print(sequence_cues.read(sys.argv[2]).fps)' "$root/tools" "$sequence")

mkdir "$work/frames"
seq 0 $((frames - 1)) | xargs -P "$(sysctl -n hw.ncpu)" -I{} \
    sh -c 'exec "$1" render "$2" --scale "$3" --frame "$4" --out "$(printf "%s/f%05d.png" "$5" "$4")" >/dev/null' \
    _ "$pxe" "$sequence" "$scale" {} "$work/frames"

python3 "$root/tools/chiptune.py" "$sequence" "$score" "$work/score.wav"
afconvert -f m4af -d aac -b 192000 "$work/score.wav" "$work/score.m4a"

swiftc -O "$root/tools/mp4_mux.swift" -o "$work/mp4_mux"
mkdir -p "$(dirname "$out")"
"$work/mp4_mux" "$work/frames" "$fps" "$work/score.m4a" "$out"
