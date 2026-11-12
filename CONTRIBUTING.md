# Contributing

Thanks for your interest. This guide covers everything needed to land a change: setting up, where code goes, the checks, and the step-by-step recipes for the most common kinds of contribution.

The short version:

1. Branch off `main`.
2. Make one focused change, following the [ground rules](#ground-rules).
3. Run `tools/check.sh` until it prints `All checks passed.`
4. Commit in plain English with no AI agent attribution, push, and open a pull request.

## Contents

- [Ground rules](#ground-rules)
- [Setup](#setup)
- [Where things live](#where-things-live)
- [Workflow](#workflow)
- [The checks](#the-checks)
- [Writing Rust here](#writing-rust-here)
- [Recipe: add an example scene](#recipe-add-an-example-scene)
- [Recipe: change how something renders](#recipe-change-how-something-renders)
- [Recipe: extend the KDL format](#recipe-extend-the-kdl-format)
- [Recipe: change the editor](#recipe-change-the-editor)
- [Recipe: add to the shared kit](#recipe-add-to-the-shared-kit)
- [Documentation](#documentation)
- [Engineering decisions](#engineering-decisions)
- [Commits and pull requests](#commits-and-pull-requests)
- [Review checklist](#review-checklist)
- [Troubleshooting](#troubleshooting)

## Ground rules

These hold for every change. Most are enforced by a script, so a violation fails `tools/check.sh`.

| Rule | What it means | Enforced by |
|---|---|---|
| SOLID and DRY | One responsibility per crate, module, type and function. One source of truth for every rule, limit and constant | Review |
| Rust standards | Follow [RUST-STANDARDS.md](docs/engineering/RUST-STANDARDS.md). An exception needs an entry in [DECISIONS.md](docs/engineering/DECISIONS.md) | clippy, review |
| No comments | No `//`, `///` or `//!` in Rust and no `#` lines in the Python and shell tools, shebangs aside (D-024). If code needs a comment, rename or restructure it. CLI help text goes in clap attributes | `tools/comment_guard.py` |
| Small files | Code ≤ 400 lines, docs ≤ 1000 lines, other files ≤ 1 MB (D-001). Split a module before it reaches the limit | `tools/file_size_guard.py` |
| Generic core | No engine code may name a benchmark, scene or prefab ([BENCHMARKS.md](docs/BENCHMARKS.md)). Scene-specific content belongs in KDL | Review |
| Deterministic | Integer frame time, seeded counter-based randomness, `libm` for transcendental math (D-003), no hash-map order in outputs | Golden hashes |
| No hosted CI | Nothing runs on GitHub Actions or any service that can bill (D-006). Every check runs on your machine | Review |
| English everywhere | Code, docs, error messages, commit messages and pull requests | Review |
| Commits by people | No AI agent `Co-Authored-By` trailers, "Generated with" footers, `🤖`, session links or agent identities (D-005) | `tools/commit_guard.py` |

KDL files are not covered by the no-comments rule: scene files open with a short `//` description, as every file in [`examples/`](examples) does.

## Setup

You need git, Python 3 and rustup. The toolchain is pinned in [`rust-toolchain.toml`](rust-toolchain.toml), and rustup installs it, with clippy and rustfmt, on the first build.

```bash
brew install rustup
```

Homebrew's rustup is keg-only, so put it on the path (add this line to your shell profile):

```bash
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
```

On other systems install rustup from <https://rustup.rs>.

Clone and render something to prove the build works:

```bash
git clone https://github.com/Coaxus-ux/animations-ngine.git
```

```bash
cargo pxe export examples/hello.kdl --out out/hello.gif
```

`cargo pxe` is an alias for `cargo run --release -p px-cli --` (see [`.cargo/config.toml`](.cargo/config.toml)), so the CLI always runs optimized. Everything it writes goes to `out/`, which git ignores.

Install the commit-message hook once per clone. It rejects a commit that carries an agent signature before it is created, instead of when `tools/check.sh` later scans the history:

```bash
printf '#!/bin/sh\nexec python3 tools/commit_guard.py msg "$1"\n' > .git/hooks/commit-msg && chmod +x .git/hooks/commit-msg
```

MP4 export (`tools/export_mp4.sh`) is macOS only: it uses `afconvert`, `swiftc` and AVFoundation. Nothing else in the repo depends on the operating system.

## Where things live

```
crates/
  px-core      no_std: ids, paints, palettes, canvas views, hashing
  px-expr      expression compiler (alloc) and evaluator (no heap)
  px-raster    no_std: shape rasterization into a canvas
  px-format    KDL loading and validation into a compiled scene
  px-engine    FrameBuffer::render: scene + frame into an indexed canvas (no alloc)
  px-director  sequences: shots and transitions into frames (no alloc)
  px-export    indexed to RGB, upscale, PNG, GIF, sprite sheets, Aseprite JSON
  px-ui        pixel-art UI toolkit drawn by the engine (no window, no files)
  px-editor    the editor: domain, tools, views, app (no window, no files)
  px-cli       the `pxe` binary; the only crate that touches windows, files and dialogs
examples/      sample scenes; examples/parts/<example>/ holds the scenes an example places
examples/kit/  motifs and parts shared by several scenes
benchmarks/    the benchmark scenes that keep the engine general
docs/          design documents; start at docs/README.md
tools/         the checks, and MP4 export with a synthesized chiptune score
```

Pick the crate by what the change is about:

| The change is about | Crate |
|---|---|
| A new id, paint, palette operation or hash | `px-core` |
| A new expression function or operator | `px-expr` |
| How a shape turns into pixels | `px-raster` |
| A new node, property, shorthand rule or load error | `px-format` |
| Lighting, fog, reflections, particles, how a frame is assembled | `px-engine` |
| Shots and transitions | `px-director` |
| A new output file format | `px-export` |
| A widget, theme or icon | `px-ui` |
| Editor behavior: documents, commands, tools, panels | `px-editor` |
| A command-line flag, a window, a file read or a dialog | `px-cli` |

If a change seems to need two of these at once, split it along that line. The `no_std` and allocation-free crates stay that way: `px-engine` and `px-director` render without allocating, and a test proves it (see [the checks](#the-checks)).

## Workflow

1. **Branch off `main`** with a short kebab-case name for the topic, such as `editor-trailer` or `scene-shorthand`.
2. **Keep the change focused.** One feature or fix per branch. A refactor that the feature needs goes in its own commit before it.
3. **Iterate with the fast checks** while you work:
   ```bash
   cargo fmt --all
   ```
   ```bash
   cargo clippy --workspace --all-targets --locked -- -D warnings
   ```
   ```bash
   cargo test -p px-format --locked
   ```
   Test only the crate you are touching until the change settles.
4. **Run the full gate** before pushing:
   ```bash
   tools/check.sh
   ```
5. **Commit, push and open a pull request** as described in [commits and pull requests](#commits-and-pull-requests).

## The checks

`tools/check.sh` is the only gate, because there is no hosted CI (D-006). It stops at the first failing step. A full run takes roughly 20 to 30 minutes on a laptop, most of it in the two golden-hash runs, so use the single commands below while iterating and keep the full run for the end.

| Step | What it guards | Run it alone |
|---|---|---|
| File size limits | D-001 limits on every tracked file | `python3 tools/file_size_guard.py check $(git ls-files)` |
| No comments | D-024 | `git ls-files -z \| xargs -0 python3 tools/comment_guard.py` |
| No agent signatures | D-005, over the whole history | `python3 tools/commit_guard.py history HEAD` |
| Format | rustfmt | `cargo fmt --all -- --check` |
| Clippy | Workspace lints, warnings are errors | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| Tests | Unit and integration tests, golden hashes included | `cargo test --workspace --locked` |
| Tests with optimized assertions | The same tests in release mode with debug assertions and overflow checks | `cargo test --workspace --profile release-assertions --locked` |
| Editor architecture | `px-editor` and `px-ui` never depend on `minifb` or `rfd` or mention `std::fs` or `std::net` (D-015, D-017) | Part of `tools/check.sh` |
| Allocation-free rendering | `px-engine` and `px-director` render without touching the heap | `cargo test -p px-engine -p px-director --test no_alloc --locked -- --test-threads=1` |
| Frame times | Every example and benchmark renders each frame inside its budget | `cargo pxe bench` |

Two of these need a little more explanation.

**Golden hashes.** `crates/px-cli/tests/golden.rs` renders every frame of every file in `examples/` and `benchmarks/` and compares a hash of the indexed buffer (never RGB) with `crates/px-cli/tests/golden/<name>.txt`. It also fails if a frame leaves its palette or if a looping scene has a seam between its last and first frame. It takes several minutes, so run it alone when a change touches rendering:

```bash
cargo test -p px-cli --test golden --locked
```

**Frame times.** `pxe bench` fails with exit code 4 when the slowest frame of any scene is over 33.3 ms. A sequence made for export can set its own limit with `frame-budget` (D-026). To time one file:

```bash
cargo pxe bench examples/hello.kdl
```

## Writing Rust here

[RUST-STANDARDS.md](docs/engineering/RUST-STANDARDS.md) is the full standard. The points that most often come up in review:

- **Names carry the meaning.** A newtype says what a number is, a `Result` says a call can fail, `#[must_use]` says a value matters. With no comments allowed, a function that needs explaining gets renamed or split.
- **Lints are fixed, not silenced.** When one must be silenced, do it locally with `#[expect(lint, reason = "…")]`. Lints are configured once in the workspace `Cargo.toml`.
- **No `unsafe`** in any library crate (D-002). The single exception is the counting allocator in the `no_alloc` test targets.
- **No platform math.** Use `libm` functions, never `f64::sin` and friends (D-003), or hashes differ between operating systems.
- **No hash-map order in outputs.** Anything written to a file or hashed is iterated in a defined order.
- **Hot paths do not allocate.** Buffers are owned by the caller and sized up front; `clippy.toml` denies heap types in the strict crates.
- **Errors for caller mistakes, panics never.** No `unwrap` on external input; a bad scene file produces a diagnostic with a file, line and column.
- **Tests state condition and expectation in their name**, and cover the edges: empty, one, exact capacity, malformed input and every error variant.
- **Dependencies stay minimal.** A new one needs a reason; core crates take none that pull in files, async runtimes or rich error frameworks. `Cargo.lock` is committed and every command runs with `--locked`.

## Recipe: add an example scene

Examples are the engine's showcase and its regression suite at once: each one is rendered frame by frame by the golden test and timed by the bench.

1. **Create `examples/<name>.kdl`** with a one- or two-line `//` description at the top. The file must hold exactly one scene or sequence that loads by default.
2. **Put what it places in `examples/parts/<name>/`:** its palette file, and any scene it places with `use-scene` or shoots with `shot`. Reuse before writing: [SHORTHAND.md](docs/SHORTHAND.md) covers `motif`, `part`, `repeat` and `scatter`, and [`examples/kit/`](examples/kit) has shared stars and town pieces.
3. **Preview while you write.** The window reloads on every save, including saves to used files:
   ```bash
   cargo pxe watch examples/<name>.kdl
   ```
4. **Look at specific frames**, enlarged, when a moment needs checking:
   ```bash
   cargo pxe render examples/<name>.kdl --frame 70 --scale 2
   ```
5. **Export it** to watch the whole thing:
   ```bash
   cargo pxe export examples/<name>.kdl --scale 3
   ```
6. **Check its timing.** A frame over 33.3 ms fails the gate:
   ```bash
   cargo pxe bench examples/<name>.kdl
   ```
7. **Record its golden hashes.** This writes `crates/px-cli/tests/golden/<name>.txt`:
   ```bash
   PX_BLESS=1 cargo test -p px-cli --test golden --locked
   ```
   Confirm that `git status` shows only your new `.txt` as changed under `tests/golden/`. Any other changed hash file means the change altered an existing scene.
8. **Add a row** to the examples table in [README.md](README.md).
9. **Run `tools/check.sh`.**

Things that keep an example healthy:

- A looping scene (`loop=#true`) must be seamless: give particles a `life` that divides the duration, and let placed looping scenes fit a whole number of times.
- Only use what [FORMAT.md appendix A](docs/FORMAT.md#appendix-a-what-the-loader-accepts-today) lists. Anything else is reported as an unknown node or property.
- A placed scene must use the host's palette or one the host's palette `include`s.
- Keep every file under the size limits; a big scene splits naturally into parts.

## Recipe: change how something renders

Any change to pixels shows up as changed golden hashes, and that is the point: the test tells you exactly which scenes a change touched.

1. Make the change and run the golden test. It names the first scene whose hashes differ.
2. Decide whether each difference is intended. Render the affected frames before and after and compare them:
   ```bash
   cargo pxe render examples/koi-pond.kdl --frame 12 --scale 2 --out out/after.png
   ```
3. If a difference is not intended, fix the change. A refactor must leave every hash untouched.
4. If it is intended, re-record the hashes with `PX_BLESS=1` (see the previous recipe) and review the diff: it must contain exactly the scenes you expect.
5. Say in the commit message and the pull request **why the pixels changed** and which scenes were affected. A hash update with no explanation will be sent back.
6. Run the allocation test and the bench as well, since rendering changes are the ones that break them.

## Recipe: extend the KDL format

1. **Read the design first.** [FORMAT.md](docs/FORMAT.md) describes the full format; its appendix A lists the part the loader accepts today. A new node or property should already be in the design, or the design changes in the same pull request.
2. **Parse and validate it in `px-format`.** Unknown names must keep producing the "Did you mean …?" hint, and every new way to get it wrong needs a diagnostic that names the file and position.
3. **Shorthand is a load-time pass** (`crates/px-format/src/expand/`, D-027): it rewrites nodes before the loader sees them, so the engine stays unchanged. A new shorthand rule goes there, not in the engine.
4. **Render it** in `px-engine` or `px-raster` only if it is a genuinely new capability. Keep it generic: the engine learns about a shape or an effect, never about a subject.
5. **Test it:** the happy path, each error, and the boundaries.
6. **Document it in the same change:** the table and prose in appendix A, and [SHORTHAND.md](docs/SHORTHAND.md) or [PIXELS.md](docs/PIXELS.md) when it belongs there.
7. **Show it:** add it to an existing example or write a new one, and record the hashes.

The format is not stable yet, so a breaking change is allowed, but every example and benchmark must be migrated in the same pull request.

## Recipe: change the editor

The editor follows ports and adapters (D-015):

- `px-ui` draws widgets into an indexed canvas with the engine itself (D-012). It knows nothing about the editor.
- `px-editor` holds the domain (documents, commands, history), the tools, the views and the app. Views only emit `Intent`s; the app dispatches them. It never opens a window or a file: it asks through its ports and returns pixels.
- `px-cli`'s `pxe edit` is the single adapter: the window (minifb), the file system and the native file dialogs (rfd, D-017).

So a new panel or tool goes in `px-editor`, a new widget in `px-ui`, and anything that needs the operating system becomes a request that `px-cli` answers. `tools/check.sh` fails if `px-editor` or `px-ui` depend on `minifb` or `rfd`, or mention `std::fs` or `std::net`.

Two more rules:

- **The KDL text is the source of truth** (D-014). Every edit splices the text and reloads it; bytes the edit did not touch never change. Each edit is a `Command`, so it lands in the one undo history shared by the visual and code views.
- **Editor state never goes into scene files** (D-023). Preferences live in `~/.config/px/editor.kdl`.

Try the result on a real file, and read [EDITOR.md](docs/EDITOR.md) for what `pxe edit` does today:

```bash
cargo pxe edit examples/pixel-room.kdl
```

## Recipe: add to the shared kit

[`examples/kit/`](examples/kit) holds motifs and parts that scenes share (D-030, D-031).

- Add a **motif** only when it replaces definitions in at least two scenes.
- Add a **part** when it is a piece a new scene would start from, at the level it belongs to: an atom draws shapes, a molecule places atoms, an organism places molecules.
- Name everything `kit-…`, because motif and part names are global across a load.
- Take `paint` by name, since every palette names its ramps its own way.
- List the addition in the kit table in [SHORTHAND.md](docs/SHORTHAND.md), and show it in [`examples/kit-street.kdl`](examples/kit-street.kdl) if it is a town piece.

## Documentation

- [docs/README.md](docs/README.md) gives the reading order. Update the document that owns a topic rather than describing the same thing in two places.
- A document describes the design; a dated "Today" note or appendix A says what is built. When you build something, update that note in the same change.
- Docs are limited to 1000 lines. Split a document by topic before it gets there, and link the pieces from `docs/README.md`.
- Keep the layout blocks in [README.md](README.md) and [CLAUDE.md](CLAUDE.md) current when a crate or a top-level directory is added.
- Write plainly: short sentences, the concrete term, an example for anything a reader must type.

## Engineering decisions

[DECISIONS.md](docs/engineering/DECISIONS.md) records every approved exception to the standards and every project-wide engineering choice. Add an entry, numbered after the last one and dated, when a change:

- needs an exception to [RUST-STANDARDS.md](docs/engineering/RUST-STANDARDS.md),
- adds a crate, a dependency with real weight, or a new boundary between crates,
- replaces something a design document says, or
- sets a rule that future changes must follow.

An entry states the decision and the reason in a few sentences. Refer to it by number (`D-024`) from the docs and from scripts that enforce it.

## Commits and pull requests

**Commit messages** follow the existing history:

- One line in the imperative, starting with a capital letter, with no trailing period and no type prefix: `Add scene shorthand: bare coordinates, repeat, motif and scatter`.
- Say what the change does for someone using the engine, not which files moved.
- Add a body when the reason is not obvious, and always when golden hashes changed.
- No AI agent attribution of any kind, and never commit under an agent identity (D-005). This applies when an assistant helped write the change, too: the person who commits is the author.

**Before opening a pull request:**

- `tools/check.sh` passed on the exact commit you are pushing.
- The branch is up to date with `main`.
- The change is one topic. Unrelated fixes go in their own pull request.

**In the pull request description:**

- What changed and why, in a few sentences.
- That `tools/check.sh` passed, since no hosted CI will show it.
- Which golden hashes changed and why, if any.
- A GIF or PNG for anything visual. Export it to `out/` and attach it; do not commit rendered output outside `docs/media/`.
- Any new decision entry.

## Review checklist

Reviewers work through [RUST-STANDARDS.md §18](docs/engineering/RUST-STANDARDS.md); going through it yourself first saves a round trip.

- **API:** are invalid states unrepresentable? Do caller mistakes produce typed errors? Is visibility minimal?
- **Allocation:** do hot paths stay allocation-free, with caller-owned buffers and documented capacities?
- **Determinism:** is the output independent of scheduling, map order and platform?
- **Tests:** are success, capacity, malformed input and each error covered?
- **SOLID and DRY:** does each unit have one responsibility, and each rule one source of truth?
- **Generality:** does the engine still know nothing about any particular scene?
- **Docs:** do appendix A and the relevant document match what the code now does?

## Troubleshooting

| Symptom | Cause and fix |
|---|---|
| `cargo: command not found` | Homebrew's rustup is keg-only: `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"` |
| `<name>: golden hashes changed; rerun with PX_BLESS=1 if intended` with an empty right-hand side | A new example has no hash file yet. Record it with `PX_BLESS=1` |
| The same message for a scene you did not touch | Your change altered its pixels. Follow [change how something renders](#recipe-change-how-something-renders) |
| `the loop has a seam` | A looping scene's last frame does not lead into its first. Check particle `life` and placed scenes' durations |
| `frame N leaves the palette` | A pixel was drawn with a paint that is not in the scene's palette, usually a placed scene whose palette the host does not include |
| `pxe bench` exits with code 4 | A frame is over budget. Reduce the work in that scene, or for an export-only sequence set `frame-budget` |
| The comment guard reports a line | Remove the comment and make the code say it: rename, extract a function or introduce a type |
| The file size guard reports a file | Split it by responsibility. An already oversized file may be edited only if it does not grow |
| The commit guard rejects a message | Remove the agent trailer or footer. For an existing commit, amend it before pushing |
| `px-editor must reach files and windows only through its ports` | Move the file or window access into `px-cli` and expose it to the editor as a request |
| An unknown node or property error on something the docs describe | It is designed but not built. Appendix A lists what loads today |
