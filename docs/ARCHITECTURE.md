# Architecture (draft v1)

> Companion to [SPEC.md](SPEC.md). Describes the Rust workspace, the graph runtime, packs, schemas, errors, hot reload, threading and testing. The render stages themselves are in [RENDERING.md](RENDERING.md).

---

## 1. Principles

| Principle | Consequence |
|---|---|
| **Scene-agnostic core** | No crate below `packs` knows about rooms, trees, streets or cats. Content lives in packs, written in KDL |
| **One engine facade** | The editor and the CLI drive the same `engine` crate; they never call render crates directly |
| **Single source of truth for parameters** | Every node, prefab and body plan exposes one schema, used by validation, the editor inspector, `.d.ts` generation and reference docs |
| **Recover, don't crash** | Bad input produces diagnostics and placeholders; the preview keeps running |
| **Deterministic by construction** | Ordered maps, counter-based RNG, deterministic math, ordered reductions |

---

## 2. Workspace crates

The engine name is not decided yet. Crates get a short common prefix (shown here as `px-`) because a crate named `core` would collide with Rust's built-in `core` crate.

| Crate | Responsibility |
|---|---|
| `px-core` | Canvas buffers, ids, frame time, units, counter-based RNG, deterministic math, OKLab, palette and ramp types |
| `px-expr` | Expression language: parser, type checking, compilation to a small bytecode, evaluator; static/animated analysis |
| `px-format` | KDL loading into a typed document with source spans; value kinds (fixed, unit, range, expression, param, reference); **format-preserving edits** |
| `px-schema` | Parameter schema types, validation, generators for editor metadata, `.d.ts` and reference docs |
| `px-raster` | 2D primitives, 2D SDF shapes and operators, patterns, pseudo-normals |
| `px-model` | 3D-to-pixel: primitives and builders (extrude, lathe, loft), triangle rasterizer, SDF ray marcher, cameras (side, top-down, isometric, perspective), shadow maps |
| `px-filters` | Cleanup passes, dithering patterns and anchoring, outlines |
| `px-light` | Lights, falloff, 2D/3D shadows, twin tinting, reflections and refraction |
| `px-fx` | Water surface, caustics, light rays, fog, glow, weather and wind renderers |
| `px-sim` | Particles, rope/chain, feedback, stateful stepping and checkpoints |
| `px-rig` | Body plans and templates, skeletons, IK, poses, clips, channels, state machines, skinning data |
| `px-pieces` | Piece parsing, slots, anchors, variants, mirroring |
| `px-graph` | Graph runtime: node trait, instantiation, evaluation, caching, invalidation, parallel scheduling |
| `px-nodes` | Standard node library (generators, filters, light, compose, repeat, animation, simulation, rig, director, export nodes) |
| `px-packs` | Pack discovery and loading, registries, name resolution, std packs bundled into the binary |
| `px-director` | Scenes, shots, sequences, camera, transitions, timeline, loop solver |
| `px-script` | QuickJS bridge, TS type stripping, sandbox, value and event scripts |
| `px-export` | GIF, PNG, MP4/WebM via ffmpeg, sprite sheets, tilesets with autotile metadata, parallax layer sets |
| `px-engine` | Facade: open a project, resolve packs, build graphs, render frames, run exports, stream diagnostics, apply edits |
| `px-editor` | egui application (preview, inspector, timeline, graph, library, console) |
| `px-cli` | The `pxe` binary; commands in [CLI.md](CLI.md) |

### 2.1 Dependency graph

```
                          px-editor     px-cli
                               \         /
                                px-engine
                    ┌─────────────┼───────────────┬─────────────┐
               px-director    px-packs        px-export      px-script
                    │             │               │             │
                    └──────── px-nodes ───────────┘             │
                                  │                             │
     ┌───────────┬──────────┬─────┼──────┬──────────┬───────────┤
  px-raster  px-model  px-filters px-light px-fx  px-sim  px-rig  px-pieces
     └───────────┴──────────┴─────┼──────┴──────────┴───────────┘
                               px-graph
                                  │
                 px-schema ── px-format ── px-expr
                                  │
                               px-core
```

**Rules**
- Arrows only point down. Nothing depends on `px-editor` or `px-cli`.
- `px-graph` knows the node *trait* but no concrete node.
- Render crates (`raster`, `model`, `filters`, `light`, `fx`) never depend on each other's internals; they share types through `px-core`.
- `px-script` depends on `px-graph`, not on render crates. Scripts only see values and the object API.

---

## 3. Graph runtime

### 3.1 Node concept
Every node, whether a low-level operator, a Rust prefab or an expanded KDL prefab, presents the same contract:

| Part | Meaning |
|---|---|
| **Inputs** | Typed ports (canvas, mask, value, palette, ramp, path, skeleton/pose, particles, light, piece, model) |
| **Outputs** | Typed ports |
| **Params** | Schema-described parameters (§5) |
| **Time class** | `static`, `animated` or `stateful` |
| **Evaluate** | Given a context and input values, produce outputs |

The **context** gives a node its frame, derived seed, canvas size, `view`, camera, palette roles and read access to registries.

### 3.2 From KDL to graph
1. **Parse:** `px-format` turns files into typed documents with spans.
2. **Resolve names:** `px-packs` maps node names to registry entries (§4.3).
3. **Expand prefabs:** a KDL prefab is a subgraph, instantiated with its parameters bound. Expansion is recursive with a depth limit, and cycles between prefabs are an error.
4. **Wire:** child nodes, sockets, references (`@id.prop`) and parameters (`$name`) become edges.
5. **Validate:** schema checks, view support, type checks of ports and expressions.
6. **Classify:** compute the time class of every node (§3.3).

The graph is a DAG. Cross-frame edges exist only through explicit stateful nodes (feedback, simulations, event scripts).

### 3.3 Static vs. animated detection
- An expression is **animated** if it reads `f` or `t`, or references an animated value. Keyframed params, modulators and scripts are always animated.
- A node is **static** only if all its params and inputs are static and its own class is static.
- Static nodes are evaluated once and cached across frames. In B1 the whole room is static; in B6 every tile except water and windmills is static.

### 3.4 Evaluation and caching
- **Pull-based:** evaluation starts from the render output and walks dependencies in topological order.
- **Cache key:** `(node id, hash of resolved params and input hashes, time key)`.
  - `time key` is empty for static nodes.
  - For animated nodes it is the **effective frame**: animating "on 2s" makes frames 2k and 2k+1 share an entry.
- **Invalidation:** an edit marks the changed node and everything downstream as dirty. Clean nodes keep their cache.
- **Memory:** static entries persist; animated entries live in an LRU with a size budget (default 512 MB in the editor, unbounded streaming in export).
- **Hashing:** a fast non-cryptographic 64/128-bit hash over inputs.

### 3.5 Stateful nodes and checkpoints
`state(f) = step(state(f − 1), inputs(f))`
- A **checkpoint** (a serialized state snapshot) is stored every `checkpoint-every` frames (default 30).
- **Seeking** to frame `f` loads the nearest checkpoint ≤ `f` and steps forward.
- **Warm-up:** loop scenes simulate `warmup` frames before frame 0, so particles, rain and smoke are already in motion when the loop starts.
- **Loop closure:** for stateful content in perfect loops, the director chooses one of two methods. Either the simulation is periodic (emitters with integer periods aligned to the loop length), or the last `blend` frames are dithered toward the first frames. See [ANIMATION.md](ANIMATION.md).
- Editing a stateful node discards its checkpoints from that frame onward.

### 3.6 Parallelism
| Level | How |
|---|---|
| Between nodes | Independent branches of the DAG run in parallel on a work-stealing pool |
| Inside a node | Row- or tile-parallel loops for raster, lighting and effects |
| Between frames (export) | Stateful parts are stepped sequentially first (cheap), then frames render in parallel |

All parallel writes go to disjoint regions, and reductions run in a fixed order, so results never depend on thread count.

---

## 4. Packs and registries

### 4.1 What a pack is
```
nature/
  pack.kdl          name, version, depends-on, provides, supported views
  prefabs/          oak-tree.kdl, hills.kdl, grass.kdl …
  body-plans/       deer.kdl, songbird.kdl …
  pieces/           antlers.kdl, beak-small.kdl …
  palettes/         forest-dusk.kdl …
  scripts/          flock.ts …
```

```kdl
pack "nature" version="0.1.0" {
    depends-on "core-kit" version="^0.1"
    views "side" "perspective" "top-down" "isometric"
}
```

### 4.2 Sources and resolution order
1. **Project** (`./prefabs`, `./body-plans`…)
2. **User packs** (a configured packs folder, shared between projects)
3. **Standard packs** (`core-kit`, `interior`, `nature`, `urban`, `water`, `creatures`), bundled inside the binary but overridable on disk

A name found earlier in this list shadows later ones, and shadowing produces an info diagnostic.

### 4.3 Names
- Full names are `pack/name` (`nature/oak-tree`).
- Short names (`oak-tree`) work when unique across loaded packs. Ambiguity is an error that lists the candidates.
- Rust-native prefabs register into the same registries as KDL prefabs and look identical to users.

### 4.4 Registries

| Registry | Entries | Source |
|---|---|---|
| Nodes | Low-level node types | Rust (`px-nodes`) |
| Prefabs | Semantic objects | Rust and KDL |
| Body plans | Creatures and machines, plus templates (`biped`, `quadruped`, `bird`, `fish`, `serpent`, `multi-leg`, `machine`) | KDL |
| Pieces | Bitmaps with slots | KDL |
| Palettes | Ramps, aliases, twins, light colors | KDL, Lospec imports |
| Clips | Pose clips and procedural clip presets | KDL and Rust |

Each entry carries its schema, its source span (for "go to definition" in the editor) and its supported views.

### 4.5 Versioning
- Packs use semver.
- Projects declare `format 1` in `project.kdl`. Format changes ship with an automatic migration (`pxe migrate`) that edits files in place, preserving formatting.

---

## 5. Schemas

One schema per node, prefab and body plan:

| Field | Example |
|---|---|
| name, type | `levels`, int |
| default, range, unit | `4`, `1..8`, none |
| enum values | `"sparse" "normal" "dense"` |
| animatable | yes |
| group | "Layout" (inspector section) |
| doc | One-line description |
| views | Views where the param applies |

**Consumers**

| Consumer | Uses the schema to… |
|---|---|
| Validator (`px-format`) | Type-check values, report unknown params and ranges, suggest names |
| Editor inspector | Pick widgets: slider (range), dropdown (enum), ramp picker (material), socket picker, seed dice, ◆ animate toggle |
| `.d.ts` generator | Type script APIs: object handles by id, typed properties, state names |
| Docs generator | Produce the reference pages (the long-term source of [NODES.md](NODES.md)) |
| Diagnostics | Explain errors with the param's doc line |

---

## 6. Diagnostics and error handling

### 6.1 Diagnostic shape
Each diagnostic has a severity (`error`, `warning`, `info`), a stable code (for example `E0102 unknown-node`), a source span, a message and optional suggestions (fix-its such as "did you mean `shelf`?").

### 6.2 Recovery rules

| Situation | Behavior |
|---|---|
| Unknown or invalid param | Warning; default value used |
| Unknown node or prefab | Error; a **placeholder** is rendered (magenta/black checker in a reserved debug ramp), so the rest of the scene still shows |
| Unsupported view | Error; placeholder plus a suggestion for an alternative prefab |
| Expression error | Error; the param falls back to its default |
| Script exception | Error with a TS stack trace; the node keeps its last good output |
| Node panic | Caught per node in the editor; that node becomes a placeholder |
| Unreadable `project.kdl` | Fatal (nothing to render) |

The CLI prints diagnostics with source excerpts. The editor shows them in the console and as squiggles in the inspector. `pxe check` runs validation only.

---

## 7. Hot reload

```
file saved ──► watcher (debounced ~50 ms)
           ──► re-parse changed file (format-preserving)
           ──► diff typed document vs. previous
           ──► map changes to graph nodes
           ──► invalidate those nodes + dependents (and checkpoints after the edit frame)
           ──► re-render the current frame
           ──► push frame + diagnostics to the editor
```

| Change | What gets invalidated |
|---|---|
| A param value | That node and its dependents |
| A prefab definition | Every instance of the prefab |
| A palette | **Only the resolve stage** (indexed pipeline): palette edits are near-instant |
| A script | Its module is recompiled; its node's caches and checkpoints are reset |
| A piece | Every object that uses it |

**Edits from the editor** rewrite the KDL file in place, keeping comments and formatting. The watcher recognizes the engine's own writes by content hash and doesn't reload twice.

---

## 8. Threading model

| Thread | Job |
|---|---|
| UI | egui, input, drawing the preview texture |
| Render coordinator | Owns the graph, applies edits, schedules frames, cancels stale renders when the user scrubs |
| Worker pool | Node evaluation and per-row work (sized to CPU cores) |
| Script | One QuickJS runtime (single-threaded). Value-script calls for a frame are batched; event scripts step in order |
| Watcher | File system events |
| Encoder (export) | Receives finished frames in order through a bounded channel and encodes or pipes to ffmpeg |

The coordinator is the only owner of mutable graph state. Workers get immutable snapshots, so no locking happens in the hot path.

---

## 9. Testing strategy

| Test type | What it checks |
|---|---|
| **Unit** | Each crate's rules: rasterizer edge cases, every cleanup pass, light quantization, expression evaluation. Fixtures are small text grids in the piece format |
| **Golden images** | Each benchmark B1–B6 renders fixed frames. **Indexed buffers** (not RGB) are compared exactly with committed goldens; failures write a visual diff. `pxe bench --update` updates goldens on purpose |
| **Determinism** | The same project rendered serially and in parallel, with different thread counts, on macOS, Linux and Windows, must produce identical hashes |
| **Loops** | For loop scenes, frame `N` equals frame `0` |
| **Property-based** | Cleanup passes are idempotent; rasterizers never write out of bounds; palette operations never leave the palette; format edits preserve untouched bytes |
| **Pack smoke tests** | Every std prefab and body plan renders in each declared view, at default params and at range extremes, without errors |
| **Format round-trip** | Parse → edit → write leaves everything except the edit byte-identical |
| **Performance** | Benchmarks per stage against the budgets in RENDERING §15; CI fails on regressions above a threshold |
| **Script sandbox** | Forbidden APIs are unavailable; `Math.random` is seeded; scripts can't break determinism |

---

## 10. Planned third-party crates

| Crate | Purpose | Why |
|---|---|---|
| `kdl` | KDL v2 parsing | Official Rust implementation, format-preserving documents |
| `miette` | Diagnostics | Span-based error reports with source excerpts in the CLI |
| `thiserror` | Error types | Concise library error definitions |
| `strsim` | "Did you mean" suggestions | Edit distance |
| `rquickjs` | QuickJS bindings | Small, embeddable, modern JS |
| `oxc` (transformer) | Strip TypeScript types | Fast and written in Rust |
| `serde`, `serde_json` | Metadata | Sprite sheet/tileset metadata, editor state |
| `rayon` | Parallelism | Work-stealing pool, scoped tasks |
| `crossbeam-channel` | Channels | Ordered export pipeline, coordinator messages |
| `notify` + `notify-debouncer-mini` | File watching | Cross-platform hot reload |
| `eframe` / `egui` | Editor UI | Immediate-mode UI, easy texture preview |
| `egui-snarl` | Node graph widget | Node editor for egui |
| `glam` | Vector math | Small, fast vector/matrix types for the 3D path |
| `libm` | Deterministic math | The same transcendental results on every OS |
| `indexmap` | Ordered maps | Deterministic iteration |
| `xxhash-rust` | Cache keys | Fast 64/128-bit hashing |
| `image`, `png`, `gif` | Image I/O and encoding | Indexed PNG and GIF with an exact palette |
| `include_dir` | Bundle std packs | Std packs ship inside the binary |
| `clap` | CLI | Argument parsing |
| `criterion` | Benchmarks | Performance budgets |
| `proptest` | Property tests | Cleanup and raster invariants |
| `insta` | Snapshot tests | Diagnostics, schemas and `.d.ts` output |

ffmpeg is used as an **external process**, not linked, which keeps licensing and builds simple. Noise functions, RNG, sprite sheet packing and the rasterizers are written in-house, because they must be deterministic and pixel-art specific.
