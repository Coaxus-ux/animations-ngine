# Node Library Reference (draft v1)

> Companion to [SPEC.md](SPEC.md). This is the low-level layer, comparable to Pixel Composer's nodes. Everything in [CATALOG.md](CATALOG.md) (prefabs, body plans) is built from these nodes. Wiring syntax is defined in [FORMAT.md](FORMAT.md); rendering details are in [RENDERING.md](RENDERING.md); animation semantics are in [ANIMATION.md](ANIMATION.md).

Every node is **generic**: none of them knows about rooms, forests or any specific scene. Scene-specific knowledge lives only in packs.

---

## 0. Conventions

### 0.1 Port types
These extend SPEC §6.3 with three types that the generic generators need: `Field`, `Model` and `Tileset`.

| Type | Short | Description |
|---|---|---|
| Canvas | `C` | Indexed buffers: material, tone, depth, emission, object id |
| Mask | `M` | Per-pixel on/off |
| Field | `F` | Per-pixel scalar (noise, distance, height); becomes a Mask or tones via filters |
| Value | `V` | Number, vector, range, color reference, string, bool |
| Palette / Ramp | `P` | A palette, or a single ramp |
| Path | `Pa` | Curves and polylines, open or closed |
| Skeleton / Pose | `Sk` | Bones and their transforms at a frame |
| Particles | `Pt` | Points with life, velocity, material, size |
| Light | `L` | Light sources |
| Piece | `Pc` | Slot grid with anchors and variants |
| Model | `Mo` | 3D shape (CSG/SDF tree or mesh) with material slots |
| Tileset | `Ts` | Tiles plus adjacency/autotile rules |

### 0.2 Time column (`T`)
| Value | Meaning |
|---|---|
| `S` | Static: computed once and cached |
| `A` | Always animated |
| `I` | Inherits: animated only if an input or parameter is animated |

Any parameter of any node accepts every value kind from SPEC §7.3 (fixed, unit, range, expression, `$param`, `@ref`) and keyframes. That's why most nodes are `I`.

### 0.3 Common params (every node)
`seed` (overrides the derived seed), `enabled`, `cache` (`auto` \| `always` \| `never`), `label` (editor name).

---

## 1. Generate (2D primitives)

| Node | In | Out | Key params | T | Purpose |
|---|---|---|---|---|---|
| `rect` | — | M | `x y width height corner` | I | Axis-aligned rectangle, optional pixel-rounded corners |
| `ellipse` | — | M | `cx cy rx ry` | I | Pixel-clean ellipse, with no bulges or doubles |
| `polygon` | Pa? | M | `points` | I | Filled polygon |
| `line` | — | M | `x1 y1 x2 y2 width` | I | Pixel-perfect line (no L-corners) |
| `path-stroke` | Pa | M | `width` (const or along path), `cap` | I | Strokes a path: cables, trunks, tails, roads |
| `path` | — | Pa | `points`, `kind` (`poly` \| `bezier` \| `catmull`) | I | Builds a path |
| `fill` | — | M | — | S | Full-canvas mask |
| `gradient` | — | F | `angle`, `kind` (`linear` \| `radial`) | I | Linear or radial field |
| `text` | — | M | `text`, `font` (bitmap font), `spacing` | I | Pixel font text for signs, screens and UI |

## 2. Shapes / SDF 2D

| Node | In | Out | Key params | T | Purpose |
|---|---|---|---|---|---|
| `sdf-circle` | — | F | `c r` | I | Circle distance field |
| `sdf-box` | — | F | `c size round` | I | Rounded box |
| `sdf-capsule` | — | F | `a b r` (or `ra rb`, tapered) | I | Limbs, branches, tails |
| `sdf-triangle` | — | F | `a b c` | I | Ears, roofs, fins |
| `sdf-polygon` | Pa | F | — | I | Any closed path as an SDF |
| `sdf-union` / `sdf-subtract` / `sdf-intersect` | F… | F | — | I | Boolean combinations |
| `sdf-smooth-union` | F… | F | `k` | I | Organic blending (bodies, blobs, clouds) |
| `sdf-round` / `sdf-onion` | F | F | `r` / `thickness` | I | Inflate, or make a shell |
| `sdf-warp` | F, F | F | `amount` | I | Displace a shape by a noise field (rocks, foliage, hair) |
| `sdf-raster` | F | M, F(normal) | `threshold`, `steps` (regular-step snapping) | I | Rasterize to a clean mask; also emits a 2D normal field for shading |

## 3. 3D primitives & model (3D-to-pixel path)

| Node | In | Out | Key params | T | Purpose |
|---|---|---|---|---|---|
| `box3` `sphere3` `cylinder3` `capsule3` `cone3` `torus3` `plane3` | — | Mo | size params, `material` slot | I | 3D primitives |
| `csg` | Mo… | Mo | `op` (`union` \| `subtract` \| `intersect` \| `smooth`) | I | Combine shapes |
| `extrude` | Pa | Mo | `depth bevel` | I | 2D path to a 3D slab (signs, walls, roofs) |
| `lathe` | Pa | Mo | `segments` | I | Revolve a profile (pots, lamps, columns, bottles) |
| `transform3` | Mo | Mo | `pos rot scale` | I | Place shapes in 3D |
| `model` | Mo… | Mo | `slots`, `sockets` | I | Group shapes into a reusable model with sockets |
| `mesh-import` | — | Mo | `file` (.obj) | S | Optional import (no AI, but outside "pure code") |
| `camera3` | — | V | `view` (`side` \| `perspective` \| `top-down` \| `isometric`), `angle`, `ortho` | I | Projection for the scene view |
| `render3` | Mo, V(camera), L? | C | `toon-bands`, `snap` (pixel snapping), `outline` | I | Render at native resolution, quantize to ramps, emit depth/normal |
| `direction-set` | Mo / Sk | C[] | `directions` (4 \| 8 \| 16) | I | Render one subject from N facings (for sprite sheets) |

## 4. Procedural generators

| Node | In | Out | Key params | T | Purpose |
|---|---|---|---|---|---|
| `lsystem` | — | Pa[] | `axiom`, `rules`, `iterations`, `angle`, `length`, `jitter` | I | Branching structures: trees, bushes, coral, lightning, cracks |
| `tree` | — | Pa[], Pt(leaf points) | `kind` (`conifer` \| `broadleaf` \| `palm` \| `dead`), `height`, `spread`, `density` | I | Space-colonization tree with leaf clusters; wraps `lsystem` |
| `terrain-1d` | — | Pa | `roughness octaves height baseline` | I | Side-view hill and mountain silhouettes |
| `terrain-2d` | — | F | `roughness octaves erosion` | I | Heightmap for top-down and isometric |
| `skyline` | — | M, Pt(windows) | `density height-range roof-styles` | I | Building silhouettes with window points |
| `facade` | — | C | `floors bays`, `window-style`, `door`, `balconies`, `signs` | I | Building fronts in any view |
| `rock` | — | F | `size facets roughness` | I | Rocks, cliffs, boulders |
| `cloud` | — | F | `size puff-count softness` | I | Clouds from smooth-unioned puffs |
| `grass` | — | M | `density height lean` | I | Grass tufts and blades (responds to `wind-field`) |
| `maze` / `dungeon` | — | M | `cells rooms corridors` | S | Layouts for games |
| `wfc` | Ts | V(grid) | `width height`, `constraints` | S | Wave Function Collapse layout from a tileset |
| `autotile` | M, Ts | C | `mode` (`16` \| `47` blob) | I | Mask → correctly joined tiles (terrain edges, walls, roads) |
| `tileset-gen` | C(s) | Ts | `tile-size`, `mode` | S | Build an autotile tileset procedurally from a material + edge rules |

## 5. Noise & patterns

| Node | Out | Key params | T | Purpose |
|---|---|---|---|---|
| `noise-perlin` / `noise-simplex` / `noise-value` | F | `scale octaves persistence`, `z` (time slice) | I | Base noises; animate `z` for evolving noise |
| `noise-cellular` / `voronoi` | F | `scale jitter`, `metric` | I | Cells, stones, scales, cracks |
| `fbm` | F | `octaves lacunarity gain` | I | Fractal sum of any noise |
| `domain-warp` | F | `amount` | I | Warp a field by another (marble, water, smoke) |
| `pattern-brick` / `pattern-plank` / `pattern-tile` / `pattern-hex` / `pattern-stripes` / `pattern-checker` | M | `size offset gap` | I | Regular materials |
| `pattern-bayer` | F | `size` (2/4/8) | S | Dither matrix as a field |
| `stars` | M | `density twinkle` | I | Star fields |
| `loopable-noise` | F | `period` | A | Noise that loops perfectly over a duration (for perfect loops) |

## 6. Field ops

| Node | In | Out | Key params | T | Purpose |
|---|---|---|---|---|---|
| `threshold` | F | M | `level`, `dither-band` | I | Field → mask, with an optional dithered edge band |
| `remap` | F | F | `from to curve` | I | Rescale and curve |
| `quantize-tones` | F | V(tone map) | `steps` | I | Field → tone indices (form shading, height shading) |
| `combine` | F… | F | `op` (`add` \| `mul` \| `min` \| `max` \| `lerp`) | I | Field math |
| `distance` | M | F | `max` | I | Distance from a mask edge (glows, rims, shorelines) |
| `normal-from-height` | F | F(normal) | `strength` | I | Bumps for lighting |

## 7. Pixel art filters

| Node | In | Out | Key params | T | Purpose |
|---|---|---|---|---|---|
| `dither` | C/F | C | `mode` (`bayer` \| `pattern` \| `error`), `anchor` (`world` \| `screen`) | I | Dither between adjacent tones |
| `posterize-ramp` | C | C | `steps` | I | Snap tones to fewer steps |
| `sel-out` | C | C | `light-dir` | I | Selective outline (lighter on the lit side) |
| `outline` | C/M | C | `mode` (`outer` \| `inner`), `corners` | I | Plain outline |
| `pixel-perfect` | M/C | same | — | I | Remove L-corners on diagonals |
| `orphan-removal` | M/C | same | `min-size` | I | Remove isolated pixels and specks |
| `cluster` | M/C | same | `min-cluster` | I | Turn noisy speckle into clean clumps |
| `regular-steps` | M | M | `tolerance` | I | Even out curve steps (3-2-1-1-2-3) |
| `rim-light` | C | C | `side width` | I | Highlight one edge of a form |
| `hard-shadow` | C | C | `offset`, `ramp` | I | Pixel drop shadow |
| `scale-int` | C | C | `factor` (integer, up or down) | I | Nearest-neighbor scaling |
| `rotsprite` | C | C | `angle` | I | Best-effort small-angle rotation; warns above ±15° |

## 8. Color / palette

| Node | In | Out | Key params | T | Purpose |
|---|---|---|---|---|---|
| `palette` | — | P | `name` | S | Load a palette (KDL, .gpl, .hex) |
| `ramp-gen` | — | P | `base steps hue-shift contrast saturation` | S | Ramp from one color (see PALETTES §5) |
| `material` | M, F? | C | `ramp`, `tone` (const or a field via `quantize-tones`), `depth`, `emissive` | I | Turn a mask into canvas pixels |
| `shade-normal` | C, F(normal) | C | `light-dir`, `bands` | I | Form shading from SDF or 3D normals |
| `palette-swap` | C | C | `from to` | I | Swap palettes by role |
| `tone-shift` | C | C | `steps`, `mask` | I | Darken or brighten by ramp steps |
| `ramp-remap` | C | C | `map` (role → role) | I | Recolor (team colors, seasons) |
| `palette-fade` | C | C | `target` (`black` \| `white` \| palette), `progress` | I | Palette transitions |
| `quantize-rgb` | image | C | `palette`, `dither` | S | Bring an external RGB image into the indexed world (references, imports) |

## 9. Light

| Node | In | Out | Key params | T | Purpose |
|---|---|---|---|---|---|
| `ambient` | — | L | `darkness` (tone steps), `color` | I | Global light level |
| `light-point` | — | L | `pos color intensity radius falloff` | I | Bulbs, fire, fireflies |
| `light-cone` | — | L | `pos angle spread reach color` | I | Lamps, headlights, flashlights |
| `light-rect` | — | L | `rect color intensity` | I | Windows, screens, signs |
| `light-directional` | — | L | `angle color intensity` | I | Sun or moon over the whole scene |
| `light-apply` | C, L… | C | `tint-at`, `dither-edges` | I | Apply lights: tone steps + twins (PALETTES §4) |
| `shadow-2d` | C, L | C | `softness` (dither band) | I | Shadows cast using depth |
| `god-rays` | L, M(occluders) | C | `length density dust` | I | Light shafts (forest canopy, underwater, windows) |
| `glow` | C | C | `radius`, `ramp` | I | Dithered halo around emissive pixels (neon, screens) |
| `reflection` | C, M(surface) | C | `axis`, `ripple`, `fade` | I | Mirror into puddles, water, glossy floors |
| `caustics` | — | F | `scale speed` | A | Underwater light patterns (loopable) |

## 10. Compose / transform

| Node | In | Out | Key params | T | Purpose |
|---|---|---|---|---|---|
| `stack` | C… | C | `order` | I | Layer canvases by depth |
| `layer` | C… | C | `parallax z name` | I | Scene layer, with parallax |
| `mask-apply` / `clip` | C, M | C | — | I | Restrict to a mask (window views, screens) |
| `crop` | C | C | `rect` | I | Cut |
| `translate` | C | C | `dx dy` (integer) | I | Move by whole pixels |
| `flip` / `rotate-90` | C | C | `axis` / `turns` | I | Lossless transforms |
| `wrap-scroll` | C | C | `speed`, `axis` | A | Endless scrolling (clouds, water, backgrounds) |
| `socket-place` | C, V(socket) | C | `at` | I | Place onto a socket of another object |
| `perspective-place` | C, V(camera) | C | `depth`, `scale-steps` | I | Place 2D sprites on a perspective ground with discrete scale steps |
| `iso-place` | C, V(grid) | C | `tile-x tile-y height` | I | Place on an isometric grid with correct sorting |

## 11. Repeat / iterate

| Node | In | Out | Key params | T | Purpose |
|---|---|---|---|---|---|
| `repeat` | graph | C | `n layout gap` | I | Repeat a subgraph with `i`/`n` (SPEC §7.6) |
| `scatter` | graph, M(area) | C | `n`, `mode` (`random` \| `poisson`), `min-dist` | I | Seeded placement inside an area |
| `distribute-path` | graph, Pa | C | `spacing align` | I | Place along a curve (fences, lamps, footprints) |
| `grid-repeat` | graph | C | `cols rows cell` | I | Grids (windows, tiles, shelves) |
| `for-each` | V[] / Pt | C | — | I | Run a subgraph per item or per particle |
| `pick` | V[] | V | `mode` (`random` \| `weighted` \| `cycle`) | I | Seeded choice from options |
| `variations` | graph | C[] | `count` | I | Same graph with N seeds (contact sheets, variant sprites) |

## 12. Animation / modulators / timing
See [ANIMATION.md](ANIMATION.md) for the semantics.

| Node | Out | Key params | T | Purpose |
|---|---|---|---|---|
| `time` | V | `unit` (`f` \| `s`) | A | The clock |
| `oscillator` | V | `shape` (`sin` \| `tri` \| `saw` \| `square`), `period amp phase` | A | Periodic motion |
| `wiggle` | V | `amp freq`, `loopable` | A | Organic jitter (flicker, sway) |
| `noise-1d` | V | `scale octaves` | A | Smooth random motion |
| `curve` | V | `points interp` | I | Hand-defined response curve |
| `counter` | V | `start step every` | A | Discrete steps (frame cycling) |
| `pulse` | V | `period duty` | A | On/off blinks |
| `keyframes` | V | `keys interp` | A | Keyframe track |
| `hold` | V | `every` (2 or 3 frames: on2/on3) | I | Pixel art timing |
| `loop` / `delay` / `stagger` / `rate-remap` | V | `period` / `by` / `step` / `curve` | A | Timing tools (from Pixel Composer) |
| `ease` | V | `kind` | I | Easing on any value |
| `state-machine` | V(state) | `states transitions initial` | A | Behavior for any object |
| `event` | V | `name`, `when` | A | Triggers for state machines and scripts |

## 13. Simulation
All simulations are deterministic (fixed timestep, seeded) and checkpointed (SPEC §6.6).

| Node | In | Out | Key params | T | Purpose |
|---|---|---|---|---|---|
| `particles` | V(emitter), M?(colliders), F?(forces) | Pt | `rate life speed spread gravity drag` | A | Rain, snow, sparks, dust, fireflies, bubbles |
| `particle-render` | Pt | C | `shape` (`pixel` \| `streak` \| `piece`), `ramp` | A | Draw particles |
| `boids` | — | Pt | `count cohesion alignment separation bounds` | A | Flocks of birds, schools of fish, insects |
| `rope` | V(anchors) | Pa | `segments stiffness gravity` | A | Tails, cables, hair strands, kelp, vines |
| `cloth` | V(pins) | M/Pa | `cols rows stiffness` | A | Curtains, flags, capes |
| `wind-field` | — | F | `strength gusts direction` | A | Global wind; plants, hair, cloth and rope read it |
| `water-surface` | — | Pa | `springs tension damping`, `disturb` | A | Side-view water line with waves and splashes |
| `ripples` | — | F | `damping`, `drops` | A | Top-down wave field (puddles, ponds) |
| `feedback` | C | C | `decay` | A | Use the previous frame (trails, drips, smoke) |

## 14. Rig
See [ANIMATION.md](ANIMATION.md) and CATALOG Part 3.

| Node | In | Out | Key params | T | Purpose |
|---|---|---|---|---|---|
| `skeleton` | — | Sk | `body` (plan or template), `build` | S | Build a skeleton from a body plan |
| `pose` | Sk | Sk | angles per bone, `posture` | I | Static pose |
| `clip-player` | Sk | Sk | `clip speed channel` | A | Play a clip on a channel |
| `channel-mix` | Sk… | Sk | `priorities` | I | Layer clips (posture < body < arms < head < face) |
| `ik-2bone` / `ik-chain` | Sk, V(target) | Sk | `bend` | I | Reach targets (hands to keys, feet to ground) |
| `look-at` | Sk, V(target) | Sk | `limit` | I | Head and eye aim |
| `spring-bones` | Sk | Sk | `stiffness damping` | A | Secondary motion (ears, tails, hair, antennae) |
| `foot-lock` | Sk | Sk | `ground` | A | Keeps feet planted while walking |
| `volume-raster` | Sk | C | `facing outfit hair` | I | Rasterize SDF volumes into a body (path A) |
| `rig-3d` | Sk | Mo | — | I | Drive a 3D model from a skeleton (path B) |
| `piece-attach` | C, Pc, Sk | C | `anchor variant slot-map` | I | Attach pieces to anchors |

## 15. Director
See [DIRECTOR.md](DIRECTOR.md).

| Node | Out | Key params | T | Purpose |
|---|---|---|---|---|
| `scene` | C | `palette seed duration view` | I | Root of a scene |
| `camera` | V | `x y zoom` (integer), `follow` | I | 2D camera |
| `shot` | C | `scene duration camera` | A | A scene through a camera |
| `transition` | C | `kind duration` | A | Between shots |
| `sequence` | C | `fps` | A | Ordered shots |
| `marker` | V | `name at` | S | Named times for scripts and sync |

## 16. Script / expression
See [SCRIPTING.md](SCRIPTING.md).

| Node | In | Out | Key params | T | Purpose |
|---|---|---|---|---|---|
| `expr` | any | F / V | `code` | I | Compiled math expression; per value or per pixel |
| `script-value` | any | V… | `file` (.ts) | I | Pure TS function of time and scene |
| `script-event` | events | events, V… | `file` (.ts) | A | Stateful TS behavior, checkpointed |
| `param` | — | V | `default min max label` | S | Exposed parameter (slider in the editor) |

## 17. Export
See [DIRECTOR.md](DIRECTOR.md) and [CLI.md](CLI.md).

| Node | In | Key params | Purpose |
|---|---|---|---|
| `export-gif` | C | `scale loop` | Lossless palette GIF |
| `export-video` | C | `format` (`mp4` \| `webm`), `scale crf` | Video via ffmpeg (nearest-neighbor upscale first) |
| `export-png` | C | `frames` (one or a range) | Stills and PNG sequences |
| `export-sprite-sheet` | C[] | `layout` (grid/packed), `meta` (`json` \| `aseprite` \| `godot` \| `unity`) | Character and animation sheets, all directions |
| `export-tileset` | Ts | `meta` (`tiled` \| `ldtk` \| `godot`) | Tilesets with autotile rules |
| `export-layers` | C | `split` (`layer` \| `depth`) | Separate parallax layers for games |
| `export-palette` | P | `format` (`gpl` \| `hex` \| `png`) | Share palettes |

## 18. Utility

| Node | Purpose |
|---|---|
| `group` / `group-input` / `group-output` | Encapsulate a subgraph. **This is what a prefab is** |
| `tunnel-in` / `tunnel-out` | Send and receive values without wires (from Pixel Composer) |
| `switch` | Choose an input by value or state (`on`/`off`, variants, views) |
| `cache` | Force caching of a subgraph (expensive static parts) |
| `debug-view` | Show depth, light, material or normals in the editor |
| `seed` | Re-seed a subgraph |
| `reroute` / `note` | Graph tidiness |

---

## 19. Prefabs are groups of nodes

A prefab is a `group` with declared inputs (its params), outputs (a Canvas and/or a Model), slots, sockets, states and emitters (CATALOG Part 1). A prefab written in Rust is the same thing, just compiled. Opening a prefab in the editor shows its graph.

### Example: a generic `tree` (nature pack)
It works in any scene, view or palette, with no scene-specific code.

```
 param height ─┐
 param kind ───┼─► tree ──► Pa[] branches ──► path-stroke (width tapers) ─┐
 param density ┘      │                                                   │
                      └─► Pt leaf points ─► for-each: sdf-circle ─►       │
                                            sdf-smooth-union ─►           │
                                            sdf-warp(noise) ─► sdf-raster ┤
 wind-field ─────────────────────────────► (offsets leaf points) ─────────┘
                                                                          ▼
          material(trunk: wood) + material(leaves: foliage) ─► shade-normal ─► cluster ─► sel-out ─► out
```

```kdl
prefab "tree" pack="nature" {
    input height 60 min=10 max=200
    input kind "broadleaf"
    input density 0.7 min=0 max=1

    tree "shape" kind="=$kind" height="=$height" density="=$density"
    path-stroke "trunk" path="@shape.branches" width="=lerp(4,1,u)"   // u = 0..1 along the branch
    sdf-raster "canopy" {
        sdf-warp amount=2 {
            sdf-smooth-union k=3 {
                for-each "@shape.leaves" { sdf-circle r="3..6" }
            }
            noise-simplex scale=0.2
        }
    }
    material trunk="@trunk" ramp="wood"
    material canopy="@canopy" ramp="foliage" tone="=quantize(normal.y, 5)"
    sway source="wind-field" amount=1 targets="canopy"
    output { cluster; sel-out }
}
```

The same graph gives oaks, pines or dead trees (`kind`), dusk or day (palette) and still or windy (`wind-field`), across benchmarks B2, B3 (street trees) and B6 (isometric, with the `model` variant).
