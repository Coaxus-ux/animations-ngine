#!/usr/bin/env bash
# Runs every project check locally. This replaces hosted CI, which the project does not
# use because it can generate costs (docs/engineering/DECISIONS.md D-006).
#
# Usage: tools/check.sh
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# Homebrew installs rustup keg-only; make cargo reachable without editing the shell profile.
if ! command -v cargo >/dev/null && [ -d /opt/homebrew/opt/rustup/bin ]; then
    export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
fi

step() {
    printf '\n== %s\n' "$1"
}

step "file size limits"
git ls-files -z | xargs -0 python3 tools/file_size_guard.py check

step "no agent signatures in commits"
if git rev-parse --verify --quiet HEAD >/dev/null; then
    python3 tools/commit_guard.py history HEAD
fi

step "format"
cargo fmt --all -- --check

step "clippy"
cargo clippy --workspace --all-targets --locked -- -D warnings

step "tests (includes golden hashes)"
cargo test --workspace --locked --no-fail-fast

step "tests with optimized debug assertions"
cargo test --workspace --profile release-assertions --locked --no-fail-fast

step "editor architecture (D-015, D-017): only px-cli touches windows, files and dialogs"
for crate in px-editor px-ui; do
    if cargo tree --locked -p "$crate" -e normal --prefix none | grep -qE '^(minifb|rfd) '; then
        echo "$crate must not depend on minifb or rfd" >&2
        exit 1
    fi
    if grep -rnE 'std::fs|std::net|minifb|rfd' "crates/$crate/src"; then
        echo "$crate must reach files and windows only through its ports" >&2
        exit 1
    fi
done

step "allocation-free rendering"
cargo test -p px-engine -p px-director --test no_alloc --locked -- --test-threads=1

step "frame times (every example and benchmark scene)"
cargo run --quiet --release --locked -p px-cli -- bench

printf '\nAll checks passed.\n'
