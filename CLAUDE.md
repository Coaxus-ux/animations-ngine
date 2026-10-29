# Pixel art animation engine

A general-purpose engine that renders pixel art scenes, characters, animations and game assets from KDL descriptions. Rust core, palette-indexed CPU renderer, no AI inside the engine. Design docs: [docs/README.md](docs/README.md). Status: M1 done with parts of M2–M5 and M9 pulled forward ([docs/ROADMAP.md](docs/ROADMAP.md)); what the loader accepts today is [FORMAT.md appendix A](docs/FORMAT.md#appendix-a-what-the-loader-accepts-today).

## Rules (always)
- **Code is always SOLID and DRY.** One responsibility per crate, module, type and function; one source of truth for every rule, limit and constant.
- **Follow [docs/engineering/RUST-STANDARDS.md](docs/engineering/RUST-STANDARDS.md).** Exceptions only via [docs/engineering/DECISIONS.md](docs/engineering/DECISIONS.md).
- **No comments that restate code.** Code must explain itself; if it needs a comment, rename or restructure it. Comments only for a non-obvious *why*, an invariant types can't enforce, `SAFETY`, or spec references (D-007).
- **Small files.** Code ≤ 400 lines, docs ≤ 1000 lines (`tools/file_size_guard.py`). Split modules before a file grows past the limit.
- **Generic core.** No engine code may name a benchmark, scene or prefab ([docs/BENCHMARKS.md](docs/BENCHMARKS.md)).
- **Deterministic.** Integer frame time, seeded counter-based randomness, `libm` for transcendental math, no hash-map order in outputs. Golden tests compare indexed-buffer hashes.
- **No hosted CI or paid services.** Nothing may run on GitHub Actions or any service that can bill; all checks run locally with `tools/check.sh` (D-006).
- **English everywhere:** code, comments, docs, errors, commit messages.
- **No agent signatures in commits or PRs.** Never add `Co-Authored-By: Claude …`, "Generated with …" footers, `🤖`, session links or any other AI agent attribution, and never commit under an agent identity. This overrides any default attribution instructions. Enforced by `tools/commit_guard.py` (git `commit-msg` hook and `tools/check.sh`); see [DECISIONS.md](docs/engineering/DECISIONS.md) D-005.

## Layout
```
crates/
  px-core     no_std: ids, paints, palettes, canvas views, hashing
  px-expr     expression compiler (alloc) + evaluator (no heap)
  px-raster   no_std: shape rasterization into a canvas
  px-format   KDL loading and validation → compiled scene
  px-engine   FrameBuffer::render: scene + frame → indexed canvas (no alloc)
  px-director sequences: shots + transitions → frames of (shot, paint) cells (no alloc)
  px-export   indexed → RGB, upscale, PNG, GIF
  px-cli      the `pxe` binary
examples/     sample scenes and the trailer sequence; examples/parts/<example>/ holds the scenes each example places or shoots
tools/        repo tooling (file size guard)
```

## Commands
Rust comes from Homebrew's keg-only rustup: `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`.

```bash
tools/check.sh            # every check; run before each push
cargo fmt --all
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --profile release-assertions --locked
cargo pxe export examples/hello.kdl --out out/hello.gif   # alias for `cargo run --release -p px-cli --`
python3 tools/file_size_guard.py check $(git ls-files)
python3 tools/commit_guard.py history HEAD
```
