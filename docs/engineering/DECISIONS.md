# Engineering Decisions

> Approved exceptions and project-wide engineering choices referenced by [RUST-STANDARDS.md](RUST-STANDARDS.md). Newest last.

| # | Date | Decision | Why |
|---|---|---|---|
| D-001 | 2026-10-28 | File size limits: code ≤ 400 lines / 64 KB, docs ≤ 1000 lines / 128 KB, data ≤ 512 KB, other files ≤ 1 MB; `Cargo.lock` exempt. Enforced by `tools/file_size_guard.py` (Claude hook, git pre-commit, `tools/check.sh`). Already-oversized files may be edited only if they don't grow. | Keeps modules single-purpose and reviewable |
| D-002 | 2026-10-28 | `unsafe` is forbidden in every library crate. The only exception is the counting global allocator in allocation-test integration targets (`tests/no_alloc.rs`), which needs `unsafe impl GlobalAlloc`. | Standard §10 and §13.5 |
| D-003 | 2026-10-28 | Floating-point transcendental functions use the pure-Rust `libm` crate, never `f64::sin` & co. | Platform libm results differ across OSes; golden hashes must match on macOS, Linux and Windows |
| D-004 | 2026-10-28 | M0 adds `px-expr` and `px-engine` to the five crates listed in ROADMAP so that expression evaluation and frame rendering each have one owner (SRP). | Keeps `px-format` a pure loader and `px-raster` a pure rasterizer |
| D-005 | 2026-10-28 | Commits and PRs carry no AI agent signature: no agent `Co-Authored-By`/`Signed-off-by`/`Assisted-by` trailers, no "Generated with …" footers, no `🤖`, no agent session links, no agent author or committer identity. Enforced by `tools/commit_guard.py` as a git `commit-msg` hook and by `tools/check.sh` over the whole history. | Project owner's rule: commits are authored by people |
| D-006 | 2026-10-28 | **No hosted CI.** No GitHub Actions or any other service that can generate costs. Every check runs locally with `tools/check.sh` before each push; wherever the docs say "CI", read `tools/check.sh`. Cross-platform hash checks run in a free local Linux container when needed; Windows stays unverified until a free option exists. | Project owner's rule: no costs |
| D-007 | 2026-10-28 | **Self-explanatory code, minimal comments.** No comments or doc comments that restate code; the `missing_docs` lint is off and clippy's `missing_errors_doc` / `missing_panics_doc` are allowed. Comments remain only for non-obvious *why*, invariants, `SAFETY`, spec references, test instructions and CLI help text. | Project owner's rule: "if code needs a comment to be understood, it is wrong" |
