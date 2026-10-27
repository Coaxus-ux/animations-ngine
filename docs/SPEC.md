# Pixel Art Animation Engine — Specification (draft v1)

> Status: design phase. No code yet. This document consolidates every decision made so far.

## 1. Goal

A **general-purpose** engine to create pixel art scenes, characters and animations **entirely from code and data**, without needing to draw and with **no AI inside the engine**. It must work for any kind of scene (interiors, nature, cities, underwater, fantasy, sci-fi) and for **game assets** (sprite sheets, tilesets, parallax layers), not just one look.

### Generality rules
1. **The core is scene-agnostic.** Rendering, palettes, lighting, animation, rigs and the director know nothing about rooms, books or cats.
2. **Content ships as swappable packs.** Prefabs and body plans live in packs (`core-kit`, `interior`, `nature`, `urban`, `water`, `creatures`…), all written in the same KDL any user can write. See [CATALOG.md](CATALOG.md).
3. **Generic building blocks first.** Every pack is built from generic kits (shapes, 3D primitives, L-systems, terrain, tiles, body plan templates), so new content never needs new engine code.
4. **Several views.** Flat side view first; perspective helpers, top-down and isometric through the 3D-to-pixel path. See [RENDERING.md](RENDERING.md).
5. **Validated against diverse benchmarks.** The original cozy-room image is only one of nine test scenes, which include the engine's weak spots (close-up faces, multi-shot stories, other art styles); unplanned hold-out scenes are added every milestone. See [BENCHMARKS.md](BENCHMARKS.md).

**Success test:** every benchmark passes its automatic checks and human rubric for several seeds ([BENCHMARKS.md](BENCHMARKS.md) §1), no benchmark needed engine code written specifically for it, and unplanned hold-out scenes can be authored from the docs alone.

### Core idea

If neither you nor an AI draws a shape, the shape has to come from **geometry plus pixel art cleanup rules**. Those rules replace the artist's hand. The engine's job is to turn *descriptions* into pixels that look hand-placed.

---

## 2. Feasibility

### Very doable, high quality
- Man-made props of any kind: furniture, electronics, buildings and facades, vehicles, signs. Built from 2D shapes or 3D primitives.
- Nature: trees and plants (L-systems), terrain and layered hills, rocks, water surfaces, skies, clouds.
- Lighting: ambient darkness, light cones, point lights, emissive screens, neon and windows, dithered light edges, colored tinting, reflections.
- Weather and FX: rain, snow, fog, steam, smoke, fireflies, dust in beams, caustics, light rays, bubbles.
- Small and medium characters (16–48 px) from body plan templates (biped, quadruped, bird, fish, serpent, multi-leg, machine): idle, breathing, blinking, looking, gaits.
- Game assets: sprite sheets per direction, tilesets with autotile rules, parallax layers.
- Infinite variation through seeds and palette swaps.

### Doable with effort
- Large characters (>64 px) with expressive faces. This needs a good piece library, and the result is "correct", not "authored".
- Complex animals, such as a galloping horse. The 3D-to-pixel path makes it possible, but it needs rigging work.
- Very different art styles. Each new style is a new set of rules and palettes.

### Not realistic
- Matching a skilled illustrator in detailed close-ups.
- Inventing a style. Style is defined by you through palettes, rules and pieces.
- Typing "a samurai" and getting one without a pre-built body plan and pieces (that would require AI).
- Free rotation of sprites. Pixel art only survives 90°/180° rotations. Other angles go through the 3D-to-pixel path.
- Sub-pixel motion or non-integer scaling.

---

## 3. Core principles

1. **A frame is a function.** `frame = render(scene, seed, frame_number)`. There is no hidden state. Any frame can be rendered instantly and in parallel, and loops are guaranteed. (Exceptions: feedback, simulations and event scripts. See §6.6.)
2. **Low resolution is real resolution.** The engine works at 256×256 (or 320×180) and upscales only at output, by integer factors, with nearest-neighbor.
3. **The palette is law.** Nothing is drawn in RGB. Pixels store a *material* (a color ramp) and a *tone* (a step on that ramp). Light moves tones along ramps. RGB is resolved only at the very end.
4. **Integer everything.** Time is counted in integer frames, positions snap to whole pixels, and zoom is integer only.
5. **Determinism.** Every object derives its own seed from the global seed and its id, so changing one object never reshuffles the rest.
6. **Data first, code as the exception.** Scenes are KDL documents that both humans and the editor can edit. TypeScript appears only inside nodes that need behavior.
7. **No AI in the engine.** All visuals are deterministic and reproducible.

---

## 4. Technology

| Area | Choice |
|---|---|
| Core language | Rust |
| Rendering | CPU software renderer (65k pixels per frame is trivial) |
| Scene format | KDL v2 (format-preserving, so comments survive UI edits) |
| Scripting | TypeScript/JavaScript via embedded QuickJS, with types stripped on load |
| Editor UI | egui, a node editor widget, framebuffer as a texture |
| Parallelism | rayon |
| Hot reload | file watching (notify) |
| Export | gif/image crates; ffmpeg as an external process for MP4/WebM |

Rejected alternatives:
- **Rust as the scene language:** compile times kill iteration.
- **A custom DSL:** a whole parser, error and tooling project of its own.
- **Rhai:** weak tooling.
- **Lua:** viable but less familiar.
- **YAML:** fragile, and comments are lost on rewrite.
- **JSON with a schema:** good autocompletion, but verbose.
- **JS-first scenes:** the UI cannot round-trip imperative code.

---

## 5. Authoring model

```
  KDL files  ◄──────►  Editor UI          (true two-way editing)
      │
      ▼
   GRAPH (internal model)  ──►  Render  ──►  Preview / Export
      ▲
      │
  TS scripts (attached to individual nodes)
```

- **KDL is the source of truth.** When you move a slider in the editor, it rewrites the number in the file and keeps your comments and formatting.
- **Scripts live inside nodes.** The scene contains scripts; scripts never contain the scene.
- **Heavy, reusable logic belongs in Rust** as new nodes or prefabs. If a scene needs lots of script logic, that is a sign a new prefab is missing.

---

## 6. Architecture

### 6.1 Rust modules (workspace crates)

Crates use a `px-` prefix (a crate named `core` would clash with Rust's own `core`). Full responsibilities and the dependency graph are in [ARCHITECTURE.md](ARCHITECTURE.md).

| Layer | Crates |
|---|---|
| Foundation | `px-core`, `px-expr`, `px-format`, `px-schema` |
| Drawing | `px-raster`, `px-model` (3D-to-pixel), `px-filters`, `px-light`, `px-fx` |
| Behavior | `px-sim`, `px-rig`, `px-pieces`, `px-script` |
| Runtime | `px-graph`, `px-nodes`, `px-packs`, `px-director`, `px-export`, `px-engine` |
| Apps | `px-editor`, `px-cli` (the `pxe` binary) |

Prefabs are not a crate: they are content in packs, loaded by `px-packs`.

### 6.2 Canvas (indexed buffers)

Each canvas is a set of parallel buffers:
- **material:** which ramp the pixel uses
- **tone:** which step of the ramp
- **depth / layer:** what is in front
- **emission:** whether the pixel glows on its own
- **object id:** used for outlines and selective effects

### 6.3 Data types flowing through the graph

| Type | Description |
|---|---|
| Canvas | Indexed buffers (above) |
| Mask | Per-pixel on/off |
| Value | Number, vector, range, palette color |
| Palette / Ramp | Ordered colors, dark to light |
| Path | Curves (cables, tails, camera paths) |
| Skeleton / Pose | Bones and their angles at a frame |
| Particles | Points with life, velocity and material |
| Light | Source with shape, color and intensity |
| Piece | Slot grid with anchors |
| Field | Per-pixel scalar (noise, distance, height) |
| Model | 3D shape (CSG/SDF tree or mesh) with material slots |
| Tileset | Tiles plus adjacency/autotile rules |

### 6.4 Cleanup passes (what replaces the artist)
1. **Pixel-perfect lines:** remove L-shaped corners on diagonals.
2. **Orphan removal:** delete stray isolated pixels.
3. **Regular steps:** curves descend in consistent runs (3-2-1-1-2-3).
4. **Selective outline (sel-out):** the outline is lighter where light hits and darker in shadow.
5. **Noise clustering:** raw noise is grouped into clean clumps.
6. **Pixel coherence:** a single pixel size everywhere, never mixed scales.

### 6.5 Lighting
- **Ambient:** a global darkness level, expressed in ramp steps.
- **Sources:** cone, point, rect (windows and screens).
- **Effect:** each pixel moves up or down its ramp; boundaries are dithered.
- **Emissive pixels** ignore darkness.
- **Color tint:** warm light shifts cold ramps toward warm ramps.
- **Simple 2D shadows** are projected from the depth buffer.
- **Dither anchoring:** dither patterns are anchored to world space, not screen space, to avoid "swimming" during camera pans.

### 6.6 Animation
1. **Modulators:** oscillators, noise, wiggle, curves, counters, connectable to any parameter.
2. **Keyframes:** on any property, with step or smooth interpolation snapped to whole pixels.
3. **Pose clips:** for skeletons.
4. **State machines:** for characters.
5. **Timing tools:** loop, delay, stagger, time remap, and animating "on 2s / on 3s".

Default rate is **12 fps**.

**Stateful exceptions:** feedback nodes, simulations and event scripts must be simulated from frame 0. The engine stores **checkpoints** every N frames so scrubbing stays fast. Perfect loops with feedback need a warm-up period.

### 6.7 Characters: three combinable paths
- **A. 2D SDF body + skeleton.** A *body plan* defines the bones, proportions and anchors. Each bone has a volume (a capsule or an ellipse). Each frame: pose → rasterize volumes → cleanup passes → shade. Best for 16–48 px characters in side view.
- **B. 3D to pixel** (the technique used by *Dead Cells*). Characters are built from 3D primitives in code, rendered at low resolution with toon shading, quantized to the palette and outlined. This gives 8 directions, walking and camera changes for free. The same path renders props in `top-down` and `isometric` views, so it arrives with those views (see [ROADMAP.md](ROADMAP.md)). Path A alone covers `front`/`side`/`back`.
- **C. Piece library.** Small text grids for the details geometry can't do well: glasses, hair, eyes, mouths, stripes. Pieces use **slots, not colors**, so they recolor for free.

### 6.8 Graph performance
- Nodes know whether they **depend on time**. Static nodes (wall, shelf, books) render once.
- Cache key: node + parameters + frame.
- Independent branches evaluate in parallel.

### 6.9 Director
- **Scene:** a place plus its objects, characters and ambience.
- **Shot:** a scene seen through a camera for a duration.
- **Sequence:** a list of shots joined by transitions.
- **Camera:** pans in whole pixels, cuts, integer zoom, automatic parallax per layer.
- **Transitions:** `cut`, `dither`, `palette-black`, `palette-swap`, `wipe`, `iris`.

### 6.10 Export
- GIF with the exact palette, lossless.
- MP4/WebM, upscaled with nearest-neighbor *before* encoding.
- APNG/WebP, PNG sequences, sprite sheets with metadata.
- **Loop solver:** finds the shortest duration in which all cycles line up.

---

## 7. Format grammar (v1)

### 7.1 Project layout
```
my-project/
  project.kdl
  scenes/
  prefabs/
  pieces/
  palettes/
  scripts/
  sequences/
  output/
```

### 7.2 KDL conventions
- **Node = a thing:** `shelf`, `light-cone`, `cat`.
- **First argument = optional id:** `character "dad"`.
- **Properties = parameters:** `x=165 levels=4`.
- **Children `{ }` = contents or detailed configuration.**
- `//` starts a comment; `/-` in front of a node disables it without deleting it.
- Names are lowercase `kebab-case`: `desk-lamp`, `window-rain`.

### 7.3 Value kinds

| Kind | Example | Meaning |
|---|---|---|
| Fixed | `x=140`, `on=#true` | Number, string or boolean |
| With unit | `"3s"` `"36f"` `"50%"` | Seconds, frames, percentage of parent |
| Random range | `"2..5"` `"3s..6s"` | Random value from the object's seed (stable) |
| Expression | `"=2*sin(t*3)"` | Starts with `=`, like a spreadsheet; evaluated per frame |
| Exposed param | `"=$rain"` | Bound to a slider in the editor |
| Reference | `"=@lamp.intensity*0.5"` | Reads another object's property |

Any property can also have keyframes (§7.7).

### 7.4 Expression language
A small math language that runs in Rust and is fast enough to evaluate per pixel.
- **Variables:** `f` (frame), `t` (seconds), `i` / `n` (index and count inside a repeater), `seed`
- **Math:** `sin cos tri saw step abs min max clamp lerp`
- **Randomness:** `rand(i)`, `noise(x, y)`, both seeded
- **Modulators:** `wiggle(amp, freq)`, `loop(value, "3s")`, `pulse("2s")`
- **Pixel art timing:** `on2(...)`, `on3(...)` change the value only every 2 or 3 frames
- Positions always round to whole pixels.

The complete function reference is in [FORMAT.md](FORMAT.md) §9.

### 7.5 Scene
```kdl
scene "night-room" palette="night-blue" seed=42 duration="12s" loop=#true {

    param rain 0.8 min=0 max=1          // shows up as a slider in the editor
    ambient darkness=2                  // shifts every ramp 2 steps darker

    layer background parallax=0.3 {
        city density=0.7 lit-windows="=0.4+wiggle(0.05,0.2)"
    }
    layer room {
        wall material="plaster-blue" baseboard=#true
        window "window" x=140 y=60 width=70 height=90 {
            window-rain strength="=$rain"
        }
        shelf x=0 levels=4 books="dense"
        desk-lamp "lamp" x=190 on=#true {
            light-cone color="warm" reach=80 intensity="=1+wiggle(0.08,6)"
        }
        tv x=120 y=200
    }
    layer characters { /* see 7.8 */ }
    layer foreground {
        book-stack x=240 y=230 count="4..7"
    }
}
```
**Properties common to every object:** `x y z anchor flip visible seed material dither-opacity`. The default `anchor` is `bottom-center`, so objects stand on the floor.

### 7.6 Repeat and scatter
```kdl
repeat n=18 layout="row" gap=0 {
    book width="2..4" height="=10+rand(i)*6" material="books-cold" tilted="=rand(i)>0.9"
}
scatter n=40 area="window" {
    light-drop size=1 brightness="=pulse('1.5s')"
}
```
**Layouts:** `row`, `column`, `stack`, `grid`, `path`, `area`.

### 7.7 Keyframes
```kdl
desk-lamp "lamp" {
    anim intensity {
        k "0s"   1
        k "5s"   1 interp="step"
        k "5.2s" 0                     // lamp goes out
        k "6s"   1 interp="smooth"
    }
}
```
**Interpolations:** `step`, `linear`, `smooth`, `ease-in`, `ease-out`, `bounce`. All of them snap to whole pixels. When the editor changes keyframes, it rewrites this block and leaves your comments intact.

### 7.8 Characters
```kdl
character "dad" body="human-adult" x=40 y=225 facing="left" {
    build height=38 bulk=0.6 posture="sitting"
    materials skin="skin-2" hair="hair-black" cloth-a="cloth-green" cloth-b="cloth-pink"
    piece "short-hair"      anchor="crown"
    piece "round-glasses"   anchor="eyes"
    piece "sweater-stripes" anchor="torso"

    machine initial="typing" {
        state typing clip="type" {
            go-to "look-screen" after="3s..6s"
        }
        state look-screen clip="lean" duration="1.5s" {
            go-to "typing"
        }
    }
}

character "kitty" body="cat" x=165 y=228 {
    materials fur="fur-black"
    pose "sitting"
    tail clip="sway" period="3s"
}
```
- **Body plan:** the skeleton, proportions and available anchors.
- **Anchors:** `crown`, `eyes`, `mouth`, `torso`, `hand-l`, `hand-r`…
- **Clips:** `breathe`, `blink`, `type`, `look`, `lean`, `walk`, `sway`…
- `breathe` and `blink` are on by default, phase-offset by seed.

### 7.9 Custom prefabs
```kdl
prefab "plant-shelf" {
    input width 60 min=20 max=200
    input plants 3 min=0 max=8

    ledge width="=$width"
    repeat n="=$plants" layout="row" {
        pot kind="small"
        plant height="6..12" leaves="3..6"
    }
}
```
From the outside, a KDL prefab and a Rust prefab look identical. In the editor, double-clicking a prefab opens its graph.

### 7.10 Pieces
```kdl
piece "round-glasses" anchor="eyes" layer="over" pivot="3,1" {
    legend "#"=outline "o"=glass "."=empty
    variant "front" """
        #.##.#
        o#..#o
        """
    variant "side" """
        ##
        o#
        """
}

piece "eye" anchor="eyes" mirror=#true {
    legend "#"=eye "-"=skin.shade "."=empty
    variant "open"   "#"
    variant "half"   "-"
    variant "closed" "-"
}
```
- **Slots:** `outline`, `skin`, `hair`, `cloth-a`, `cloth-b`, `accent`, `eye`, `glass`. A dot picks a tone on the slot's ramp (`hair.shade`, `hair.-1`); see [PALETTES.md](PALETTES.md).
- **`mirror=#true`:** write the left half; the engine mirrors it.
- **Variants:** by direction (`front`, `side`, `back`) or by state (`open`, `closed`).
- **`pivot`:** the piece pixel that aligns with the anchor.

### 7.11 Palettes
```kdl
palette "night-blue" {
    ramp plaster-blue "#0d1824" "#16273a" "#21384f" "#2e4c66" "#3f627f"
    ramp warm         "#3a1f1a" "#7a3f2a" "#c0703f" "#e8a964" "#ffd9a0"
    ramp outline      "#070d14"
    tint warm over="cold" steps=2      // how warm light tints cold ramps
}
```
Palettes that share ramp names are interchangeable, so switching `night-blue` → `sunset` changes the mood of the whole scene. Lospec `.gpl`/`.hex` files can be imported.

### 7.12 Sequences and camera
```kdl
sequence "rain-short" fps=12 {
    shot "night-room" duration="8s" {
        camera {
            k "0s" x=0  y=0
            k "8s" x=12 y=0 interp="smooth"
        }
    }
    transition "dither" duration="1s"
    shot "window-close" duration="5s" zoom=2
    transition "palette-black" duration="2s"
}
```

### 7.13 TypeScript scripts
Scripts attach to any node with `script "jump.ts"`.

| Kind | What it does | Rule |
|---|---|---|
| **Value script** | Receives time and scene, returns property values | Pure: no memory between frames |
| **Event script** | Reacts to events (`on("jumped")`), emits events, changes states | Stateful: simulated from frame 0 with checkpoints |

- **Available** through the `ctx` object and object handles: time, seed, `rand`, params, `ctx.get("lamp")`, `ctx.emit(...)`, event handlers, `obj.set(...)`, `obj.setState(...)`, `obj.play(...)`. Full API in [SCRIPTING.md](SCRIPTING.md).
- **Not available:** file system, network, real `Date`, unseeded `Math.random` (replaced automatically).
- **Editor support:** the engine generates `.d.ts` types from your scenes and prefabs, so VS Code autocompletes real ids, properties and states.
- **Per-pixel effects** belong in the expression language, not in scripts (scripts are too slow per pixel).

### 7.14 Validation and errors
Every prefab and body plan declares a schema (parameters, types and ranges) generated from Rust.
```
scenes/room.kdl:14 — unknown node "shlef". Did you mean "shelf"?
scenes/room.kdl:22 — "levels" out of range (1..8), using 4.
```
Warnings never stop rendering, so the preview keeps working while you type.

### 7.15 `project.kdl`
```kdl
project "dreams" {
    resolution 256 256
    preview-scale 4
    fps 12
    default-palette "night-blue"
    export format="gif" scale=4 folder="output"
}
```

---

## 8. Node catalog (categories)

- **Generate:** rect, polygon, ellipse, line, SDF shapes (union, subtract, smooth union), noise (Perlin, cellular, value), patterns (brick, planks, tiles, grid), dithered gradient
- **Pixel art filters:** dither (Bayer, custom pattern, error diffusion), posterize-to-ramp, sel-out outline, pixel-perfect lines, orphan removal, noise clustering, palette shift, palette replace
- **Light:** ambient, cone, point, emissive rect, 2D shadow, beam dust
- **Compose:** layers, mask, crop, integer transform (move, flip, rotate 90°), parallax
- **Repeat:** repeat, scatter, stack, distribute along a path
- **Animation:** oscillator, wiggle, curve, counter, loop, delay, stagger, time remap, on2/on3, state machine
- **Simulation:** particles, feedback, rope/chain (tails, cables, curtains)
- **Rig:** bone, IK, pose, pose clip, pose stagger, volume (capsule/ellipse), piece anchor
- **3D & generators:** 3D primitives and models (3D-to-pixel), L-systems, terrain, facades, tiles and autotiles
- **Director:** scene, shot, camera, transition, sequence
- **Script:** script node (logic), expression node (per pixel)
- **Export:** GIF, MP4/WebM, APNG/WebP, PNG, sprite sheet, tileset, parallax layers

Prefabs and body plans are not nodes of the engine: they are content in packs (see [CATALOG.md](CATALOG.md)). The full node reference is [NODES.md](NODES.md).

---

## 9. Editor UI

```
┌──────────────┬───────────────────────────────┬───────────────┐
│ LIBRARY      │                               │  INSPECTOR    │
│ prefabs      │          PREVIEW              │  parameters   │
│ pieces       │   (crisp pixels, 4x zoom)     │  ◆ animatable │
│ palettes     │                               │  seed 🎲      │
├──────────────┤  [light][depth][material][rig]│               │
│ HIERARCHY    │  debug views                  │               │
│ scene/layers │                               │               │
├──────────────┴───────────────────────────────┴───────────────┤
│ TIMELINE  ◆──◆────◆   per-object tracks · loop region          │
├───────────────────────────────────────────────────────────────┤
│ NODE GRAPH (collapsible)          │  CONSOLE (errors)          │
└───────────────────────────────────────────────────────────────┘
```

- **Preview:** integer zoom, optional pixel grid, color picker, debug views.
- **Inspector:** the ◆ toggle makes a parameter animatable; any modulator can be wired in.
- **Timeline:** per-object tracks, step or smooth keys, loop region, markers. Audio track later.
- **Hierarchy:** objects and layers of the scene.
- **Node graph:** double-click a prefab to open its group.
- **Hot reload:** save the KDL or TS in VS Code and the preview updates in under a second.

---

## 10. Pixel Composer: what we borrow, what we change

**Borrowed**
- Node graph.
- Every property animatable (◆).
- Modulator nodes: wiggler, curves, wavetable, counter.
- Timing nodes: loop, delay, stagger, rate remap.
- Groups, which become prefabs.
- Iteration nodes.
- Feedback.
- Tunnels (send and receive values without wires).
- Pixel filters: outline, dither, posterize, palette shift and replace, 2D light, glow.
- Armature: bone, IK, pose stagger, skin.
- Script nodes.
- Collection panel, which becomes the library.
- Export and sprite sheet nodes.

**Different**
- **Code/data first.** A whole room is a short KDL file, not hundreds of low-level nodes.
- **Semantic nodes** (shelf, lamp, character) on top of low-level ones.
- **Indexed pipeline** (material + tone) instead of RGBA, so lighting, palette swaps and sel-out are correct by design.
- **A dedicated character system:** skeleton + volumes + pieces.
- **A director** (scenes, shots, camera, transitions) as a first-class concept.

Reference: https://docs.pixel-composer.com/

---

## 11. Roadmap

The detailed, benchmark-driven roadmap lives in [ROADMAP.md](ROADMAP.md). It replaces the earlier room-first milestone list.

---

## 12. Decision log

| Decision | Choice |
|---|---|
| Interactive / real-time use | Later; not in v1 |
| Core language | Rust, CPU renderer |
| Scene format | KDL v2, source of truth, round-trip with the editor |
| Scripting | TypeScript in nodes only, QuickJS, deterministic sandbox |
| Character details | Text-grid pieces with slots |
| AI | None inside the engine |
| UI | Inspired by Pixel Composer |
| Language | English everywhere: format, code, errors, docs |
| Resolution | 256×256 default |
| Frame rate | 12 fps default, on2/on3 supported |
| Expressions | `=` prefix, `$param`, `@ref` |
| Time | Explicit units (`"3s"`, `"36f"`) |
| Scope | General-purpose: scene-agnostic core, content in packs, nine benchmarks + hold-out scenes |
| Styles | Rendering style presets (`moody`, `cartoon`, `one-bit`, `handheld`, `crisp-hd`, `chunky`), independent of palette |
| Views | `side` first; `perspective` helpers; `top-down` and `isometric` via 3D-to-pixel |
| Output | Videos and loops **and** game assets (sprite sheets, tilesets, parallax layers) |
| Tones | `ramp.shade` / `ramp.-1`; dash is part of a name (`skin-3`) |

## 13. Related documents
See [README.md](README.md) for the full index of documents.
