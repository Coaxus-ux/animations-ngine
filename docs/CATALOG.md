# Prefab Catalog & Body Plans (draft v1)

> Companion to [SPEC.md](SPEC.md). See also [PALETTES.md](PALETTES.md), [FORMAT.md](FORMAT.md), [PIECES.md](PIECES.md), [RENDERING.md](RENDERING.md), [ANIMATION.md](ANIMATION.md) and [BENCHMARKS.md](BENCHMARKS.md).
> All sizes are in pixels at the default 256×256 resolution unless stated otherwise.

Nothing in this document is engine code. Every prefab and body plan here is **content**, written in the same KDL any user can write, and shipped in **packs**. The engine core knows nothing about rooms, trees or cats.

---

## Part 0 — Packs

| Pack | Scope | Depends on | Typical views |
|---|---|---|---|
| `core-kit` | Generic, scene-agnostic building blocks: shapes, 3D primitives, plant/tree generators, terrain, water, rocks, facades, tiles, text, ropes, particles, sky | — | all |
| `interior` | Rooms, furniture, books, electronics, lamps, decor | `core-kit` | side, perspective, isometric |
| `nature` | Trees, bushes, grass, flowers, rocks, rivers, fog, fireflies | `core-kit` | all |
| `urban` | Facades, streets, sidewalks, street furniture, neon, vehicles | `core-kit`, `creatures` | side, perspective, isometric |
| `water` | Ocean, shore, seabed, kelp, coral, bubbles, caustics, light rays | `core-kit`, `nature` | side, top-down |
| `creatures` | Body plan templates and instances (humans, animals, vehicles) | `core-kit` | all |

### 0.1 Using packs
```kdl
// project.kdl
project "dreams" {
    use "core-kit" "interior" "nature"
    use "./packs/my-castle"            // a local pack
}
```
- **Names:** prefabs are addressed bare (`tree`) when unique, or qualified (`nature/tree`) when two packs share a name.
- **Override order:** the project's own `prefabs/` folder wins over local packs, and local packs win over shipped packs.
- **Lazy loading:** a pack costs nothing until one of its prefabs is used.

---

## Part 1 — The prefab contract

Every prefab, whether built in Rust or written in KDL, declares the same eight things. The editor and the `.d.ts` generator read them from the prefab's schema.

| Part | What it is | Example |
|---|---|---|
| **Params** | Typed parameters with defaults and ranges | `levels` int 1..8, default 4 |
| **Material slots** | Named surfaces, each defaulting to a palette **role** | `frame` → `wood` |
| **Sockets** | Named attachment points for other objects | `top`, `level-2`, `screen` |
| **States** | Discrete modes, animatable or set by state machines | `on` / `off` |
| **Emitters** | Lights, emissive regions or particle emitters the prefab owns | a lamp owns a `light-cone` |
| **Time dependence** | `static` (rendered once, cached) or `animated` | a rock is static, a river is animated |
| **Views** | Which scene views the prefab supports | `side perspective` |
| **Construction path** | How it is built (below) | `shape` |

### 1.1 Construction paths

| Path | Built from | Views | Best for |
|---|---|---|---|
| `shape` | 2D primitives and SDF shapes, through the cleanup passes | `side`; `perspective` via helpers; `top-down` / `isometric` as **billboards** | Flat, front-facing objects; the classic pixel art look |
| `model` | 3D primitives (box, cylinder, sphere, cone, capsule, wedge) combined with CSG, rendered to pixels (toon steps, palette quantization, outline) | all | Objects that need several angles, isometric or top-down scenes, rotation |
| `composite` | Other prefabs arranged by rules | union of its children's views | Rooms, streets, villages, forests |

A prefab may ship **both** a `shape` and a `model` version. The engine picks the one that matches the scene's view, so the same scene file can render flat or isometric when every prefab in it supports both.

### 1.2 Views

| View | Projection | Status |
|---|---|---|
| `side` | Flat orthographic front/side, with layers and parallax | v1 |
| `perspective` | 2D helpers for one- and two-point perspective (vanishing points, converging guides, scale by depth) | v1.x |
| `top-down` | Orthographic from above (3/4 top-down supported) | via `model` path |
| `isometric` | 2:1 pixel isometric | via `model` path |

`shape` prefabs placed in a `top-down` or `isometric` scene render as **billboards**: upright sprites sorted by depth, as in classic isometric games. Trees, signs and characters usually work this way.

### 1.3 Common params (all prefabs)
These come on top of the common object properties `x y z anchor flip visible seed material dither-opacity`.

| Param | Type | Default | Meaning |
|---|---|---|---|
| `detail` | 0..1 | 0.5 | How many small features are generated |
| `wear` | 0..1 | 0.2 | Age and mess: tilt, gaps, cracks, scuffs, moss |
| `outline` | `none` \| `sel-out` \| `full` | `sel-out` | Outline style |
| `shade` | `flat` \| `soft` \| `dither` | `dither` | How form shading is rendered |
| `scale` | int | 1 | Integer size multiplier (keeps the pixel grid) |

### 1.4 Sockets and placement

| Socket kind | Shape | Typical use |
|---|---|---|
| `point` | One pixel | A seat, a hook, a chimney top |
| `line` | Horizontal span | A tabletop, shelf level, branch, sidewalk |
| `area` | Rectangle | A wall, window view, screen, facade bay |
| `surface` | A face of a `model` prefab | The top of a crate in isometric |
| `ground` | The scene's ground line (side) or ground plane (top-down/isometric) | Default target for placing things |

```kdl
mug on="desk.top" at="80%"
character "kid" on="tv.top"
bird on="tree.branch-3"
lamp-post on="sidewalk" at="30%"
```
- On a `line` socket without `at`, objects are laid out left to right without overlapping.
- With no `on`, an object stands on `ground`.

### 1.5 Material slots
Slots name surfaces, not colors, and default to standard **roles** (see [PALETTES.md](PALETTES.md)):
```kdl
shelf materials frame="wood-dark" back="wall"
```
Each pack lists the roles it uses. Every role is defined in [PALETTES.md](PALETTES.md) §3, with a fallback.

---

## Part 2 — `core-kit` (generic building blocks)

Every other pack is built from these. They are deliberately abstract: none of them knows what scene it is in.

### 2.1 Shapes (`shape` path)

| Prefab | Key params | Sockets | Notes |
|---|---|---|---|
| `box` | `width`, `height`, `side-depth`, `top-depth`, `bevel` | `top`, `front` | Generic solid; `side-depth`/`top-depth` fake a 3D face in `side` view (ignored in other views, where the view provides real depth) |
| `panel` | `width`, `height`, `inset`, `divisions` (`"2x3"`), `trim` | `area`, per-cell `cell-n` | Doors, cabinets, windows, facade bays, screens |
| `cylinder` | `width`, `height`, `cap` (`flat` \| `round`), `ellipse-depth` | `top` | Mugs, pots, barrels, pillars, trunks |
| `container` | `width`, `height`, `rows`, `cols`, `frame-thickness`, `back` | `cell-r-c`, `row-n`, `top` | Generic compartments: shelves, crates, cubbies, shop displays |
| `blob` | `points` (metaball centers), `radius`, `smooth`, `noise` | `top` | Organic masses: bushes, clouds, coral, hair mass, rocks |
| `strip` | `length`, `thickness`, `path`, `pattern` | — | Edges, trims, borders, fences rails |

### 2.2 Model kit (`model` path)

| Prefab | Key params |
|---|---|
| `m-box`, `m-cylinder`, `m-sphere`, `m-cone`, `m-capsule`, `m-wedge`, `m-torus` | Size, position, rotation (any angle, since it's 3D), material slot |
| `m-group` | Children plus CSG operations: `union`, `subtract`, `intersect`, `smooth-union` |
| `model` | Wraps an `m-group` as a prefab: toon steps (2–4), outline, light response, per-view overrides |

```kdl
prefab "crate" path="model" views="side top-down isometric" {
    input size 12 min=6 max=32
    m-group {
        m-box size="=$size" material="wood"
        m-box size="=$size*0.9" y=1 subtract=#true      // carved top
    }
}
```

### 2.3 Plants and trees

#### `lsystem`
A general L-system turtle renderer; most vegetation is a preset of it.

| Param | Type | Default | Notes |
|---|---|---|---|
| `axiom` | string | `"F"` | |
| `rules` | map | — | `F="F[+F]F[-F]F"` |
| `iterations` | 1..8 | 4 | |
| `angle` | deg | 25 | Randomized by `angle-jitter` |
| `step` | int | 3 | Segment length in px |
| `thickness` | range | `"3..1"` | Tapers from root to tips |
| `leaf` | shape \| piece \| `none` | `none` | Drawn at tips |
| `wind` | 0..1 | 0.2 | Sway amount; tips move more than the base |

- **Slots:** `stem` → `wood`, `leaf` → `foliage`
- **Presets:** `fern`, `weed`, `vine`, `coral-branch`, `crack`, `lightning`

#### `tree-gen`
| Param | Type | Default | Notes |
|---|---|---|---|
| `height` | range | `"40..70"` | |
| `trunk` | `straight` \| `bent` \| `forked` \| `twisted` | `straight` | |
| `crown` | `round` \| `conical` \| `columnar` \| `palm` \| `willow` \| `bare` | `round` | |
| `crown-size` | 0..1 | 0.6 | Relative to height |
| `clusters` | range | `"5..9"` | Leaf clumps (each one is a `blob`) |
| `density` | 0..1 | 0.7 | Gaps in the canopy |
| `wind` | 0..1 | 0.2 | |
| `season` | `spring` \| `summer` \| `autumn` \| `winter` | `summer` | Picks leaf roles and density |

- **Slots:** `bark`, `leaves` → `foliage`
- **Sockets:** `branch-1` … `branch-n` (line), `crown` (area), `base` (point)
- **Views:** `shape` (side, billboard) and `model` (cone/sphere crowns for isometric)

### 2.4 Terrain

| Prefab | Key params | Notes |
|---|---|---|
| `ground` | `surface` (`grass` \| `dirt` \| `sand` \| `snow` \| `stone` \| `asphalt`), `height`, `edge` (`flat` \| `bumpy` \| `tufts`), `roughness` | Side-view ground strip; defines the `ground` socket |
| `hills` | `layers` 1..5, `height` range, `roughness`, `fog` 0..1, `parallax` (per layer, auto) | Layered silhouettes; farther layers lighter and foggier |
| `mountains` | Same as `hills`, plus `peaks`, `snow-line` | Sharper ridges |
| `terrain-map` | `size`, `noise`, `levels` (water, sand, grass, rock, snow), `seed` | Height map for `top-down`/`isometric`; feeds `tileset-gen` |

### 2.5 Water

#### `water-surface`
| Param | Type | Default | Notes |
|---|---|---|---|
| `level` | int | — | Y of the surface (side view) |
| `waves` | 0..1 | 0.3 | Amplitude |
| `wavelength` | int | 24 | |
| `speed` | 0..1 | 0.3 | |
| `reflection` | 0..1 | 0.5 | Mirrors the scene above, with ripple distortion |
| `transparency` | 0..1 | 0.4 | How much of what is below shows through |
| `depth-tint` | 0..1 | 0.5 | Darkens and tints with depth |
| `foam` | 0..1 | 0.1 | Foam line at the surface and at objects crossing it |

- **Slots:** `water`, `foam` → `paper.hi`
- **Modes:** `side` (horizontal surface line), `top-down` (animated surface tiles), `puddle` (small reflective area on a ground plane)

### 2.6 Rocks and cliffs

| Prefab | Key params |
|---|---|
| `rock-gen` | `size`, `facets` 3..9, `roundness`, `cracks` 0..1, `moss` 0..1, `half-buried` |
| `cliff` | `height`, `width`, `strata` (layer count), `overhang`, `ledges`, `moss` |

Slots: `stone`, `moss` → `foliage`. Both have `shape` and `model` versions.

### 2.7 Buildings

#### `facade-gen`
| Param | Type | Default | Notes |
|---|---|---|---|
| `floors` | 1..20 | 4 | |
| `bays` | 1..12 | 3 | Window columns |
| `window` | `square` \| `tall` \| `arched` \| `shop` \| `none` | `square` | Per floor overrides allowed |
| `ground-floor` | `door` \| `shop` \| `garage` \| `plain` | `door` | |
| `roof` | `flat` \| `gable` \| `hip` \| `mansard` \| `dome` | `flat` | |
| `material` | `brick` \| `plaster` \| `wood` \| `stone` \| `concrete` \| `metal` | `brick` | |
| `balconies` | 0..1 | 0 | Chance per bay |
| `lit-windows` | 0..1 | 0.3 | Seeded; animatable |
| `ornament` | 0..1 | 0.3 | Cornices, sills, pipes, AC units |

- **Sockets:** `bay-f-b` (area per floor/bay), `roof` (line), `chimney` (point, when the roof has one), `sign` (area above ground floor), `door` (point)
- **Emitters:** lit windows (emissive), optional interior light spill
- **Views:** `shape` (side, perspective via helper) and `model` (isometric, top-down)

### 2.8 Tiles

#### `tileset-gen`
| Param | Type | Default | Notes |
|---|---|---|---|
| `tile` | 8 \| 16 \| 24 \| 32 | 16 | |
| `projection` | `square` \| `isometric` \| `side` | `square` | Side = platformer tiles |
| `terrains` | list | `"grass dirt water"` | Each terrain is a generator preset |
| `autotile` | `none` \| `16` \| `47` (blob) | `47` | Edge/corner rule set |
| `transitions` | bool | true | Generates blend tiles between terrains |
| `variants` | 1..8 | 3 | Random variants per tile to break repetition |
| `animated` | list | `"water"` | Terrains that get animated frames |

#### `tilemap`
Places tiles from a grid written as text, from a `terrain-map`, or from noise.
```kdl
tilemap tileset="village" source="terrain-map" size="32x32"
tilemap tileset="cave" grid="""
    ####....####
    #..........#
    """
```
`path-tiles from=… to=… terrain=…` paints a path of one terrain between two sockets (shortest route on the tile grid).

Exports to Tiled-compatible JSON (see [DIRECTOR.md](DIRECTOR.md)).

### 2.9 Text and signs

| Prefab | Key params | Notes |
|---|---|---|
| `text` | words first, `font` (`3x5` \| `5x7` \| `tall` \| custom piece font), `align`, `spacing`, `bold`, `italic`, inline `[b]` `[i]` `[size=N]` | Built-in pixel fonts; fonts are piece sets. Today: `5x7` only (FORMAT.md A.7) |
| `sign` | `content`, `shape` (`board` \| `plaque` \| `banner` \| `hanging`), `border` | Wood, metal, paper |
| `neon-text` | `content`, `font`, `tube` (1–2 px), `flicker` 0..1, `broken-letters` list, `glow` | Emissive; casts colored light |

### 2.10 Ropes and chains

#### `rope`
| Param | Type | Default | Notes |
|---|---|---|---|
| `from`, `to` | socket or point | — | Ends; either can be free |
| `segments` | int | auto | |
| `sag` | 0..1 | 0.3 | |
| `thickness` | 1..3 | 1 | |
| `stiffness` | 0..1 | 0.2 | |
| `sway` | 0..1 | 0.1 | Wind response (simulated, checkpointed) |
| `beads` | shape \| piece | none | Bulbs, leaves, flags along the rope |

Presets: `cable`, `power-line`, `vine`, `string-lights`, `bunting`, `kelp`, `chain`, `tail` (used by body plans).

### 2.11 Particles

#### `emitter`
| Param | Type | Notes |
|---|---|---|
| `shape` | `point` \| `line` \| `area` \| `socket` | Where particles spawn |
| `rate` | per second | |
| `life` | range | |
| `velocity`, `gravity`, `wind`, `drag` | vectors / numbers | |
| `size` | range or curve over life | Integer pixels |
| `material` | role | Tone may change over life (`fade`) |
| `emissive` | bool | |
| `collide` | `none` \| `ground` \| `surfaces` | With a `splash` sub-emitter on hit |
| `layer` | name | Particles can live in front of or behind objects |

Presets: `rain`, `snow`, `steam`, `smoke`, `dust`, `fireflies`, `bubbles`, `sparks`, `embers`, `falling-leaves`, `petals`, `splash`.

#### `flock`
Boids-style group motion with a seed: `count`, `bounds`, `cohesion`, `separation`, `alignment`, `speed`, `member` (any prefab or body plan instance). Used for fish schools, bird flocks and insect swarms. Simulated from frame 0 with checkpoints.

#### Effects (`vfx`)
One-shot or looping game and film effects. They are built only from generic nodes: path sweeps, SDF shapes, emitters, and tone fades over life, using emissive ramps for glow. Every effect is a clip, so it can play inside a scene, be triggered by a script or cue, or be exported as an effect sheet ([DIRECTOR.md](DIRECTOR.md) §7.4).

| Prefab | Key params | Notes |
|---|---|---|
| `slash-trail` | `arc` (deg), `radius`, `width`, `frames`, `ramp` | Follows a weapon socket's path over the attack clip; smear frames like hand-drawn arcs |
| `impact` | `size`, `spikes`, `frames`, `sparks` (0..1) | Star burst + sparks on hit |
| `dust-puff` | `size`, `direction`, `frames` | Landing, dashing, footsteps |
| `explosion` | `size`, `stages` (flash, fireball, smoke), `debris` | Dithered fireball cooling down its ramp |
| `fire` | `height`, `width`, `intensity`, `loop` | Flame tongues from noise + rising embers; candles to bonfires |
| `magic-burst` | `shape` (`ring` \| `star` \| `spiral`), `particles`, `ramp` | Spell casts, pickups, portals |
| `shockwave` | `radius`, `thickness`, `frames` | Expanding dithered ring |
| `projectile-trail` | `length`, `fade`, `sparkle` | Arrows, fireballs, bullets |

### 2.12 Sky

| Prefab | Key params |
|---|---|
| `sky` | `kind` (`night` \| `dusk` \| `dawn` \| `day` \| `overcast`), `gradient-steps`, `stars` 0..1, `aurora` 0..1 |
| `clouds` | `layers`, `coverage`, `kind` (`cumulus` \| `stratus` \| `wisps`), `drift`, `lit-from` (sun/moon direction) |
| `celestial` | `kind` (`sun` \| `moon` \| `planet`), `phase`, `size`, `glow` |

---

## Part 3 — `interior`

Built on `core-kit`. Roles used: `wall`, `wall-side`, `floor`, `ceiling`, `baseboard`, `wood`, `wood-dark`, `metal`, `plastic`, `fabric`, `paper`, `glass`, `ceramic`, `books-*`, `warm`, `screen`.

| Prefab | Built on | Key params | Sockets | Emitters | Views |
|---|---|---|---|---|---|
| `room` | composite | `floor-height`, `ceiling`, `corner` (`none` \| `left` \| `right`), `wall-texture`, `floor-texture`, `baseboard` | `wall`, `side-wall`, `floor`, `ceiling` | — | side, perspective, iso |
| `window` | `panel` | `width`, `height`, `panes`, `sill`, `curtains`, `glass`, `light` | `view` (clipped, parallax), `sill` | rect light | side, perspective, iso |
| `door` | `panel` | `open` 0..1, `handle`, `style` | `opening`, `handle` | — | side, iso |
| `curtain` | `rope` + cloth | `folds`, `gathered`, `sway` | — | — | side |
| `wall-shelf` | `box` | `width`, `brackets`, `bracket-style` | `top`, `under` | — | side, iso |
| `shelf` | `container` | `levels`, `books` (`none`…`packed`), `clutter`, `top-clutter`, `side-depth` | `level-n`, `top` | — | side, iso |
| `desk`, `table` | `box` | `width`, `height`, `legs`, `drawers` | `top`, `under` | — | side, iso |
| `chair`, `stool` | `model` / `shape` | `style`, `facing` | `seat`, `back` | — | all |
| `bed`, `sofa` | `box` + `blob` | `width`, `blanket`, `pillows` | `top`, `pillow` | — | side, iso |
| `cabinet` | `panel` | `doors`, `drawers` | `top` | — | side, iso |
| `book` | `box` | `width`, `height`, `lying`, `lean`, `bands`, `label` | — | — | side |
| `book-row` | `book` × repeat | `fill`, `height`, `width`, `gap-chance`, `lean-chance`, `flat-stack-chance`, `palette-group` | — | — | side |
| `book-stack` | `book` × stack | `count`, `size`, `jitter` | `top` | — | side, iso |
| `box`, `bag`, `papers` | `box` / `blob` | `kind`, `open`, `logo` piece | `top` | — | side, iso |
| `crt-monitor` | `box` + `panel` | `size`, `facing`, `on`, `glow` | `screen`, `top` | screen + rect light | side, iso |
| `tv` | `box` + `panel` | `size`, `on`, `knobs`, `antenna`, `stand` | `top`, `screen`, `side-l/r` | screen glow | side, iso |
| `keyboard` | `box` | `width` | `keys` (IK target) | — | side |
| `gadget` | `box` | `kind` (`radio` \| `handheld` \| `clock` \| `cassette`), `on` | `screen` | small light | side |
| `heater` | `box` | `kind`, `on`, `flicker` | — | grill + point light | side, iso |
| `desk-lamp` | rig (2–3 bones) | `mount`, `arm-angles`, `head-angle`, `on`, `color`, `intensity`, `reach`, `spread` | — | `light-cone` | side |
| `pendant-lamp` | `rope` + `cylinder` | `cord`, `shade`, `on`, `swing` | — | downward cone | side, iso |
| `candle`, `string-lights` | `cylinder` / `rope` | `flicker`, `bulbs` | — | point lights | all |
| `frame`, `poster`, `calendar` | `panel` | `content` (child scene or preset), `hanging` | `content` | — | side |
| `plant`, `pot` | `lsystem` / `cylinder` | `kind`, `height`, `sway` | `soil` | — | all |
| `mug`, `kettle` | `cylinder` | `size`, `handle`, `steam` | — | `steam` emitter | side, iso |
| `bust`, `ornament` | `blob` / `model` | `kind`, `size` | — | — | side, iso |
| `counter` | `container` | `width`, `height`, `front` (`panel` \| `glass` \| `open`) | `top`, `behind` | — | side, iso |
| `ball` | `shape` (circle) / `model` (sphere) | `size`, `pattern` (`solid` \| `stripe` \| `panels`), `bounce` | — | — | all |

---

## Part 4 — `nature`

Roles used: `foliage`, `wood`, `stone`, `soil`, `sky`, `cool`, `warm`, plus `bark`, `grass`, `leaves-autumn`, `flower-*`, `sand`, `snow`, `water`.

### 4.1 Key prefabs

#### `tree`
A preset layer over `tree-gen`:

| Preset | Crown | Height | Notes |
|---|---|---|---|
| `oak` | round, wide | 50–80 | Many clusters |
| `pine` | conical, stacked tiers | 60–100 | Tiers are layered triangles |
| `birch` | columnar, sparse | 50–70 | Light bark with marks |
| `palm` | palm | 50–90 | Bent trunk, frond `lsystem` |
| `willow` | willow | 40–60 | Hanging `rope` strands |
| `dead` | bare | 40–70 | Branches only |

```kdl
tree preset="pine" height="70..90" wind=0.3 season="winter"
```

#### `grass`
| Param | Type | Default | Notes |
|---|---|---|---|
| `height` | range | `"3..8"` | |
| `density` | 0..1 | 0.7 | |
| `blade` | `thin` \| `tuft` \| `reed` | `tuft` | |
| `wind` | 0..1 | 0.3 | A traveling gust wave crosses the field |
| `flowers` | 0..1 | 0 | Mixes in `flower` prefabs |

#### `river` / `stream`
`water-surface` along a path: `width`, `flow` (speed), `banks` (`grass` \| `stone` \| `sand`), `rocks` (0..1, with foam around them), `reflection`. Animated.

#### `fireflies`
`emitter` preset: `count`, `area`, `wander` (0..1), `pulse` (period range), `color` (`warm` \| `screen-green`), `light` (each casts a 1-step point light). Emissive.

### 4.2 Other prefabs

| Prefab | Built on | Key params |
|---|---|---|
| `bush` | `blob` | `size`, `clusters`, `berries` 0..1, `wind` |
| `flower` | `lsystem` + piece | `kind` (`daisy` \| `tulip` \| `wildflower` \| `sunflower`), `color` role |
| `mushroom` | `cylinder` + `blob` | `cap`, `spots`, `glow` (bioluminescent) |
| `fallen-log`, `stump` | `cylinder` | `length`, `moss`, `rings` |
| `rock`, `boulder`, `pebbles` | `rock-gen` | `size`, `moss`, `count` |
| `cliff` | `cliff` | as core-kit |
| `waterfall` | `water-surface` + `emitter` | `height`, `width`, `mist` |
| `fog-bank` | `blob` + noise | `height`, `density`, `drift`, `layer` |
| `campfire` | `emitter` + light | `size`, `smoke`, `crackle` (flicker) |
| `falling-leaves`, `petals` | `emitter` presets | `rate`, `season` |

---

## Part 5 — `urban`

Roles used: `metal`, `glass`, `plastic`, `stone`, `warm`, `cool`, `screen`, plus `brick`, `concrete`, `asphalt`, `neon-*`, `paint-*`.

### 5.1 Key prefabs

#### `building`
A preset layer over `facade-gen`: `apartment`, `shop-row`, `office-tower`, `townhouse`, `warehouse`, `cottage`, `tower`.

#### `street`
| Param | Type | Default | Notes |
|---|---|---|---|
| `lanes` | 1..4 | 2 | |
| `markings` | `none` \| `dashed` \| `solid` \| `crosswalk` | `dashed` | |
| `curb` | bool | true | |
| `wet` | 0..1 | 0 | Darkens and adds reflections (uses `water-surface` puddle mode) |
| `vanishing-point` | `"x,y"` | center | Perspective view only: everything on the street converges here |

- **Sockets:** `lane-n` (path, for vehicles), `sidewalk-l/r` (line), `crosswalk` (area)
- **Views:** side (as a strip), perspective, top-down, isometric

#### `neon-sign`
Built on `neon-text` plus a mount: `content`, `shape` (`text` \| `arrow` \| `icon` piece), `color` role, `mount` (`wall` \| `hanging` \| `rooftop`), `flicker`, `buzz` (period of a full blackout), `broken-letters`. Casts colored light and reflects in wet surfaces.

#### `puddle`
`water-surface` in `puddle` mode: `size`, `shape` (seeded blob), `reflection`, `ripples` (driven by a `rain` emitter's splash hits).

### 5.2 Other prefabs

| Prefab | Built on | Key params |
|---|---|---|
| `sidewalk` | `strip` | `width`, `tiles`, `cracks`, `curb` |
| `street-lamp` | `cylinder` + light | `style` (`classic` \| `modern` \| `gas`), `on`, `flicker`, `bugs` (moths emitter) |
| `traffic-light` | `box` | `cycle` (state machine: green → amber → red), `timing` |
| `shop-front` | `panel` | `awning`, `display` (content area), `sign`, `lit` |
| `billboard` | `panel` | `content`, `lights` |
| `power-lines` | `rope` presets | `poles`, `wires`, `birds` (socket-ready) |
| `vending-machine` | `box` | `glow`, `items` |
| `steam-vent` | `emitter` | `rate`, `grate` |
| `fence`, `railing` | `strip` | `style`, `posts` |
| `bench`, `bin`, `hydrant`, `mailbox`, `well` | `model` / `shape` | `style`, `wear` |
| `car`, `bus`, `bike` | `machine` body plan instances | see Part 7 |

---

## Part 6 — `water`

Roles used: `sand`, `stone`, `foliage`, `glass`, `cool`, plus `water`, `water-deep`, `coral-*`.

### 6.1 Key prefabs

#### `ocean`
`water-surface` preset: `swell` (large slow waves), `chop` (small fast waves), `horizon` (y), `sun-glint` (sparkle on wave crests, emissive), `split-view` (bool: shows above and below the waterline at once, with a refracted underwater part).

#### `kelp`
`rope` preset anchored to the seabed with its free end floating: `height`, `fronds` (leaf pieces along the rope), `sway` (current-driven), `count` (as a forest with `scatter`).

#### `caustics`
A light, not an object: an animated cellular-noise mask projected onto surfaces below the water, adding +1 tone where the bright lines fall. Params: `scale`, `speed`, `strength`, `depth-fade`.

### 6.2 Other prefabs

| Prefab | Built on | Key params |
|---|---|---|
| `shore` | `ground` + `water-surface` | `slope`, `wave-cycle` (period of the lapping foam), `wet-band` |
| `seabed` | `ground` | `surface` (`sand` \| `rock` \| `mud`), `ripples`, `shells` |
| `coral` | `lsystem` / `blob` | `kind` (`branch` \| `brain` \| `fan` \| `tube`), `color` role, `sway` |
| `seagrass` | `grass` preset | `height`, `current` |
| `bubbles` | `emitter` preset | `rate`, `size`, `wobble`, `source` (socket) |
| `light-rays` | light + dither mask | `count`, `angle`, `shimmer` |
| `underwater-fog` | depth tint | `density`, `color` role |
| `boat`, `buoy` | `machine` / `model` | `bob` (follows the surface) |

---

## Part 7 — `creatures`

### 7.1 Templates and instances
A **template** is a bone topology with its generic systems (gaits, postures, anchors). An **instance** is a template plus proportions, volumes, coverings, pieces and default clips. Templates and instances are both KDL; new creatures are new instances, and truly new topologies are new templates. Neither needs engine code.

```kdl
body-plan "fox" template="quadruped" {
    build size=20 leg-length=0.5 tail-length=0.9 ear-size=0.8 snout=0.7
    covering kind="fur" length=0.3 pattern="two-tone"
    materials fur="fur-orange" fur-accent="cloth-cream"
}
```

### 7.2 Shared systems (all templates)

| System | What it does | Applies to |
|---|---|---|
| **Covering** | Generates the outer mass and texture: `fur`, `hair`, `feathers`, `scales`, `skin`, `shell`, `metal`. Params: `length`, `volume`, `pattern` (`solid` \| `two-tone` \| `tabby` \| `spots` \| `stripes` \| `tuxedo` \| `calico` \| `countershade`) | all |
| **Hair** | Head hair mass: `style` (`short` \| `medium` \| `long` \| `bun` \| `curly` \| `bald` \| `ponytail`), `volume` | biped |
| **Outfit / gear** | Rules that assign material slots to bones and adjust volumes. Bipeds: `top` (`tshirt` \| `sweater` \| `hoodie` \| `shirt` \| `dress` \| `tunic` \| `jacket` \| `coat` \| `robe` \| `armor` \| `none`), `sleeves` (`none` \| `short` \| `long`), `bottom` (`pants` \| `shorts` \| `skirt`), `shoes` (`none` \| `shoes` \| `slippers` \| `boots`). Others: `collar`, `saddle`, `harness`, `armor` | outfit: biped; gear: all |
| **Face** | Small heads (< 16 px): anchors for `eyes`, `mouth`, `nose`/`beak`/`snout`, with expressions as piece variants. Large heads: the parametric **face rig** (§7.2.1) | all with a head |
| **IK targets** | Any limb end can be aimed at a socket: `target hand-r="cup.handle"`, `target paws="tv.side-r"` | all |
| **Gait generator** | Walk/run cycles generated from leg count, leg lengths and a phase table. No hand-made walk clips | biped, quadruped, multi-leg, bird (hop/walk) |
| **Clip channels** | Clips layer by channel: `posture` < `body` < `limbs` < `head` < `face`, plus `tail` / `appendage` | all |
| **Facings** | `shape` path: `side`, `front`, `back`. `model` path: any of 4 or 8 directions, top-down and isometric | all |

#### 7.2.1 Face rig (close-ups)
When a head is 16 px or taller (close-ups, portraits, dialogue: B7), facial features are **generated from parameters**, the same way bodies are. They are still drawn through the cleanup passes, so they stay crisp.

| Feature | Params |
|---|---|
| `eyes` | `shape` (`round` \| `almond` \| `narrow` \| `big`), `size`, `spacing`, `iris` (role), `highlight` (bool), `lashes` |
| `brows` | `thickness`, `length`, `arch` |
| `nose` | `kind` (`dot` \| `button` \| `long` \| `hook` \| `none`), `size` |
| `mouth` | `width`, `lip` (0..1), `teeth` (bool) |
| `ears` | `kind`, `size` |
| `cheeks`, `freckles`, `beard`, `wrinkles` | optional details (`0..1` amounts) |

**Expressions are parameter sets, not drawings.** Each expression sets brow angle, eye openness, pupil position, mouth curve and mouth openness. Blending between expressions happens in parameter space, and the face is re-rasterized every frame:

| Expression | Brows | Eyes | Mouth |
|---|---|---|---|
| `neutral` | flat | open | flat |
| `happy` | raised | slightly closed | curve up |
| `sad` | inner up | lowered | curve down |
| `angry` | inner down | narrowed | tight |
| `surprised` | high | wide | open round |
| `thinking` | one raised | look up-side | small, offset |

Custom expressions are KDL: `expression "smug" brows="-10,15" eyes=0.7 mouth-curve=0.4 mouth-offset=2`.

**Talking.** The `talk` clip drives **mouth shapes** (`rest`, `a`, `e`, `o`, `m`, `f`) from:
- a `line` text, using deterministic letter-to-shape rules (no speech recognition, no AI), or
- a `cue` track with explicit shapes and times.

**Framing.** `framing` (`full` \| `bust` \| `head`) on a character renders only that part of the body, at the character's `height` for the visible part, with the face rig and `hd` pieces. The same cast prefab can appear full-body in one shot and as a bust in the next (B8).

Blinks, small head turns, eye darts and breathing are added on top by the default face machine. Faces too small for the rig fall back to pieces (`face-dot`, `face-big-eyes`; see [PIECES.md](PIECES.md) §16.1).

**Gait phase tables** (fraction of the cycle at which each foot lifts):

| Gait | Template | Phases |
|---|---|---|
| `walk` | biped | L 0.0, R 0.5 |
| `run` | biped | L 0.0, R 0.5, with a flight phase |
| `walk` | quadruped | LH 0.0, LF 0.25, RH 0.5, RF 0.75 |
| `trot` | quadruped | LF+RH 0.0, RF+LH 0.5 |
| `gallop` | quadruped | LH 0.0, RH 0.1, LF 0.4, RF 0.5 |
| `tripod` | multi-leg (6) | alternating sets of 3 |
| `wave` | multi-leg (n) | each leg offset by 1/n |

### 7.3 Templates

#### `biped`
| Aspect | Definition |
|---|---|
| **Bones (19)** | `pelvis`, `spine`, `chest`, `neck`, `head`, `shoulder-l/r`, `upper-arm-l/r`, `forearm-l/r`, `hand-l/r`, `thigh-l/r`, `shin-l/r`, `foot-l/r` |
| **Volumes** | Head: ellipse. Torso: tapered rounded box. Limbs: capsules. Hands: blobs. Feet: rounded boxes |
| **Build params** | `height`, `bulk`, `head-ratio`, `leg-ratio`, `shoulder-width` |
| **Anchors** | `crown`, `forehead`, `eyes`, `nose`, `mouth`, `ear`, `neck`, `chest`, `torso`, `waist`, `back`, `hand-l/r`, `foot-l/r`, `seat`, `lap` |
| **Postures** | `stand`, `sit-chair`, `sit-edge`, `sit-floor`, `kneel`, `crouch`, `lie` |
| **Generic clips** | `breathe`, `blink`, `idle-sway`, `look`, `turn-head`, `nod`, `talk`, `wave`, `point`, `reach`, `walk`, `run`, `jump`, `fall`, `hurt` |

#### `quadruped`
| Aspect | Definition |
|---|---|
| **Bones (~22 + tail)** | `pelvis`, `spine`, `chest`, `neck`, `head`, `jaw`, `ear-l/r`, `front-upper/lower/paw-l/r`, `hind-thigh/shin/paw-l/r`, `tail-1…n` |
| **Volumes** | Body: capsule. Head: circle + `snout` capsule. Legs: thin capsules. Tail: capsule chain (`rope` sim) |
| **Build params** | `size`, `bulk`, `leg-length`, `neck-length`, `snout`, `tail-length`, `ear-size`, `ear-type` (`pointed` \| `round` \| `floppy`) |
| **Anchors** | `head-top`, `eyes`, `nose`, `mouth`, `collar`, `back`, `saddle`, `paw-fl/fr/hl/hr`, `tail-tip` |
| **Postures** | `stand`, `sit`, `lie`, `curl`, `loaf`, `stretch`, `rear-up` |
| **Generic clips** | `breathe`, `blink`, `ear-twitch`, `look`, `sniff`, `graze`, `scratch`, `shake`, `walk`, `trot`, `gallop`, `jump`, `tail-sway`, `tail-wag` |

#### `bird`
| Aspect | Definition |
|---|---|
| **Bones** | `body`, `neck`, `head`, `beak`, `wing-l/r` (3 segments each), `tail`, `leg-l/r`, `foot-l/r` |
| **Volumes** | Body: ellipse. Wings: wedge shapes with feather steps. Beak: triangle |
| **Build params** | `size`, `wingspan`, `neck-length`, `beak` (`short` \| `long` \| `hooked` \| `flat`), `tail` (`short` \| `long` \| `fan`), `legs` |
| **Postures** | `perch`, `stand`, `fly`, `glide`, `swim` |
| **Generic clips** | `flap`, `glide`, `hop`, `peck`, `preen`, `look`, `take-off`, `land` |

#### `fish`
| Aspect | Definition |
|---|---|
| **Bones** | `head`, `body-1…4` (spine chain), `tail-fin`, `dorsal`, `pectoral-l/r`, `jaw` |
| **Volumes** | Body: tapered ellipse chain. Fins: thin wedges |
| **Build params** | `size`, `body` (`slim` \| `round` \| `flat` \| `long`), `fins` (size), `tail` (`fork` \| `round` \| `lunate`) |
| **Generic clips** | `swim` (traveling S-wave along the spine), `dart`, `idle-hover`, `turn`, `gulp` |

#### `serpent`
| Aspect | Definition |
|---|---|
| **Bones** | `head`, `jaw`, `segment-1…n` |
| **Volumes** | Capsule chain tapering toward the tail |
| **Build params** | `length`, `thickness`, `taper`, `segments` |
| **Generic clips** | `slither` (lateral wave), `coil`, `rise`, `strike`, `tongue` |
| **Used for** | Snakes, worms, eels, dragons' tails, tentacles |

#### `multi-leg`
| Aspect | Definition |
|---|---|
| **Bones** | `head`, `thorax`, `abdomen`, `leg-1…n` (2–3 segments each), `antenna-l/r`, optional `wing-l/r` |
| **Build params** | `legs` (4 \| 6 \| 8 \| n), `leg-length`, `body` (`round` \| `long` \| `segmented`), `antennae`, `wings` |
| **Generic clips** | `walk` (`tripod` / `wave` gait), `idle-twitch`, `fly` (if wings) |
| **Used for** | Insects, spiders, crabs, centipedes |

#### `machine`
For vehicles and mechanisms: rigid parts joined by typed joints.

| Joint | Motion | Examples |
|---|---|---|
| `wheel` | Spins with travel distance; with `suspension` it bobs | Car wheels, bike wheels |
| `hinge` | Rotates within limits | Doors, hoods, lamp arms |
| `slider` | Moves along an axis | Pistons, drawers |
| `spin` | Rotates continuously | Fans, propellers, gears |
| `bob` | Follows a surface or noise | Boats, hovering things |

Build params: `length`, `height`, `body` (preset shapes or a `model`), `wheels`, `lights` (headlights/taillights as light emitters). Generic clips: `drive` (wheels spin, body bobs), `idle` (engine vibration: 1 px jitter on2), `brake`, `turn`.

### 7.4 Instances

| Instance | Template | Size | Notes |
|---|---|---|---|
| `human-adult` | biped | 32–48 px (default 38), ~5 heads | Hair, outfit, face systems on |
| `human-child` | biped | 20–32 px (default 26), ~3.5–4 heads | Larger `head-ratio`; extra clips `swing-legs`, `look-around`, `fidget` |
| `human-elder` | biped | 32–44 px | Stooped posture, slower gait |
| `hero` | biped | 24–32 px | Game-ready: `attack`, `block`, `dodge`, `climb`, `cast` clips; 8 directions via `model` path |
| `cat` | quadruped | 12–30 px long | Ears pointed, tail clips `sway`, `flick`, `curl`, `puff` |
| `dog` | quadruped | 14–40 px | `ear-type`, `snout`, `tail-type` presets per breed-ish shape |
| `horse` | quadruped | 40–70 px | `gallop` gait, `mane` covering, `saddle` socket |
| `deer` | quadruped | 30–50 px | `antlers` (`lsystem` preset) |
| `songbird` | bird | 6–12 px | Perch-and-hop behavior |
| `gull` | bird | 12–20 px | Glide-heavy flight |
| `chicken` | bird | 8–14 px | Ground bird: `peck`, `scratch`, short `flap` |
| `fish` | fish | 6–30 px | `body` presets; schools via `flock` |
| `snake` | serpent | 30–80 px | |
| `spider` | multi-leg (8) | 4–16 px | `rope` silk thread |
| `crab` | multi-leg (8+2 claws) | 8–20 px | Sideways walk |
| `beetle` | multi-leg (6) | 4–12 px | Optional wings |
| `car` | machine | 40–64 px | 4 wheels, head/tail lights, `drive` |
| `bike` | machine | 20–30 px | 2 wheels, rider socket (`seat`) for a biped |
| `boat` | machine | 20–60 px | `bob` on `water-surface` |

---

## Part 8 — How to make your own pack

```
packs/my-castle/
  pack.kdl          ← manifest
  prefabs/          ← KDL prefabs (shape, model or composite)
  bodies/           ← body plan instances or templates
  pieces/           ← small text-grid details
  presets/          ← named parameter sets for existing prefabs
  gallery.kdl       ← a scene that shows every prefab (used for review and tests)
```

```kdl
pack "my-castle" version="0.1.0" {
    depends "core-kit" "nature"
    roles-required "stone" "wood" "warm" "banner-red"   // roles the palette must provide
    views "side" "isometric"
}
```

Guidelines:
1. **Build on `core-kit`.** A castle wall is a `box` + `strip` crenellations; a tower is a `cylinder` + `cone` roof. Write new Rust only when no combination of kit parts can express it.
2. **Ask for roles, never colors.** If you need a new role, add it with a fallback (`banner-red` → `cloth-red`).
3. **Declare views honestly.** If a prefab only works in side view, say so; the validator warns when a scene uses it in another view.
4. **Ship a gallery scene.** It doubles as documentation and as a visual regression test.
5. **Presets before prefabs.** If an existing prefab with different params gives the result, ship a preset, not a new prefab.
