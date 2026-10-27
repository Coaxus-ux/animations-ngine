# Glossary (draft v1)

> Companion to [SPEC.md](SPEC.md). Terms are listed alphabetically.

| Term | Meaning | See |
|---|---|---|
| **3D-to-pixel** | Rendering 3D primitives or models at low resolution, banded to ramps and outlined, to get pixel art from any angle | [RENDERING.md](RENDERING.md) |
| **Alias** | Maps a palette's local ramp name to a standard role (`alias wall="plaster-blue"`) | [PALETTES.md](PALETTES.md) |
| **Ambient** | Global darkness level, expressed in tone steps | [PALETTES.md](PALETTES.md) |
| **Anchor** | A named point on a body plan or prefab where a piece attaches (`eyes`, `crown`) | [PIECES.md](PIECES.md) |
| **Autotile** | Rules that choose the right tile variant from its neighbors | [DIRECTOR.md](DIRECTOR.md) |
| **Bake** | Converts a script-generated scene into an editable graph (one way) | [EDITOR.md](EDITOR.md) |
| **Benchmark** | One of nine reference scenes (B1–B9) used to prove the engine stays general | [BENCHMARKS.md](BENCHMARKS.md) |
| **Body plan** | A KDL definition of a creature or machine: bones, volumes, anchors, slots, postures, clips | [CATALOG.md](CATALOG.md) |
| **Body plan template** | A generic skeleton family (`biped`, `quadruped`, `bird`, `fish`, `serpent`, `multi-leg`, `machine`) that body plans build on | [ANIMATION.md](ANIMATION.md) |
| **Canvas** | The set of indexed buffers (material, tone, depth, emission, id) a node produces | [RENDERING.md](RENDERING.md) |
| **Channel** | A clip layer (`posture`, `body`, `arms`, `head`, `face`); higher channels override the bones they own | [ANIMATION.md](ANIMATION.md) |
| **Checkpoint** | A saved state of a stateful node, so scrubbing doesn't re-simulate from frame 0 | [ARCHITECTURE.md](ARCHITECTURE.md) |
| **Cleanup passes** | Algorithms that apply pixel artists' rules (pixel-perfect lines, orphan removal, sel-out…) | [RENDERING.md](RENDERING.md) |
| **Clip** | A named animation for a rig (`walk`, `blink`, `type`) | [ANIMATION.md](ANIMATION.md) |
| **`detail-scale`** | Regenerates content at a different size instead of resampling pixels | [RENDERING.md](RENDERING.md) |
| **Emissive** | A ramp or pixel that ignores ambient darkness | [PALETTES.md](PALETTES.md) |
| **Event script** | A stateful TypeScript script that reacts to events; simulated from frame 0 with checkpoints | [SCRIPTING.md](SCRIPTING.md) |
| **Expression** | A value starting with `=`, evaluated every frame (`"=2*sin(t)"`) | [FORMAT.md](FORMAT.md) |
| **Face rig** | Parametric eyes, brows, nose and mouth with expression blending, for heads ≥ 16 px | [CATALOG.md](CATALOG.md) |
| **Facing** | The view of a character (`front`, `side`, `back`, or 4/8 directions in 3D) | [ANIMATION.md](ANIMATION.md) |
| **Framing** | Rendering only part of a character (`full`, `bust`, `head`) for close-ups | [CATALOG.md](CATALOG.md) |
| **Gait** | Parameters that generate locomotion cycles (walk, run, swim, fly, slither) | [ANIMATION.md](ANIMATION.md) |
| **Group (nodes)** | A sub-graph with inputs and outputs; prefabs are groups | [NODES.md](NODES.md) |
| **Group (ramps)** | Interchangeable ramps an object picks from by seed (`books-cold`) | [PALETTES.md](PALETTES.md) |
| **Hold-out scene** | An unplanned scene authored from the docs alone to test generality | [BENCHMARKS.md](BENCHMARKS.md) |
| **Light color** | A named color lights use to tint (`warm`, `cool`, `screen`) | [PALETTES.md](PALETTES.md) |
| **Local name** | A palette's own descriptive ramp name | [PALETTES.md](PALETTES.md) |
| **Model** | A prefab built from 3D primitives and rendered via 3D-to-pixel; works in every view | [CATALOG.md](CATALOG.md) |
| **Modulator** | A time-varying value source (wiggle, oscillator, curve) wired into any parameter | [ANIMATION.md](ANIMATION.md) |
| **on2 / on3** | Holding each animated value for 2 or 3 frames, for the classic pixel art cadence | [ANIMATION.md](ANIMATION.md) |
| **Pack** | A swappable bundle of prefabs, body plans, pieces and palettes (`core-kit`, `nature`…) | [CATALOG.md](CATALOG.md) |
| **Param** | A scene value exposed as an editor control (`param rain 0.8`), used as `$rain` | [FORMAT.md](FORMAT.md) |
| **Piece** | A small text grid of slots for details geometry can't do well (glasses, logos) | [PIECES.md](PIECES.md) |
| **Pivot** | The pixel of a piece that aligns with its anchor | [PIECES.md](PIECES.md) |
| **Posture** | A base pose of a body plan (`stand`, `sit-chair`, `rear-up`) | [CATALOG.md](CATALOG.md) |
| **Prefab** | A reusable object: a group of nodes with params, slots, sockets, states and emitters | [CATALOG.md](CATALOG.md) |
| **Ramp** | Ordered colors from dark to light for one material | [PALETTES.md](PALETTES.md) |
| **`ramp-gen`** | Builds a hue-shifted ramp from a single base color | [PALETTES.md](PALETTES.md) |
| **Role** | A standard semantic ramp name that prefabs default to (`wall`, `skin`, `foliage`) | [PALETTES.md](PALETTES.md) |
| **Seed** | The number that makes randomness repeatable; each object derives its own | [SPEC.md](SPEC.md) |
| **Sel-out** | Selective outline: lighter where lit, darker in shadow | [RENDERING.md](RENDERING.md) |
| **Sequence** | Shots joined by transitions | [DIRECTOR.md](DIRECTOR.md) |
| **Shot** | A scene seen through a camera for a duration | [DIRECTOR.md](DIRECTOR.md) |
| **Slot** | A named surface on a prefab, body plan or piece that maps to a ramp | [CATALOG.md](CATALOG.md) |
| **Socket** | A named attachment point on a prefab (`top`, `seat`, `screen`) | [CATALOG.md](CATALOG.md) |
| **Style preset** | A bundle of rendering rules (outlines, shading, dither, tones, color limit) independent of palette and content: `moody`, `cartoon`, `one-bit`… | [RENDERING.md](RENDERING.md) |
| **Tone** | A step on a ramp: `deep`, `shade`, `base`, `lit`, `hi` | [PALETTES.md](PALETTES.md) |
| **Twin** | The ramp a pixel switches to under a colored light (`wall` → `wall-warm`) | [PALETTES.md](PALETTES.md) |
| **Value script** | A pure TypeScript script that returns values from time and scene | [SCRIPTING.md](SCRIPTING.md) |
| **View** | The scene projection: `side`, `perspective`, `top-down`, `isometric` | [RENDERING.md](RENDERING.md) |
| **Volume** | The 2D or 3D shape attached to a bone (capsule, ellipse, rounded box) | [ANIMATION.md](ANIMATION.md) |
| **Wear** | A common param (0..1) adding age and mess | [CATALOG.md](CATALOG.md) |
