# Contributing

Thanks for your interest. The rules below keep the engine general, deterministic and easy to review.

## Setup

1. Install rustup; it fetches the toolchain pinned in [`rust-toolchain.toml`](rust-toolchain.toml). With Homebrew's keg-only rustup, add it to the path first:
   ```bash
   export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
   ```
2. Render something to check the build:
   ```bash
   cargo pxe export examples/hello.kdl --out out/hello.gif
   ```

## Before every push

```bash
tools/check.sh
```

It runs the file size and commit guards, `cargo fmt`, clippy with warnings as errors, the tests (golden hashes included) in debug and with optimized assertions, and the editor architecture check. There is no hosted CI: nothing runs on GitHub Actions or any service that can bill ([D-006](docs/engineering/DECISIONS.md)), so this script is the gate.

## Rules

- **Follow [RUST-STANDARDS.md](docs/engineering/RUST-STANDARDS.md).** Exceptions go through [DECISIONS.md](docs/engineering/DECISIONS.md).
- **SOLID and DRY.** One responsibility per crate, module, type and function; one source of truth for every rule, limit and constant.
- **Generic core.** No engine code may name a benchmark, scene or prefab ([BENCHMARKS.md](docs/BENCHMARKS.md)). Scene-specific content belongs in KDL.
- **Deterministic.** Integer frame time, seeded counter-based randomness, `libm` for transcendental math, no hash-map order in outputs. A change that alters rendered pixels must update the golden hashes on purpose and say why.
- **Small files.** Code ≤ 400 lines, docs ≤ 1000 lines, other files ≤ 1 MB (`tools/file_size_guard.py`). Split a module before it grows past the limit.
- **Few comments.** Code explains itself; comments only for a non-obvious *why*, an invariant types can't enforce, `SAFETY`, or a spec reference ([D-007](docs/engineering/DECISIONS.md)).
- **English everywhere:** code, comments, docs, errors and commit messages.
- **Commits are authored by people.** No AI agent `Co-Authored-By` trailers, "Generated with" footers or agent identities ([D-005](docs/engineering/DECISIONS.md)); `tools/commit_guard.py` checks the history.

## Changing the format

The KDL format is documented in [FORMAT.md](docs/FORMAT.md). When the loader learns something new, update [appendix A](docs/FORMAT.md#appendix-a-what-the-loader-accepts-today) in the same change and add or extend an example in [`examples/`](examples).
