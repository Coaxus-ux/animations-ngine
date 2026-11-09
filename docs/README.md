# Pixel Art Animation Engine — Documentation

> Status: M1 is built, with parts of M2–M5 and M9 (scene composition, timeline, zoom, text, sequences of shots) pulled forward; see [ROADMAP.md](ROADMAP.md) and [FORMAT.md appendix A](FORMAT.md#appendix-a-what-the-loader-accepts-today) for what works today.

A general-purpose engine to create pixel art scenes, characters, animations and game assets **entirely from code and data**: no drawing skills required and no AI inside the engine. Scenes are written in KDL, behaviors in TypeScript, and everything renders through a palette-indexed Rust core.

## What it is for
- Animated scenes and perfect loops (GIF, MP4, WebM, APNG)
- Short sequences with shots, camera moves and transitions
- Game assets: sprite sheets per direction, tilesets with autotile rules, parallax layers
- Any setting: interiors, nature, cities, underwater, fantasy, sci-fi

## Reading order

| # | Document | What's inside |
|---|---|---|
| 1 | [SPEC.md](SPEC.md) | Vision, generality rules, feasibility, principles, technology, decision log |
| 2 | [BENCHMARKS.md](BENCHMARKS.md) | The nine test scenes, automatic checks and hold-out process that keep the engine general |
| 3 | [FORMAT.md](FORMAT.md) | Complete KDL format and expression language reference |
| 3a | [SHORTHAND.md](SHORTHAND.md) | Shorter scene files: bare values, `repeat=`, `motif`, `scatter`, the shared kit |
| 3b | [PIXELS.md](PIXELS.md) | Pixel layers: text grids and PNG art (`pixels src=`) |
| 4 | [CATALOG.md](CATALOG.md) | Prefab contract, content packs, body plan templates |
| 5 | [PALETTES.md](PALETTES.md) | Ramps, tones, roles, twins, ramp generation, shipped palettes |
| 6 | [PIECES.md](PIECES.md) | Text-grid pieces and the standard piece library |
| 7 | [ANIMATION.md](ANIMATION.md) | Time, keyframes, modulators, rigs, gaits, clips, simulations, loops |
| 8 | [DIRECTOR.md](DIRECTOR.md) | Scenes, shots, camera, transitions, all export formats |
| 9 | [RENDERING.md](RENDERING.md) | Render pipeline, views, 3D-to-pixel, cleanup passes, lighting |
| 10 | [NODES.md](NODES.md) | Low-level node library reference |
| 11 | [SCRIPTING.md](SCRIPTING.md) | TypeScript API for value and event scripts |
| 12 | [EDITOR.md](EDITOR.md) | The Pixel Composer-inspired editor |
| 13 | [CLI.md](CLI.md) | Command-line tool |
| 14 | [ARCHITECTURE.md](ARCHITECTURE.md) | Rust workspace, graph runtime, packs, testing |
| 15 | [ROADMAP.md](ROADMAP.md) | Milestones, v1 scope, risks |
| 16 | [GLOSSARY.md](GLOSSARY.md) | Every term in one place |
| 17 | [engineering/RUST-STANDARDS.md](engineering/RUST-STANDARDS.md) | Rust engineering standard (SOLID, DRY, allocation profiles, tests, lints) |
| 18 | [engineering/DECISIONS.md](engineering/DECISIONS.md) | Approved exceptions and engineering decisions |

## The short version
1. **Describe, don't draw.** Shapes come from geometry, 3D primitives, generators and small text-grid pieces; cleanup rules make them look hand-placed.
2. **The palette is law.** Pixels are a ramp plus a tone; light, mood and transitions are ramp operations.
3. **A frame is a function** of scene, seed and frame number, so it is deterministic, parallel and loops perfectly.
4. **Data first.** KDL is the source of truth and the editor round-trips it; TypeScript only inside nodes.
5. **Generic core, content in packs.** Nothing in the engine knows about rooms, forests or cats.
