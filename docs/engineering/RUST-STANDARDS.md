# Rust Engineering Standards

> Normative standard for all Rust in this repository. It condenses the project's adopted "Rust Engineering Best Practices" (ownership, API design, errors, `debug_assert!`, allocation discipline, testing, linting, performance, determinism) and applies it to this engine. **MUST / SHOULD / MAY** have their RFC meaning; deviating from a MUST needs a written decision in [docs/engineering/DECISIONS.md](DECISIONS.md).

Correctness, safety, determinism and maintainability come before cleverness. Allocation behavior and boundedness are designed into APIs up front; micro-optimization is driven by measurement.

---

## 0. Always: SOLID and DRY

- **Single responsibility:** one crate, module, type or function = one reason to change. Files stay small (code ≤ 400 lines, enforced by `tools/file_size_guard.py`).
- **Open/closed:** add behavior by adding a variant, node or prefab, not by editing unrelated match arms across the codebase. Closed sets use enums; open sets use traits.
- **Liskov:** every trait implementation honors the trait's documented contract (errors, bounds, allocation profile).
- **Interface segregation:** small traits with one purpose; callers depend only on what they use.
- **Dependency inversion:** core crates define traits and types; adapters (I/O, CLI, formats) depend on the core, never the reverse.
- **DRY:** one source of truth for every rule, constant, limit and policy. Duplicate code is extracted only when it shares *meaning*, not just shape.

## 1. Core principles

1. **Make invalid states unrepresentable:** newtypes for ids, units and offsets; enums instead of flag booleans or sentinels; `Option` for absence; `Result` for recoverable failure; type-state when lifecycle matters; private fields by default.
2. **Explicit over hidden:** signatures show allocation, capacity limits, errors, ownership transfer, mutation, blocking, determinism, panics and thread safety. No undocumented global state, implicit init, hidden retries or environment-dependent defaults.
3. **Bounded runtime work:** explicit bounds on iterations, candidates, recursion depth, queue length, output size, temporary storage, retries and concurrency. Hitting a bound returns a typed error or status.
4. **Rich construction, lean execution:** the authoring layer (KDL parsing, validation, compilation) may allocate; it produces compact validated data that a bounded, allocation-free runtime borrows.
5. **Keep a clear reference implementation** behind every optimized path, usable as oracle for differential tests.

## 2. Memory and allocation contracts

Every crate and performance-sensitive public operation declares one profile in its crate docs:

| Profile | Contract |
|---|---|
| **Strict heapless** | No heap storage. `#![no_std]`, no `alloc`. |
| **Allocation-free steady state** | May allocate during initialization; the named operation and all transitive callees never allocate afterwards. |
| **Allocation-conscious** | Allocates only at explicit, justified, bounded boundaries. |

Project assignment (M0):

| Crate | Profile |
|---|---|
| `px-core` | Strict heapless (`no_std`) |
| `px-raster` | Strict heapless (`no_std`) |
| `px-expr` | Compile: allocation-conscious. Evaluate: strict heapless |
| `px-engine` | `render_frame`: allocation-free steady state |
| `px-format`, `px-export`, `px-cli` | Allocation-conscious (authoring and I/O adapters) |

Rules:
- Be precise about the claim ("this function", "this exercised path", "all valid inputs", "this subsystem").
- Runtime APIs take `&T`, `&[T]`, `&str`, caller-owned output and scratch buffers, fixed-capacity storage, iterators; they return lengths, ranges and statuses instead of owned collections. Prefer `fn parse(input: &[u8], out: &mut [Rec]) -> Result<usize, E>` over `fn parse(String) -> Vec<Rec>`.
- Never move unbounded storage onto the stack. Large buffers are caller-supplied, static, arena-backed or chunked. Recursion only with a documented structural bound.
- Capacity exhaustion is an explicit error (`CapacityError::Full { capacity }`); never spill to the heap, drop, overwrite or retry silently.
- In strict or allocation-free paths: no `Vec`, `String`, `Box`, `Rc`, `Arc`, maps, `vec!`, `format!`, `to_vec`, `to_string`, `collect` into owned collections, growth via `push`/`extend`/`reserve`, boxed traits or futures, `Cow::into_owned`, lazy heap init, unaudited logging/tracing/serialization, or unknown callbacks and dependencies.
- Formatting writes into a caller-supplied `fmt::Write` sink; no owned diagnostic strings in runtime code.
- Sorting: `sort_unstable*` / `select_nth_unstable*` where order ties are irrelevant; when determinism matters, define total order and tie-breaking explicitly.
- Core errors are small `Copy` enums; `Display` writes to the formatter. Rich context is added only in adapter crates (`anyhow` only in binaries).
- Strict crates start with `#![no_std]`, `#![forbid(unsafe_code)]`, `#![deny(clippy::disallowed_macros, clippy::disallowed_types)]` (see `clippy.toml`).

## 3. Ownership and values

- Borrow by default: `&str` not `&String`, `&[T]` not `&Vec<T>`, `&Path` not `&PathBuf`. Clone only when independent ownership is required, never to silence the borrow checker.
- Pass small `Copy` values (scalars, newtypes, small structs) by value.
- Naming: `into_*` consumes, `to_*` may allocate, `as_*` is a cheap view. Document allocating conversions.
- Prefer contiguous data and index-based relations over pointer graphs.

## 4. Option, Result, panics and arithmetic

- `Option` for absence, `Result` for failure; no magic values.
- No `unwrap()`/`expect()` in production paths (lint-enforced). Tests may use them sparingly.
- Typed, focused library errors. `?` for plain propagation; explicit `map_err` when translating.
- Panic only on genuine programmer defects; document under `# Panics`.
- Checked arithmetic (`checked_add`, `checked_mul`) for offsets, lengths and capacities at trust boundaries. Domain arithmetic documents checked, saturating or wrapping semantics. Release builds keep `overflow-checks = true`.

## 5. `debug_assert!`

Use for internal invariants already established by types or release-active checks. Never for validating external input, never as an `unsafe` precondition, never with side effects, never allocating in the condition or message. Run tests in the `release-assertions` profile as well.

## 6. Iterators and loops

Iterators are lazy; allocation happens on `collect`. Return borrowed iterators instead of collecting. Use a plain `for` loop when it makes bounds, early exits or multiple accumulators clearer. Avoid intermediate collections; review closure captures in hot paths.

## 7. Functions and modules

- Single-purpose functions; extract only meaningful boundaries.
- Hot paths keep bounds, memory access, temporaries, allocation and error exits visible.
- No boolean blindness: enums and policy structs instead of `fn f(strict: bool, retry: bool)`.
- Explicit imports, no wildcard imports outside tests and preludes.
- Narrow visibility: private, then `pub(crate)`, then `pub`.

## 8. Generics, dispatch and concurrency

- Static dispatch (generics, `impl Trait`) in hot and strict paths; enum dispatch for closed sets; `dyn Trait` only for real runtime polymorphism outside hot paths.
- Allocation lives at an explicit outer boundary.
- Bounded concurrency: max workers, queue capacities, per-worker scratch, backpressure, deterministic merge order. Never assign canonical ids by completion order.

## 9. Type-state

Use for a few meaningful lifecycle states (e.g. unvalidated → validated). Debug assertions may re-check what construction established; they are never the only validation.

## 10. Pointers, sharing and unsafe

- References and slices first; `Box`/`Rc`/`Arc` only for real ownership needs. `Arc<Mutex<T>>` is a smell, not a default.
- `#![forbid(unsafe_code)]` in every crate. Any exception: isolated module, safe API, `// SAFETY:` on every block, release-active preconditions, Miri where applicable, an approved decision record. Test-only allocator counters are the one standing exception (integration tests only).

## 11. Performance

Measure before and after (release mode, representative workloads, allocation counts, memory). Order of work: algorithms → bounded candidates → avoid work → contiguous layout → fewer bytes → fewer allocations and copies → cache locality → branches → SIMD. Reuse prepared state and worker-local scratch.

## 12. Lints and formatting

- Every change: `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- Lints are configured once in the workspace `Cargo.toml`; crates opt in with `[lints] workspace = true`.
- Fix warnings; suppress only locally with `#[expect(lint, reason = "…")]`.
- `clippy.toml` lists disallowed heap types and macros, denied in strict crates. It is a guardrail, not a proof.

## 13. Tests

- Tests are executable documentation; names state condition and expectation.
- Layers: unit, integration, doc tests, property tests, fuzz tests (parsers), differential tests (optimized vs reference).
- Boundaries: empty, one, exact capacity, capacity − 1, max capacity, malformed offsets, overflow edges, ties, repeated warm runs, every error variant.
- Allocation-free claims are tested with a counting global allocator in a dedicated integration test, run with `--test-threads=1`.
- Golden tests compare **indexed-buffer hashes**, never RGB output.
- Snapshot tests stay small and reviewable.

## 14. Comments and documentation

**Code explains itself.** If code needs a comment to be understood, rename, extract or retype it instead ([DECISIONS.md](DECISIONS.md) D-007).

- **No comments that restate code.** No doc comments on items whose name and signature already say what they do.
- **A comment is allowed only for what code cannot express:** a non-obvious *why*, an invariant that types cannot enforce, a `// SAFETY:` proof, a reference to a spec rule, or instructions in a test file (how to bless golden files, how to run).
- **Contracts that matter go where callers see them:** a short doc line on a public function only when its allocation, panic or determinism behavior is not obvious from its signature.
- **Crate roots** state their memory profile in one line.
- **CLI help text** is written as doc comments because clap turns it into `--help`.
- **TODOs** reference an issue: `// TODO(#12): …`.

## 15. Dependencies

Minimal dependency graph for core crates; `default-features = false` and explicit features; `Cargo.lock` committed; audit build scripts and proc macros. Core crates never depend on CLI, async runtimes, telemetry, filesystem or rich error frameworks.

## 16. Determinism

Defined input order, stable ids, total ordering with tie-breaking, no hash-map iteration order in outputs, seeded counter-based randomness, integer frame time, fixed-width integers at boundaries, deterministic merge order under parallelism. Floating-point math uses pure-Rust `libm` (no platform libm) so results are identical on every OS.

## 17. Cargo and check baseline

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --profile release-assertions --locked
cargo test -p px-engine --test no_alloc -- --test-threads=1
python3 tools/file_size_guard.py check $(git ls-files)
```

All of the above run locally through `tools/check.sh` (no hosted CI, [DECISIONS.md](DECISIONS.md) D-006). Cross-platform hash checks use a free local Linux container.

## 18. Review checklist

- **API:** invalid states unrepresentable? typed errors for caller-controlled failures? checked arithmetic? ownership visible? minimal visibility?
- **Allocation:** profile declared? hot paths allocation-free? caller-owned buffers? finite documented capacities? hidden allocation paths reviewed?
- **Assertions:** real invariants only, side-effect free, not safety proofs, non-allocating?
- **Performance and determinism:** measured? contiguous data? no needless clones? canonical output independent of scheduling and map order?
- **Safety:** `unsafe` forbidden or isolated with `SAFETY:`? bounded queues and retries?
- **Tests and comments:** success, capacity, malformed and error paths tested? allocation behavior tested? no comment that restates code?
- **SOLID/DRY:** one responsibility per unit? single source of truth for each rule?

## 19. Anti-patterns

Cloning to silence the borrow checker · owned parameters where a borrow suffices · returning `Vec` from every query · `unwrap` on external input · `debug_assert!` as validation or safety proof · side effects in assertions · `format!` in strict error paths · calling a preallocated `Vec` "heapless" · spill-capable small vectors · allocation hidden in logging · boxing in hot paths without evidence · unbounded channels, retries, recursion or tasks · ids by completion order · abstractions that hide bounds · unsafe before proving safe code insufficient · workspace-wide lint suppression · allocation-freedom claimed from one input.
