# Roadmap (draft v2)

> Companion to [SPEC.md](SPEC.md). Replaces SPEC §11. Benchmarks are defined in [BENCHMARKS.md](BENCHMARKS.md); nodes in [NODES.md](NODES.md).

## 0. Principles

1. **Generic first.** Every milestone adds engine capabilities that serve **at least two benchmarks**.
2. **No benchmark-specific engine code.** If a benchmark needs something, it becomes a generic node, or content in a pack written in KDL. A milestone fails review if it adds code that only one scene can use.
3. **Prove generality early.** The nature benchmark (B2) is reached *before* the cozy room (B1) is finished, so the core is never shaped around one image.
4. **Benchmarks are regression tests.** Each benchmark has golden frames (stored hashes of rendered frames) checked by `tools/check.sh` from the moment it first renders (no hosted CI, [DECISIONS.md](engineering/DECISIONS.md) D-006).
5. **Code first, editor second.** The engine is usable from KDL + CLI + preview before the full editor exists.
6. **Hold-outs every milestone from M3 on** (before M3 there is not enough engine to author a scene). Two unplanned scenes are authored from the docs alone at the end of each milestone ([BENCHMARKS.md](BENCHMARKS.md) §12). The hold-out pass rate is tracked and must trend up.
7. **Measured, not eyeballed.** A benchmark is "complete" only when its automatic checks and human rubric pass ([BENCHMARKS.md](BENCHMARKS.md) §1).

### Benchmarks

| | Scene | Main stress |
|---|---|---|
| **B1** | Cozy night room | Interior, side view, lighting, small characters |
| **B2** | Forest at dusk | Organic generators, parallax, wind, fireflies |
| **B3** | Rainy neon street | One-point perspective, neon glow, reflections, walkers, a car |
| **B4** | Game hero + effects | Biped in 4/8 directions, clips, effects, sheet export |
| **B5** | Underwater coast | Water surface, caustics, god rays, fish schools, kelp |
| **B6** | Isometric village | Isometric view, autotiles, buildings, tileset export |
| **B7** | Character close-up | Face rig, expressions, talking, large-form cleanup |
| **B8** | Short story | Shots, transitions, continuity, interacting rigs |
| **B9** | Style switch | Style presets, `detail-scale` regeneration, color limits |

---

## 1. Milestones

### M0 — Walking skeleton
> **Status (2026-10-28): done.** Done: workspace with `px-core`, `px-expr`, `px-raster`, `px-format`, `px-engine`, `px-export`, `px-cli` (two more crates than planned, see [DECISIONS.md](engineering/DECISIONS.md) D-004); `pxe render` / `pxe export`; `examples/hello.kdl` exports a seamless 24-frame GIF; golden hashes, loop-seam, palette-compliance and allocation-free tests pass locally; local check script `tools/check.sh` (hosted CI dropped to avoid costs, D-006); both spikes done: KDL round-trip ([spikes/kdl-roundtrip.md](spikes/kdl-roundtrip.md)) and cleanup quality ([spikes/cleanup.md](spikes/cleanup.md)), each with a "go" verdict. Optional: confirming identical hashes in a local Linux container.

A thin end-to-end slice: a tiny KDL file becomes a looping GIF. Every later milestone thickens this slice instead of building layers in isolation.

- **Scope (in):**
  - Git repository, Cargo workspace with only the crates needed now: `px-core`, `px-format`, `px-raster`, `px-export`, `px-cli`.
  - `px-core`: indexed canvas (material + tone buffers; depth, emission and id reserved but unused), integer frame clock, counter-based seeded RNG, palette and ramp types.
  - `px-format`: load `project.kdl`, one `palette` (ramps only) and one `scene` with `rect`, `ellipse` and `line` nodes (`x y width height material tone`). Values: fixed numbers and a minimal expression subset (`=` prefix; `t`, `f`; `+ - * /`; `sin`, `cos`, `floor`). Errors with file:line.
  - `px-raster`: integer rasterization of those three shapes, pixel-snapped.
  - `px-export`: resolve indexed → RGB, nearest-neighbor integer upscale, PNG frames, looping GIF.
  - `px-cli`: `pxe render` (PNG) and `pxe export` (GIF).
  - Test harness: golden **indexed-buffer hashes** plus a palette-compliance check (the first automatic check of [BENCHMARKS.md](BENCHMARKS.md) §1).
  - `tools/check.sh`: size and commit guards, format, clippy, tests, golden hashes, release-assertions and the allocation test, run locally (no hosted CI, D-006).
- **Risk spikes (time-boxed, throwaway code, findings written into the docs):**
  1. **Cleanup quality:** render a few SDF shapes (blob, capsule, leaf) through orphan removal, pixel-perfect lines and sel-out; compare side by side with reference pixel art crops. Output: a short verdict and before/after images in `docs/spikes/cleanup.md`. If the result looks "generated", RENDERING §7 is revised before M2.
  2. **KDL round-trip:** with the chosen Rust KDL v2 crate, change a value and a keyframe in a commented file; check that comments, ordering and formatting survive. Output: `docs/spikes/kdl-roundtrip.md` with the crate choice, or the fallback plan (our own format-preserving layer).
- **Scope (out):** graph runtime, generators, lighting, dither, packs, preview window, editor, scripts.
- **Done when:**
  - `pxe export examples/hello.kdl` produces a seamless looping GIF of a ball bouncing (`y="=…sin(t…)"`) in front of a two-ramp background;
  - the same scene gives byte-identical indexed-buffer hashes on macOS and in a local Linux container (Windows deferred, D-006);
  - both spike verdicts are written down.
- **Unlocks:** the authoring loop exists end to end (KDL → pixels → file), and both make-or-break assumptions are tested.
- **Risks:** over-engineering the buffers. *Mitigation:* freeze the canvas layout after M2; profile before optimizing. Spikes balloon. *Mitigation:* strict time box; spike code is deleted, only the findings are kept.

### M1 — Format & live preview
> **Plan (2026-10-28):** built in five slices, each ending with `tools/check.sh` green.
> 1. ✅ **Expression language** (done 2026-10-28) — everything in [FORMAT.md](FORMAT.md) §9: comparisons, logic, ternary, `**`, time and angle literals, every math, periodic, time-scope, easing, randomness and noise function; rounding per §9.7; non-finite values become 0 with a per-frame report (§9.8).
> 2. ✅ **Value kinds** (done 2026-10-28) — units, seeded ranges (`"2..5"`), `param` / `$name`, `@id.prop` references with cycle detection.
> 3. ✅ **Keyframes** (done 2026-10-28) — `anim` tracks, every interpolation, relative times, track loop and blend modes, multi-property keys in `camera`. Tracks compile into expression programs, so references and time scopes see animated values.
> 4. ✅ **Validation** (done 2026-10-28) — a schema for every node kind (in `px-format`, not a separate `px-schema` crate yet); "did you mean" suggestions for nodes, properties, anim targets, interpolations, ids, params, palettes, ramps, light colors and scenes; unknown properties are warnings (§15). Pending: `--json` diagnostics and `--strict`.
> 5. ✅ **Live loop** (done 2026-10-28) — `pxe watch` with hot reload, a preview window and per-frame render time; `pxe bench` reports frame times for every example and benchmark scene, and `tools/check.sh` fails when a frame is over budget.
>
> Slice 2 notes: params are substituted when the file loads (a changed param means a reload, which the live loop in slice 5 does automatically) and `--param name=value` overrides them; text params can't be used in expressions yet. References are inlined when the file loads, keeping the target's seed; `@id.inner.prop`, `@parent`, sockets, anchors and states arrive with prefabs and rigs. Ranges don't use a repeater index yet, because `repeat` doesn't exist yet.
>
> Slice 1 notes: strings in expressions are limited to ease names and time literals until references land (slice 2); shape seeds derive from the shape's position in the scene until objects have ids; time noise loops automatically only through `wiggle` (use `loop()` for `noise`/`fbm` fed with `t`).
- **Scope:** the full format on top of the M0 loader: every value kind (units, ranges, `=expr`, `$param`, `@ref`), the complete expression language ([FORMAT.md](FORMAT.md) §9), keyframes, schema validation, errors with suggestions, hot reload, a minimal preview window, and frame-time measurement in `tools/check.sh`.
- **Deliverables:** `px-expr`, `px-schema`, extended `px-format`, `pxe watch`, preview window, preview-speed check in `pxe bench`.
- **Done when:** editing a number in VS Code updates the preview in under 1 s; a typo produces `file:line — … Did you mean …?`; a warning never stops rendering; `tools/check.sh` reports frame times for every example scene.
- **Unlocks:** authoring loop for every benchmark.
- **Risks:** KDL v2 crate maturity. *Mitigation:* wrap it behind our own document model so the parser can be swapped.

### M2 — Graph, 2D shapes & pixel cleanup
> **Pulled forward (2026-10-28), without the graph runtime:** `rounded-rect`, `capsule`, `polygon`, `blob` and thick lines through SDF rasterization; a per-shape coverage mask with orphan and notch cleanup; dome and soft form shading from a distance field; full and selective outlines; clustered-noise texture; gradients as solid bands with short dithered seams; clip groups. Still M2: the graph runtime, generators, sel-out tuned against reference crops, prefabs.
- **Scope:** graph runtime (lazy evaluation, caching, static/animated detection, parallel branches); generate, SDF 2D, noise and patterns, field ops, pixel art filters, color/palette nodes; `ramp-gen`; groups (prefabs in KDL).
- **Deliverables:** `px-graph`, extended `px-raster`, `px-filters`, `px-nodes` (NODES §1, 2, 5–8, 18).
- **Done when:**
  - noise → threshold → cluster → sel-out produces clean shapes;
  - automated checks report zero L-corners, zero orphans and regular steps on test shapes;
  - a KDL-defined prefab renders identically to the same graph built by hand.
- **Unlocks:** static props for B1, rocks and clouds for B2, flat tiles for B6.
- **Risks:** procedural shapes look "generated". *Mitigation:* cleanup passes are tuned against reference pixel art crops; side-by-side review is part of the done criteria.

### M3 — Procedural generators & first packs (generality proof)
> **Pulled forward (2026-10-28):** layers with parallax and a keyframed camera.
- **Scope:** `lsystem`, `tree`, `terrain-1d`, `skyline`, `facade`, `rock`, `cloud`, `grass`, `scatter`, `distribute-path`; layers with parallax; packs `core-kit` and `nature` (written in KDL); `interior` pack v0.
- **Deliverables:** `px-nodes` (NODES §4, 10, 11), `px-packs`, packs `core-kit`, `nature`, `interior`.
- **Done when:** **B2 static** (a dusk forest with parallax hills and three tree kinds) and **B1 static, without lights or characters** both render with no engine code specific to either.
- **Unlocks:** B2 static, B1 static.
- **Risks:** the generators turn into a zoo. *Mitigation:* every generator must be used by at least two packs or benchmarks.

### M4 — Lighting
> **Pulled forward (2026-10-28):** ambient darkness and occlusion; point, cone and rect lights with banded falloff; light colors through hand-written twins; emissive surfaces; cast shadows (ray-marched against `casts-shadow` shapes); contact shadows; fog as atmospheric perspective; reflections limited to a surface ramp. The B1 re-attempt's lamp tints the blue wall warm. Still M4: directional lights, auto-twins, glow and god rays, style presets, shipped palettes.
- **Scope:** ambient, point, cone, rect, directional lights; tone steps with dithered falloff; twins and auto-twins; emissive ramps; `shadow-2d`, `glow`, `god-rays`; dither anchored to world space; **style presets** and `detail-scale` regeneration ([RENDERING.md](RENDERING.md) §16).
- **Deliverables:** `px-light`, the palette twin system, style presets, shipped palettes `night-blue`, `dusk`, `morning`.
- **Done when:** the B1 lamp cone tints the blue wall warm; the B2 dusk light comes through the canopy as god rays; swapping the palette `night-blue` ↔ `dusk` works on both scenes unchanged; B1 and B2 stills render in all six style presets with zero scene edits.
- **Unlocks:** B1 and B2 lit, B9 stills.
- **Risks:** auto-twins look muddy. *Mitigation:* hand-written twins for the key roles; snap to existing palette colors (`closed=#true`).

### M5 — Animation & simulation
> **Pulled forward (2026-10-28):** stateless loop-aware particles, ropes with sag and sway, a wind field read by `wind(x)`, loop-aware reflection ripples.
- **Scope:** time, modulators, keyframes, `hold` (on2/on3), loop/delay/stagger, loop solver, loopable noise; particles, `boids`, `rope`, `cloth`, `wind-field`, `water-surface`, `caustics`, `feedback`; checkpoints for stateful nodes.
- **Deliverables:** `px-sim`, `px-fx`, NODES §12–13, simulation checkpoints, `water` pack v0.
- **Done when:**
  - every animated benchmark passes the loop test (`frame[duration] == frame[0]`);
  - scrubbing to any frame takes under 200 ms with checkpoints;
  - B5 renders a water line, caustics, light rays and a boid school of placeholder fish.
- **Unlocks:** B1 and B2 animated (rain, flicker, fireflies, wind), B5 environment, B9 complete (loops in every style), B4 effects (`vfx` prefabs).
- **Risks:** simulations break perfect loops. *Mitigation:* warm-up periods plus loopable variants of noise and wind; the loop solver reports which node breaks the loop.

### M6 — Rig & characters (2D path)
- **Scope:** body plan templates (`biped`, `quadruped`, `bird`, `fish`, `serpent`, `multi-leg`, `machine`) and instances (`human-adult`, `human-child`, `cat`, `dog`, `bird`, `fish`); volumes, facings `front`/`side`/`back`, postures, clips, channels, IK targets, `spring-bones`, state machines; pieces (KDL `piece`), parametric hair and outfits; `creatures` pack.
- **Deliverables:** `px-rig`, `px-pieces`, the `creatures` pack, standard piece library (PIECES.md).
- **Done when:** the B1 characters type, look and sway; B2 gets a deer and birds; B5 gets real fish from the `fish` template; adding a new creature needs only a KDL body plan; the B7 shopkeeper renders all six expressions as stills and passes the face cleanup checks.
- **Unlocks:** B1 complete, B2 complete, B5 complete, B7 stills, B8 cast prefabs.
- **Risks:** 2D limbs look stiff at larger sizes; faces look generic. *Mitigation:* path A is officially for 16–48 px full bodies; close-ups use the parametric **face rig** and `framing` (CATALOG §7.2.1); large full-body action waits for path B (M10). The B7 blind expression test runs here.

### M7 — Editor v1
- **Scope:** preview with debug views, inspector with ◆, timeline, hierarchy, library, palette panel, console; KDL round-trip edits that preserve comments; node graph panel (read-only at first, then editable).
- **Deliverables:** `px-editor`, `px-engine` facade (EDITOR.md).
- **Done when:** a slider change in the editor rewrites the KDL with a minimal diff; keyframes created in the timeline appear as an `anim` block; opening a prefab shows its graph.
- **Unlocks:** comfortable authoring for all benchmarks.
- **Risks:** round-trip edge cases. *Mitigation:* round-trip fuzz tests on every benchmark file.

### M8 — TypeScript scripts
- **Scope:** QuickJS embedding, type stripping, deterministic sandbox, value and event scripts, `.d.ts` generation, checkpointed event state.
- **Deliverables:** `px-script` (SCRIPTING.md).
- **Done when:** an event script ("the cat jumps when the lamp goes out") is deterministic across runs and survives scrubbing; the B8 fetch script (throw, chase, pick up via `on=`, return) passes the continuity checks; VS Code autocompletes real ids and states.
- **Unlocks:** behaviors in any benchmark.
- **Risks:** scripts become the main way to build scenes. *Mitigation:* documentation rule and editor hint: "repeated script logic → make a node or prefab".

### M9 — Director & export
- **Scope:** shots, sequences, camera (follow, pan), transitions, MP4/WebM, APNG/WebP, PNG sequences, `export-layers` (parallax layers for games).
- **Deliverables:** `px-director`, extended `px-export`, `pxe export` for sequences.
- **Done when:** a 30 s sequence with 3 shots across B1, B2 and B5 exports as MP4 with crisp pixels; B2 exports as separate parallax layers usable in a game engine.
- **Unlocks:** video output for every benchmark; B7 complete (talking and expression cues on the timeline); B8 shots 2 and 3 with transitions and the palette swap.
- **Risks:** ffmpeg dependency. *Mitigation:* GIF/PNG stay pure Rust; ffmpeg is optional and detected at runtime.

### M10 — 3D-to-pixel path & views
- **Scope:** 3D primitives, `csg`, `extrude`, `lathe`, `model`, `camera3`, `render3` (toon bands, snapping, outline), `rig-3d`, `direction-set`; scene `view`: `top-down`, `isometric`; `model` variants of key prefabs.
- **Deliverables:** `px-model`, 3D nodes (NODES §3), `view` support in the director and packs.
- **Done when:** B4's hero renders in 8 directions with idle/walk/run/jump/attack with no pixel crawl between frames; a `model` house renders correctly in side, top-down and isometric views.
- **Unlocks:** B4 complete, B6 buildings.
- **Risks:** pixel crawl and jitter on rotation. *Mitigation:* snapping, fixed rotation sets per direction, temporal stability tests on golden frames.

### M11 — Perspective & urban
- **Scope:** `view="perspective"` (one and two point), `perspective-place` with discrete scale steps, `reflection`, neon (`glow` + `text` + flicker), puddles (`ripples`), walkers on paths, vehicles from the `machine` template; `urban` pack.
- **Deliverables:** perspective support, `packs/urban`.
- **Done when:** B3 renders: a street vanishing to a point, neon signs reflected in puddles, rain, pedestrians walking toward and away from the camera, a passing car.
- **Unlocks:** B3 complete, B8 complete (the perspective street shot).
- **Risks:** scaling sprites in perspective breaks pixel art. *Mitigation:* discrete scale steps with a re-rasterize per step (never a resample), or 3D models through `render3`.

### M12 — Game asset pipeline
- **Scope:** `tileset-gen`, `autotile` (16/47 blob), `wfc`, `terrain-2d`, `iso-place`; sprite sheet metadata (JSON, Aseprite, Godot, Unity); tileset metadata (Tiled, LDtk, Godot); `variations` contact sheets.
- **Deliverables:** export plugins, the isometric part of `core-kit`.
- **Done when:** B6 renders an isometric village built by `wfc` from autotiles and imports into Tiled/Godot with working autotile rules; B4's sheet imports into Godot and Aseprite with named animations.
- **Unlocks:** B4 and B6 complete.
- **Risks:** too many export targets. *Mitigation:* JSON + Aseprite + Godot first; others are community plugins.

### M13 — v1 hardening
- **Scope:** performance pass, pack authoring guide, palette and body plan authoring guides, every benchmark tuned, docs updated.
- **Done when:** all nine benchmarks pass their automatic checks and human rubric for 5 seeds; a 256×256 scene previews at ≥ 30 fps on a mid-range laptop; **at least 5 of the last 6 hold-out scenes** passed with no engine changes.

---

## 1.1 Release checkpoints

Usable versions along the way, so the engine is useful long before v1:

| Release | After | What you can do with it |
|---|---|---|
| **v0.1** | M5 | Ambient loops without characters: rooms, forests, water, weather, in any palette and style (B1/B2/B5 environments, B9) |
| **v0.2** | M7 | Characters and creatures in side view, plus the editor (B1, B2, B5 complete, B7 stills) |
| **v0.3** | M9 | Scripts, shots, sequences and video export (B7 complete, most of B8) |
| **v1.0** | M13 | All views, game asset pipeline, all nine benchmarks, hold-out pass rate ≥ 5/6 |

---

## 2. Benchmark progress by milestone

`◐` partial · `●` complete

| | M2 | M3 | M4 | M5 | M6 | M8 | M9 | M10 | M11 | M12 |
|---|---|---|---|---|---|---|---|---|---|---|
| **B1** room | ◐ | ◐ | ◐ | ◐ | ● | | | | | |
| **B2** forest | ◐ | ◐ | ◐ | ◐ | ● | | | | | |
| **B3** street | | ◐ | ◐ | ◐ | ◐ | | | ◐ | ● | |
| **B4** hero + fx | | | | ◐ | ◐ | | | ● | | ● (sheet metadata) |
| **B5** underwater | | ◐ | ◐ | ◐ | ● | | | | | |
| **B6** isometric | ◐ | | | | | | | ◐ | | ● |
| **B7** close-up | | | | | ◐ | | ● | | | |
| **B8** story | | | | | ◐ | ◐ | ◐ | | ● | |
| **B9** styles | | | ◐ | ● | | | | | | |

---

## 3. Cross-cutting risks

| Risk | Impact | Mitigation |
|---|---|---|
| Everything looks "procedural" | The core promise fails | Cleanup passes + hand-tuned palettes + reference crops in reviews; `wear`/`detail` knobs |
| Core slowly bends toward one benchmark | Not general | Principle 2 enforced in review; each milestone must serve ≥ 2 benchmarks |
| Determinism breaks (floats, threads) | Loops and golden tests fail | Integer frame time, fixed-step sims, seeded RNG per object, cross-platform golden tests |
| KDL round-trip is fragile | Editor loses user edits | Format-preserving document model + fuzz tests |
| 3D-to-pixel quality | B4/B6 look off | Snapping, toon bands, outline passes, temporal stability tests; a time-boxed 3D-to-pixel spike (28 px biped, 8 directions) **before M6**, so a bad result can still change the character plan |
| QuickJS determinism or speed | Scripts break loops or scrubbing | A time-boxed QuickJS spike **before M7** |
| Scope creep | v1 never ships | Out-of-scope list below; new content goes into packs, not the core |

---

## 4. Out of scope for v1
- Interactive runtime or game engine (we export assets; we don't run games).
- AI generation of any kind (text-to-scene, image generation).
- Fluid simulation beyond particles, `water-surface` and `ripples`.
- Three-quarter view in the 2D character path (use path B).
- Audio-reactive animation and an audio timeline.
- Real-time collaboration and cloud features.
- Mesh import beyond basic `.obj`.
- Mobile or web editor (the CLI can still export for the web).

---

## 5. Feature table

| Feature | v1 | v2 | Later |
|---|---|---|---|
| Indexed renderer, palettes, ramps, twins | ● | | |
| KDL format, expressions, hot reload | ● | | |
| Node graph runtime + core nodes | ● | | |
| Generators (L-system, tree, terrain, skyline, facade, rock, cloud) | ● | More kinds | |
| Lighting, shadows, glow, god rays, reflections | ● | Normal-mapped sprites | |
| Animation (modulators, keyframes, clips, state machines) | ● | | |
| Simulations (particles, boids, rope, cloth, wind, water) | ● | Fluid-lite | |
| Characters path A (2D, 16–48 px) | ● | Three-quarter facing | |
| Characters path B (3D-to-pixel) | ● (basic) | Advanced rigs | |
| Views: side, perspective, top-down, isometric | ● | | Free 3D camera |
| Editor (preview, inspector, timeline, graph) | ● | Graph authoring polish | |
| TypeScript scripts | ● | | |
| Export: GIF, MP4/WebM, PNG, sprite sheets, tilesets, layers | ● | More engine metadata | |
| Packs: core-kit, nature, interior, urban, water, creatures | ● | fantasy, sci-fi, weather+ | Community pack registry |
| Audio track / beat sync | | ● | |
| Interactive preview runtime (playable scenes) | | | ● |
| Plugin API for third-party nodes (Rust/WASM) | | ● | |
