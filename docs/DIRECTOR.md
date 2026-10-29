# Director & Export (draft v1)

> Companion to [SPEC.md](SPEC.md). Covers scenes, shots, sequences, camera, transitions, timing, audio, and every export format, including game assets. Motion itself is defined in [ANIMATION.md](ANIMATION.md), and command-line usage in [CLI.md](CLI.md).

---

## 1. Concepts

| Concept | What it is | Example |
|---|---|---|
| **Scene** | A place: objects, characters, lights, palette, `view` | `night-room`, `forest-dusk`, `neon-street` |
| **Shot** | A scene seen through a camera for a duration | 8 s of the street, slow pan right |
| **Sequence** | Ordered shots joined by transitions | A 30 s short |
| **Asset set** | A non-linear export target: sprite sets, tilesets, parallax packs | `hero` sprite sheet (B4), `village` tileset (B6) |

A project can contain any mix of these. A scene can be used by many shots, sequences and asset sets.

---

## 2. Scenes

```kdl
scene "neon-street" view="perspective" palette="neon-night" seed=7 duration="10s" loop=#true {
    vanishing-point x=128 y=90
    // layers, objects, characters…
}
```

| Property | Values | Notes |
|---|---|---|
| `view` | `side`, `perspective`, `top-down`, `isometric` | See [RENDERING.md](RENDERING.md) |
| `palette` | Palette name | Can be keyframed through `palette-swap` |
| `seed` | Integer | Root of all object seeds |
| `duration` | Time | Or `auto` (from the loop solver) |
| `loop` | Bool | Enables loop validation and loop-safe defaults |
| `size` | `W H` | Defaults to the project resolution; can be larger than the camera (for scrolling) |
| `warm-up` | Time | Pre-simulation for stateful nodes |

---

## 3. Camera

| Feature | Params | Notes |
|---|---|---|
| **Position** | `x`, `y` (keyframable) | Whole pixels only |
| **Zoom** | `zoom` = any value > 0, keyframable | Shapes are drawn again at every zoom, never resampled: pixels stay crisp and on-palette, outlines stay 1 px and forms gain detail as they grow ([DECISIONS.md](engineering/DECISIONS.md) D-008). Integer zooms are plain enlargements of the same art |
| **Follow** | `follow="hero" lead=8 deadzone="16,8" smooth=0.2` | Snapped to pixels |
| **Path** | `path="dolly-1" speed=…` | Camera moves along a path |
| **Shake** | `shake amp=2 freq=12 decay="0.5s"` | Integer offsets, seeded |
| **Parallax** | Per layer: `parallax=0.3` | Layers move by `camera × parallax`, rounded per layer |
| **Bounds** | `bounds="scene"` | Camera never shows outside the scene |
| **Sub-pixel policy** | `snap="camera"` (default) or `snap="layer"` | Prevents layers jittering against each other |

```kdl
camera {
    k "0s"  x=0   y=0
    k "8s"  x=40  y=0 interp="smooth"
    shake amp=1 freq=8 when="=@thunder.flash"
}
```

**Multiple cameras:** a scene may define several named cameras (`camera "wide"`, `camera "close"`), and shots choose one.

---

## 4. Shots and sequences

> **Today (2026-10-29):** `sequence` with `shot` (`duration`, `from`, `speed`, param overrides) and the `cut`, `dither`, `black`, `white` and `wipe` transitions load, render and export as GIF (FORMAT.md appendix A.8, crate `px-director`). Not yet: named cameras, per-shot palette or camera overrides, `hold`, `iris`, `pixelate`, `slide`, `palette-swap`, markers as sequence events.

```kdl
sequence "forest-short" fps=12 {
    shot "forest-dusk" camera="wide" duration="6s"
    transition "dither" duration="1s"
    shot "forest-dusk" camera="close" duration="4s" time-offset="6s"   // same scene, continuous time
    transition "palette-black" duration="2s"
    shot "night-room" duration="8s"
}
```

| Feature | Detail |
|---|---|
| **Scene time** | `time-offset` picks where in the scene's time the shot starts. Continuous by default when the same scene repeats |
| **Overrides** | A shot can override params, palette or the camera without changing the scene |
| **Markers** | `marker "door-opens" at="3.5s"`: emits an event that scripts and state machines can react to |
| **Holds** | `hold="1s"` freezes the last frame |
| **Speed** | `time-scale=0.5` for slow motion (content still snaps to frames) |

### 4.1 Transitions
| Transition | Effect |
|---|---|
| `cut` | Instant |
| `dither` | Pixels switch from A to B following an ordered dither pattern |
| `palette-black` / `palette-white` | Steps every ramp toward `ink` (or the highest tone) and back up |
| `palette-swap` | Same frame, different palette, dithered over time |
| `wipe` | Directional, with a stepped or dithered edge |
| `iris` | Circle closes/opens on a target |
| `pixelate` | Blocks grow then resolve (integer block sizes only) |
| `slide` | Push A out with B, in whole pixels |

Transitions never blend colors. They always choose palette colors from A or B.

---

## 5. Timing, markers and audio

| Feature | Detail |
|---|---|
| **Markers** | Named points in sequence time; visible in the editor timeline |
| **Events** | Markers emit events into scenes (`on "door-opens"`) |
| **Beat grid** (later) | `tempo=90` enables the `"2b"` time unit and beat-snapped keyframes |
| **Audio track** (later) | Reference audio for timing only. The engine doesn't mix audio, but it exports an MP4 with the track muxed in |
| **Audio-reactive** (later) | `=audio.level` and `=audio.band(2)` from an analyzed track, precomputed per frame so renders stay deterministic |

---

## 6. Video and image export

| Format | Use | Notes |
|---|---|---|
| **GIF** | Loops for sharing | Exact palette, no dithering added by the encoder, optional disposal optimization |
| **APNG** | Lossless loops with more colors | |
| **WebP (animated)** | Small lossless loops | |
| **MP4 (H.264)** | Social and video | Upscaled with nearest-neighbor first; `yuv444` or high bitrate recommended to avoid color bleeding |
| **WebM (VP9)** | Web video | Lossless mode available |
| **PNG sequence** | Editing elsewhere | One PNG per frame, optionally at native resolution |
| **Single PNG** | Stills, wallpapers | Any frame |

### 6.1 Export settings
```kdl
export "rain-loop" target="night-room" {
    format "gif" scale=4
    format "mp4" scale=6 fps=24 repeat=3          // 3 loops in one video
    range from="0s" to="auto"                      // auto = loop length
}
```

| Setting | Meaning |
|---|---|
| `scale` | Integer upscale (nearest-neighbor only) |
| `fps` | Output rate. A 12 fps animation exported at 24 fps duplicates frames exactly |
| `repeat` | Repeat the loop N times in a video |
| `range` | Time range, or `auto` to use the loop solver's length |
| `crop` / `pad` | Integer crop or padding (for aspect ratios like 9:16) |
| `background` | Color, `transparent` (PNG/APNG/WebP/GIF) or a palette role |

### 6.2 Loop and length rules
- A scene with `loop=#true` exports **exactly** one loop length unless `repeat` is set.
- If the loop length exceeds `max-loop`, export stops with a report of the offending periods, unless `allow-long=#true`.
- Stateful nodes use `warm-up` before frame 0, so the first exported frame is already settled.

---

## 7. Game asset export

### 7.1 Sprite sheets
Built from a `sprite-set` (see [ANIMATION.md](ANIMATION.md) §12).

```kdl
export "hero-sheet" target="hero" {
    sheet layout="rows-by-direction" padding=1 cell="auto" pivot="bottom-center"
    image "hero.png"
    meta formats="aseprite,godot,unity,generic"
}
```

| Option | Values | Notes |
|---|---|---|
| `layout` | `rows-by-direction` (one row per direction, clips side by side), `rows-by-clip` (one row per clip × direction), `packed` (tight atlas), `strip` (one row per clip, one file per direction) | |
| `cell` | `auto` (largest frame bounds) or `W H` | All cells the same size, so engines can slice by grid |
| `trim` | Bool | Trim transparent pixels, keeping offsets in metadata |
| `padding` / `extrude` | Pixels | Prevents bleeding in game engines |
| `pivot` | `bottom-center`, `center` or anchor name | Exported per frame |
| `scale` | Integer | Usually 1 for games |
| `palette-variants` | List of palettes | One sheet per palette (team colors, seasons) |

**Metadata** (per frame): position in the sheet, size, pivot, duration in ms, clip name, direction, loop flag, root motion (dx, dy), anchor positions (`hand-r`, `weapon`, `feet`) and event markers (`footstep`, `hit-frame`).

| Format | Target |
|---|---|
| `aseprite` | Aseprite JSON (array mode, with `frameTags` per clip × direction) |
| `godot` | `SpriteFrames` resource (`.tres`) with named animations |
| `unity` | Sprite atlas JSON + slice metadata importable through a small editor script |
| `generic` | Documented plain JSON |

### 7.2 Tilesets and autotiles (B6)
```kdl
export "village-tiles" target="village-kit" {
    tileset tile=16 view="isometric" autotile="blob-47"
    terrain grass dirt water cobble
    transitions all
    meta formats="tiled,godot,ldtk"
}
```

| Option | Values |
|---|---|
| `tile` | Tile size (8, 16, 24, 32…) |
| `view` | `top-down` or `isometric` (2:1 diamond) |
| `autotile` | `none`, `edge-16` (4-bit), `blob-47` (8-bit), `wang-corner` |
| `terrain` | Terrain prefabs to generate |
| `transitions` | Which terrain pairs get transition tiles |
| `variants` | Number of seeded variations per tile (to avoid visible repetition) |
| `animated` | Animated tiles (water, torches), exported with frame durations |

Formats: **Tiled** (`.tsx` + Wang sets), **Godot** TileSet with terrain bits, **LDtk** auto-layer rules, and generic JSON with the bitmask of each tile.

Seamlessness is guaranteed by construction: tile generators sample a world-space pattern that tiles at the tile size, and dither patterns are anchored to the tile grid.

### 7.3 Parallax layer packs (B2, B3)
```kdl
export "forest-parallax" target="forest-dusk" {
    parallax-pack layers="sky,far-hills,near-hills,trees,ground" seamless-x=#true width=512
    meta formats="godot,unity,generic"
}
```
- Each layer is exported as its own PNG (or animated strip) with its parallax factor in metadata.
- `seamless-x` makes each layer wrap horizontally: generators are evaluated on a cylinder of `width` pixels.

### 7.4 Other asset kinds
| Asset | Description |
|---|---|
| **Prop sheets** | Every prefab variant on a grid (e.g. 32 seeded trees) with pivots |
| **Effect sheets** | FX clips (explosions, splashes, magic) as sprite strips |
| **Palette files** | `.gpl`, `.hex`, PNG swatch, and a palette lookup texture for shader-based palette swaps in engines |
| **Normal or depth maps** (later) | Exported from the depth buffer for dynamic lighting in game engines |

---

## 8. Batch and variant export

```kdl
batch "tree-variants" target="oak-tree" {
    vary seed from=1 to=32
    vary palette "spring" "summer" "autumn" "winter"
    output "trees/{palette}/oak-{seed}.png"
    contact-sheet "trees/overview.png" columns=8
}
```

| Feature | Detail |
|---|---|
| **Axes** | `seed`, `palette`, any `param` (`vary param="rain" values="0,0.5,1"`) |
| **Combination** | Cartesian product, or `zip=#true` to pair them |
| **Naming** | Template paths with `{seed}`, `{palette}`, `{param:name}`, `{dir}`, `{clip}` |
| **Contact sheet** | Overview grid image for picking favorites |
| **Parallel** | Each variant renders independently on all cores |
| **Reproducible** | A manifest JSON records engine version, seeds and params for every output |

---

## 9. Output layout on disk

```
output/
  rain-loop.gif
  rain-loop.mp4
  hero-sheet/
    hero.png
    hero.aseprite.json
    hero.tres
  village-tiles/
    village.png
    village.tsx
  manifest.json
```

The manifest lists every file with its source (scene, shot or asset set), seed, palette, engine version and a content hash, so any output can be regenerated bit-for-bit.
