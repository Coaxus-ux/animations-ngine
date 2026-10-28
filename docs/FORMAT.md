# Format Reference (draft v1)

> Companion to [SPEC.md](SPEC.md). The complete reference for the KDL files the engine reads. Content (prefabs, body plans) is in [CATALOG.md](CATALOG.md), pieces in [PIECES.md](PIECES.md), palettes in [PALETTES.md](PALETTES.md), animation semantics in [ANIMATION.md](ANIMATION.md), sequences and export in [DIRECTOR.md](DIRECTOR.md), scripts in [SCRIPTING.md](SCRIPTING.md).

Everything here is scene-agnostic. Examples come from the benchmarks (see [BENCHMARKS.md](BENCHMARKS.md)): **B1** cozy room, **B2** forest at dusk, **B3** neon street, **B4** hero sprite sheet, **B5** underwater coast, **B6** isometric village.

---

## 1. Files and project layout

### 1.1 File kinds

| Extension | Contains | Top-level nodes allowed |
|---|---|---|
| `project.kdl` | Project settings | `format`, `project` |
| `pack.kdl` | Pack manifest | `format`, `pack` |
| `*.kdl` in `scenes/` | Scenes | `use`, `scene` |
| `*.kdl` in `prefabs/` | Prefabs, body plans, clips | `use`, `prefab`, `body-plan`, `clip` |
| `*.kdl` in `pieces/` | Pieces, piece sets, fonts | `piece`, `piece-set`, `font` |
| `*.kdl` in `palettes/` | Palettes | `palette` |
| `*.kdl` in `sequences/` | Sequences and exports | `use`, `sequence`, `export` |
| `*.kdl` in `tiles/` | Tilesets and tilemaps | `use`, `tileset`, `tilemap` |
| `*.ts` in `scripts/` | Scripts | — (see SCRIPTING.md) |
| `*.tweaks.kdl` | Reserved: editor-only overrides | — |

Folder placement is a convention, not a rule: any `.kdl` file may hold any declaration node. Folders only decide the default search order.

### 1.2 Project layout
```
my-project/
  project.kdl
  scenes/  prefabs/  pieces/  palettes/  sequences/  tiles/  scripts/
  packs/            ← vendored or shared packs (each has a pack.kdl)
  output/
```

### 1.3 Packs
A pack is a folder with a `pack.kdl` and the same subfolders as a project. The engine's standard library ships as packs: `core-kit`, `interior`, `nature`, `urban`, `water`, `creatures`.

```kdl
format 1
pack "nature" version="1.2.0" {
    description "Trees, terrain, sky, weather, wildlife"
    depends "core-kit" ">=1.0"
    depends "creatures" ">=1.0"
    exports "tree" "bush" "grass-field" "hills" "sky" "fireflies" "deer"   // optional; default = everything
}
```

### 1.4 Imports and name resolution
```kdl
use "nature"                              // whole pack
use "urban" only="neon-sign street-lamp"  // selected names
use "./packs/my-props" as="props"         // local pack, prefixed
```
How a bare name (for example `tree`) resolves:
1. Declarations in the project itself.
2. Packs in `use` order, top to bottom.
3. `core-kit`, which is always imported.

- A **qualified name** always wins: `nature:tree`, `props:barrel`.
- If two imported packs both export a name, using it bare is an **error**; you must qualify it.
- `use` lines can appear in `project.kdl` (applied everywhere) or at the top of a single file.

---

## 2. KDL conventions

| Rule | Example |
|---|---|
| KDL v2 syntax | booleans `#true` / `#false`, null `#null` |
| A node is a thing | `tree`, `light-cone`, `character` |
| The first argument is an optional **id** | `character "hero"` |
| Properties are parameters | `x=40 height=38` |
| Children `{ }` hold contents or detailed config | `window { window-rain }` |
| Names are English, lowercase, kebab-case | `desk-lamp`, `neon-sign` |
| `//` starts a comment, `/* */` is a block comment | |
| `/-` disables the next node, argument or property | `/-deer x=80` |
| Multi-line strings use `"""` (dedented) | piece grids |
| Inside a KDL string, quoted text uses single quotes | `"=pulse('1.5s')"` |

**Ids** are unique within their scope:
- In a scene, the scope is the scene.
- In a prefab, the scope is the prefab instance; inner ids are addressed as `@instance.inner`.
- Objects without an id get an automatic index path. The editor warns when you reference or animate an object that has no id, because inserting siblings would shift its seed.

---

## 3. Top-level nodes

| Node | Purpose | Detailed in |
|---|---|---|
| `format` | Format version of the file set | §15 |
| `project` | Resolution, fps, defaults | §4 |
| `pack` | Pack manifest | §1.3 |
| `use` | Import a pack | §1.4 |
| `scene` | A place with objects, characters and ambience | §5 |
| `prefab` | A reusable object (2D or `model`) | §11 |
| `body-plan` | A creature or machine rig | §12 |
| `clip` | A named pose animation for a body plan or template | §12.2, ANIMATION.md |
| `piece`, `piece-set`, `font` | Small grids | PIECES.md |
| `palette` | Ramps, roles, twins, light colors | PALETTES.md |
| `style` | A rendering style preset | §5.2, RENDERING.md §16 |
| `tileset`, `tilemap` | Game tiles and maps | §13 |
| `sequence` | Shots and transitions | §14, DIRECTOR.md |
| `export` | Output job (video, sprite sheet, tileset, layers) | §14, DIRECTOR.md |

---

## 4. `project`

```kdl
format 1
project "dreams" {
    resolution 320 180
    preview-scale 4
    fps 12
    seed 1
    default-palette "night-blue"
    default-view "side"
    use "nature"
    use "urban"
    export format="gif" scale=4 folder="output"
}
```

| Child / prop | Type | Default | Meaning |
|---|---|---|---|
| `resolution` | 2 ints | `256 256` | Logical canvas size |
| `preview-scale` | int | 4 | Integer zoom in the editor |
| `fps` | int | 12 | Default output frame rate |
| `seed` | int | 0 | Global seed (combined with each object path) |
| `default-palette` | name | — | Used when a scene sets none |
| `default-view` | view | `side` | Used when a scene sets none |
| `default-style` | style | `moody` | Used when a scene sets none |
| `use` | pack | — | Project-wide imports |
| `export` | props | — | Default export settings |

---

## 5. `scene`

```kdl
scene "dusk-forest" palette="dusk" view="side" seed=7 duration="10s" loop=#true {
    param wind 0.4 min=0 max=1
    ambient darkness=1
    layer sky parallax=0 { sky kind="dusk" stars=0.2 }
    layer hills parallax=0.3 { hills layers=3 roughness=0.5 }
    layer trees parallax=0.7 {
        scatter n=14 area="ground" min-dist=12 {
            tree kind="pine" height="40..70" sway="=$wind"
        }
    }
    layer ground { grass-field density=0.8 wind="=$wind" }
    layer fx { fireflies n=20 area="ground" }
}
```

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `palette` | name | project default | |
| `view` | view | project default | §6 |
| `style` | style | project default | Rendering style preset (§5.2) |
| `seed` | int | project seed | Scene seed |
| `duration` | time | `"10s"` | |
| `loop` | bool | `#false` | Makes all time noise periodic (§9.6) and enables the loop solver |
| `fps` | int | project fps | |
| `size` | 2 ints | project resolution | Canvas override (for example, a tall sprite) |
| `seamless-x` | bool | `#false` | The world wraps horizontally, so a panning camera can loop (B2) |
| `world-width` | int | canvas width | Wrap period in px; each layer wraps at `world-width × parallax` (must be an integer) |

| Child | Meaning |
|---|---|
| `param` | Exposed parameter (§7.6) |
| `ambient` | Ambient darkness and color |
| `layer` | A depth layer (§5.1) |
| `camera` | Default camera for the scene (see DIRECTOR.md) |
| any object | Placed directly in an implicit `main` layer |

### 5.1 `layer`
| Prop | Type | Default | Meaning |
|---|---|---|---|
| `parallax` | 0..2 | 1 | Camera-motion factor (0 = fixed sky, >1 = foreground) |
| `depth` | int | file order | Draw order between layers |
| `lighting` | bool | `#true` | Whether scene lights affect this layer |
| `palette-override` | name | — | Use another palette's ramps for this layer |
| `fog` | 0..1 | 0 | Shift tones toward the scene's fog ramp (distance haze) |
| `export-as` | string | — | Name used when exporting parallax layers |

### 5.2 `style`
A style preset is a named set of rendering rules ([RENDERING.md](RENDERING.md) §16). Shipped presets: `moody` (default), `cartoon`, `one-bit`, `handheld`, `crisp-hd`, `chunky`. Custom presets extend one:

```kdl
style "storybook" extends="cartoon" {
    tones 4
    dither "bayer-2" strength=0.3
    outline "full" weight=1 ramp="ink"
}

scene "forest-dusk" style="storybook" { /* … */ }
```

Any setting can also be overridden inline on a scene or layer: `scene "x" style="moody" style-dither="off"`.

---

## 6. Views

| `view` | Coordinates | How it draws | Status |
|---|---|---|---|
| `side` | `x`, `y` screen px; `z` is draw order | Flat, front-facing; 2D shapes and `model` prefabs | v1 |
| `perspective` | `x`, `y`, `z` world; `z` = distance | 2D shapes placed with one- or two-point helpers; `model` prefabs fully projected | v1.x |
| `top-down` | `x`, `y` on the ground plane; `h` height | `model` prefabs; 2D prefabs that declare `top-down` | later |
| `isometric` | `x`, `y` on the ground plane (tiles or px); `h` height | 2:1 isometric; `model` prefabs and tilesets | later |

```kdl
scene "neon-street" view="perspective" {
    perspective mode="one-point" horizon=70 vanish=160      // B3
}
scene "village" view="isometric" {
    isometric tile=16 ratio="2:1"                            // B6
}
```

- Every prefab declares the views it supports (`views="side perspective"`). `model` prefabs support all views automatically.
- Using a prefab in an unsupported view is an **error** that names the views the prefab does support.

---

## 7. Values

### 7.1 Value kinds

| Kind | Syntax | Example |
|---|---|---|
| Fixed | number, string, bool | `x=140`, `kind="pine"`, `on=#true` |
| With unit | `"<n><unit>"` | `"3s"`, `"36f"`, `"50%"`, `"45deg"` |
| Random range | `"a..b"` | `"2..5"`, `"3s..6s"`, `"-10deg..10deg"` |
| Expression | `"=…"` | `"=2*sin(t*3)"` |
| Param | `"=$name"` | `"=$wind"` |
| Reference | `"=@id.prop"` | `"=@lamp.intensity*0.5"` |
| List | space-separated string | `clips="idle walk run"` |
| Keyframed | `anim` child block | §8 |

Params and references are ordinary expressions, so they combine freely: `"=$wind*0.5+@lamp.intensity"`.

### 7.2 Units

| Unit | Meaning | Where |
|---|---|---|
| *(none)* | Pixels for sizes and positions, plain numbers elsewhere | everywhere |
| `px` | Pixels (explicit) | positions, sizes |
| `%` | Percentage of the parent's matching dimension | positions, sizes, `at=` |
| `f` | Frames | time |
| `s`, `ms` | Seconds, milliseconds | time |
| `deg` | Degrees (the only angle unit in KDL values) | angles |
| `hu` | Head units | inside body plans |
| `tile` | Tile size of the current tileset or isometric grid | B6, tilemaps |

Time values are converted to integer frames at the scene's fps. `"1.5s"` at 12 fps = 18 frames. Values that aren't whole frames round to the nearest frame, with a warning.

### 7.3 Ranges
- `"a..b"` picks a value once per object, from the object's seed. It is stable across frames and reloads.
- If both ends are integers, the result is an integer (inclusive).
- Both ends must use the same unit.
- To get a new value per repetition, the range uses the repeater's index automatically (§10).
- For anything other than a uniform pick, use an expression: `"=range(10,20,i)"`, `"=pick(i,'oak','pine','birch')"`.

### 7.4 Expressions
A string starting with `=` is an expression, evaluated every frame (or once, if it doesn't depend on time). See §9.

### 7.5 References

| Form | Means |
|---|---|
| `@id.prop` | A property of the object `id` |
| `@id.inner.prop` | A property of an object inside a prefab instance |
| `@id.socket-name` | A socket (used by `on=`) |
| `@id.anchor.eyes.x` | An anchor position (body plans) |
| `@id.state` | Current state name of a state machine |
| `@self`, `@parent` | The current object, its parent |
| `@id.port` | An output port of a node inside a prefab or group graph (node wiring; see [NODES.md](NODES.md) §19) |

- References can't form a cycle in the same frame; a cycle is an error that shows the chain.
- A reference to a time-dependent property makes the referring property time-dependent too.

### 7.6 Params
Scene-level parameters (shown as controls in the editor):
```kdl
param wind 0.4 min=0 max=1 step=0.05 label="Wind" group="Weather"
param weather "rain" options="rain snow clear"
param hero-color "cloth-red" kind="ramp"
```
| Prop | Meaning |
|---|---|
| default (argument) | Initial value; the type is inferred |
| `min`, `max`, `step` | Numeric limits |
| `options` | Enumeration (shown as a dropdown) |
| `kind` | `number`, `bool`, `string`, `ramp`, `time`, `color` (when inference isn't enough) |
| `label`, `group` | Editor presentation |

Prefabs declare their parameters with `input` (same props; §11). Both are read as `$name` in expressions.

---

## 8. Keyframes

```kdl
car "taxi" {                                     // B3
    anim x loop="none" blend="replace" {
        k "0s"  -40
        k "4s"  360 interp="linear"
    }
}
camera {                                         // multi-property keys
    k "0s" x=0  y=0
    k "8s" x=12 y=0 interp="smooth"
}
```

| Element | Syntax | Meaning |
|---|---|---|
| Track | `anim <prop> { … }` | Animates one property |
| Key | `k <time> <value> [interp=…]` | `interp` applies to the segment that **starts** at this key |
| Multi-property key | `k <time> a=1 b=2` | Allowed in `camera` and in `clip` |
| Relative time | `k "+1s" …` | Relative to the previous key |
| Track loop | `loop="none\|repeat\|pingpong"` | Behavior after the last key |
| Blend with base value | `blend="replace\|add\|multiply"` | How the track combines with the property's value or expression |

**Interpolations:** `step`, `linear`, `smooth`, `ease-in`, `ease-out`, `ease-in-out`, `bounce`, `elastic`, `hold-2`, `hold-3` (the last two step the segment on 2s or 3s).

Positions are snapped to whole pixels *after* interpolation (§9.7). See ANIMATION.md for the evaluation order.

---

## 9. Expression language

A small, pure, deterministic language that runs in Rust. It is fast enough to evaluate per pixel inside the `expression` node.

### 9.1 Literals and operators

| Category | Syntax |
|---|---|
| Numbers | `3`, `0.5`, `-2` |
| Time literals | `3s`, `500ms`, `36f` (all evaluate to **seconds**) |
| Angle literals | `45deg` (evaluates to **degrees**) |
| Strings | `'pine'` (single quotes inside KDL strings) |
| Booleans | `true`, `false` |
| Arithmetic | `+ - * / %` and `**` (power) |
| Comparison | `== != < <= > >=` |
| Logic | `&& \|\| !` |
| Conditional | `cond ? a : b` |
| Grouping | `( )` |

**Precedence**, high to low: `!` and unary `-` → `**` → `* / %` → `+ -` → comparisons → `&&` → `||` → `? :`.

**Hyphens:** `$names` and `@ids` are kebab-case, and a hyphen followed by a letter is part of the name (`$hero-color`, `@street-lamp.on`). When the right-hand side of a subtraction starts with a letter, write it with spaces: `$width - w`.

### 9.2 Variables

| Name | Meaning |
|---|---|
| `t` | Scene time in seconds |
| `f` | Frame index (integer) |
| `fps` | Frames per second |
| `dur` | Scene duration in seconds |
| `seed` | The object's own seed |
| `i`, `n` | Index and count inside `repeat` / `scatter` (0 and 1 outside them) |
| `u` | `i / max(n-1, 1)`, a 0..1 position in the repeat |
| `x`, `y` | Pixel coordinates (only in per-pixel contexts: `expression` node, mask fields) |
| `w`, `h` | Size of the current object, or the canvas in per-pixel contexts |
| `$name` | Scene param or prefab input |
| `@…` | Reference (§7.5) |
| `pi`, `tau` | Constants |

### 9.3 Math functions

| Function | Returns |
|---|---|
| `abs(x)` `sign(x)` `floor(x)` `ceil(x)` `round(x)` `fract(x)` | Usual meanings; `round` rounds half away from zero |
| `min(a,b,…)` `max(a,b,…)` | |
| `clamp(x, lo, hi)` | |
| `lerp(a, b, k)` | `a + (b-a)*k` |
| `map(x, a, b, c, d)` | Remaps `[a,b]` to `[c,d]` (unclamped) |
| `step(edge, x)` | 0 if `x < edge`, else 1 |
| `smoothstep(a, b, x)` | Smooth 0..1 |
| `mod(a, b)` | Always non-negative |
| `sqrt` `exp` `log` `pow(a,b)` | |
| `sin` `cos` `tan` `atan2(y,x)` | Radians |
| `deg(r)` `rad(d)` | Unit conversion |
| `dist(x1,y1,x2,y2)` `len(x,y)` | Distances |
| `snap(x, step=1)` | Round to a multiple of `step` |

### 9.4 Periodic shapes
These take a **phase in cycles** (1.0 = one period).

| Function | Output |
|---|---|
| `wave(p)` | Sine, 0..1..0 (smooth) |
| `tri(p)` | Triangle, 0..1..0 |
| `saw(p)` | Ramp 0→1 |
| `square(p, duty=0.5)` | 0 or 1 |

### 9.5 Time helpers

| Function | Meaning |
|---|---|
| `cycle(period)` | Phase 0..1 of `t` within `period` |
| `osc(period, lo=-1, hi=1)` | Sine between `lo` and `hi` with the given period |
| `pulse(period, width=0.1)` | 1 during the first `width` fraction of each period, else 0 |
| `between(a, b)` | `true` while `a <= t < b` |
| `loop(expr, period)` | Evaluates `expr` with `t` wrapped into `[0, period)` |
| `delay(expr, d)` | Evaluates `expr` at `t - d` |
| `stagger(expr, step)` | `delay(expr, i*step)`: offsets each repetition |
| `hold(expr, frames)` | Updates the value only every `frames` frames |
| `on2(expr)`, `on3(expr)` | `hold(expr, 2)`, `hold(expr, 3)` |
| `ease(name, k)` | Applies an interpolation curve (§8 names) to `k` in 0..1 |

Time arguments accept time literals (`1.5s`) or strings (`'1.5s'`).

### 9.6 Randomness and noise

| Function | Output |
|---|---|
| `rand(k=0)` | 0..1, stable for (object seed, `k`) |
| `range(a, b, k=0)` | Uniform value in `[a,b]` |
| `randi(a, b, k=0)` | Integer in `[a,b]` |
| `chance(p, k=0)` | `true` with probability `p` |
| `pick(k, a, b, …)` | One of the arguments, chosen by `k` |
| `noise(x)` `noise(x,y)` `noise(x,y,z)` | Smooth value noise, 0..1 |
| `fbm(x, y, octaves=4)` | Fractal noise, 0..1 |
| `cells(x, y)` | Cellular / Voronoi distance, 0..1 |
| `wiggle(amp, freq)` | Smooth time noise in `-amp..amp`, `freq` wiggles per second |

**Determinism and loops:**
- Every random function hashes (global seed, object path, `k`). Nothing depends on wall-clock time or evaluation order.
- When the scene has `loop=#true`, every time-dependent noise (`wiggle`, and `noise`/`fbm` fed with `t`) is sampled on a circle whose period is the loop length. Wiggles therefore loop seamlessly with no extra work.

### 9.7 Pixel snapping

| Property type | Snapping |
|---|---|
| Positions, sizes, offsets | Rounded to whole pixels after all math, keyframes and camera (round half away from zero) |
| Angles (rig bones) | Snapped to the body plan's `angle-step` (default 1deg) before rasterization |
| Tones and light steps | Integer; the fractional remainder is dithered |
| Scales | Integer only (an error if not) |
| Everything else | Not snapped |

Sub-pixel values never accumulate between frames, because each frame is computed from scratch.

### 9.8 Errors at evaluation
- Division by zero and `NaN` produce `0` and a warning that names the object and property (reported once per property).
- A wrong type (string vs. number) is a load-time error when it can be detected; otherwise a warning plus the property's default.

---

## 10. Structure nodes: group, repeat, scatter, clone

### 10.1 `group`
A container with its own transform, seed and visibility. It has no visuals of its own.
```kdl
group "stall" x=80 { awning; crate; crate x=6 }
```

### 10.2 `repeat`
```kdl
repeat n=18 layout="row" gap=0 {                       // B1 books
    book width="2..4" height="=10+rand(i)*6"
}
repeat n=7 layout="path" path="@reef-edge" {           // B5 kelp along a curve
    kelp height="=range(30,60,i)" sway="=stagger(osc(3s),0.2s)"
}
```

| Prop | Meaning |
|---|---|
| `n` | Count |
| `layout` | `row`, `column`, `stack`, `grid`, `radial`, `path` |
| `gap` | Spacing in px |
| `align` | `start`, `center`, `end`, `baseline` |
| `cols` / `rows` | For `grid` |
| `radius`, `start-angle` | For `radial` |
| `path` | For `path` (a `path` object or a socket) |
| `fit` | `#true`: stop when the parent socket is full instead of after `n` |

### 10.3 `scatter`
```kdl
scatter n=40 area="@lagoon.water" min-dist=6 {          // B5 fish school seed positions
    fish body="fish-small" facing="=pick(i,'left','right')"
}
scatter density=0.02 area="ground" mask="=fbm(x*0.05,y*0.05)>0.5" {   // B6 flowers
    flower kind="=pick(i,'daisy','tulip')"
}
```

| Prop | Meaning |
|---|---|
| `n` or `density` | Fixed count, or items per pixel² of area |
| `area` | A socket, a `rect "x y w h"`, or an object id |
| `mask` | Per-pixel expression; items are placed only where it is true |
| `min-dist` | Poisson-disk minimum distance |
| `align` | `ground` (stand on the area's bottom line) or `free` |
| `sort` | `y` (lower items drawn in front; default in top-down and isometric views) or `none` |

### 10.4 `clone`
Copies another object, with overrides and a new seed by default:
```kdl
clone "@hero" id="hero-2" x=120 seed=99 materials cloth-a="cloth-blue"
```

---

## 11. Prefabs, sockets and placement

### 11.1 Declaring a prefab
```kdl
prefab "street-lamp" views="side perspective" {                    // B3
    input height 60 min=30 max=120
    input on #true
    slot pole default="metal"
    slot glass default="warm"
    socket top kind="point" x=0 y="=-$height"
    state on values="on off"
    rect w=2 h="=$height" material="pole" anchor="bottom-center"
    ellipse w=6 h=4 y="=-$height" material="glass" emissive="=$on"
    light-cone "beam" y="=-$height" angle="90deg" reach=50 intensity="=$on?1:0"
}

prefab "barrel" kind="model" {                                     // works in every view
    input height 12
    slot wood default="wood"
    slot band default="metal"
    cylinder r=5 h="=$height" material="wood"
    torus r=5 thickness=1 y=2 material="band"
    torus r=5 thickness=1 y="=$height-2" material="band"
}
```

| Element | Meaning |
|---|---|
| `views` | Supported views; ignored for `kind="model"` (which supports all) |
| `kind` | `shape` (2D, default) or `model` (3D primitives rendered to pixel; see RENDERING.md) |
| `input` | Parameter, same props as `param` |
| `slot` | Material slot and its default role |
| `socket` | Attachment point (`point`, `line`, `area`) |
| `state` | A discrete mode with named values |
| `emits` | Optional explicit list of emitter ids (otherwise detected) |
| `time` | `static` or `animated` (otherwise detected) |
| body nodes | Any objects, primitives, structure nodes and FX |

**2D primitives:** `rect`, `ellipse`, `polygon`, `line`, `path`, `sdf` (with `union`, `subtract`, `smooth-union`), `fill` (pattern or noise).
**3D primitives (in `model` prefabs):** `box`, `cylinder`, `sphere`, `capsule`, `cone`, `wedge`, `torus`, plus `csg` (`union`, `subtract`).

### 11.2 Sockets
```kdl
socket top     kind="line"  x1=0 y1=0 x2="=$width" y2=0
socket seat    kind="point" x=10 y=-14
socket screen  kind="area"  x=3 y=3 w="=$width-6" h="=$height-8" clip=#true
socket level   kind="line"  count="=$levels" x1=2 y1="=-i*$height/$levels" x2="=$width-2" y2="=-i*$height/$levels"
                                                     // count= generates level-1 … level-N (i = 1..N)
```

### 11.3 Placement properties

| Prop | Meaning |
|---|---|
| `on="id.socket"` | Attaches to a socket. Example: `on="tv.top"`, `on="boat.deck"` |
| `at="40%"` / `at="12"` | Position along a `line` socket, or inside an `area` socket as `"x% y%"` |
| `socket="name"` | When written as a child of a prefab: fills that socket of the parent |
| `offset="dx,dy"` | Extra pixel offset after attaching |
| `follow=#true` | Default. Follows the socket when it moves (a boat bobbing, B5) |

When several objects share a `line` socket without `at`, they are laid out left to right without overlapping. An object attached to a socket inherits the host's layer and draws just in front of the host.

---

## 12. Body plans, clips and state machines

The full semantics are in [ANIMATION.md](ANIMATION.md); built-in body plans are in [CATALOG.md](CATALOG.md).

### 12.1 `body-plan`
```kdl
body-plan "hero" template="biped" {                 // B4
    build height=40 bulk=0.5 head-ratio=0.25 angle-step="5deg"
    facings "s se e ne n" mirror="e->w se->sw ne->nw"
    slot armor default="metal"
    outfit top="tunic" bottom="pants" shoes="boots"
    piece "helmet-round" anchor="crown"
}
```

| Template | Typical use |
|---|---|
| `biped` | Humans, humanoids, robots, B4 hero |
| `quadruped` | Cats, dogs, deer (B2), horses |
| `bird` | Perched and flying birds (B2) |
| `fish` | Fish, whales (B5) |
| `serpent` | Snakes, eels, worms, dragons' tails |
| `multi-leg` | Insects, spiders, crabs |
| `machine` | Vehicles, mechs, doors, cranes (B3 car): wheels, hinges, pistons |

A body plan either uses `template=` and overrides parts, or declares everything itself with `bone`, `volume`, `anchor`, `facing`, `posture` and `clip` children.

### 12.2 `clip`
```kdl
clip "attack-slash" for="biped" channel="arms" duration="0.5s" {
    k "0s"    upper-arm-r="-150deg" forearm-r="-20deg"
    k "0.15s" upper-arm-r="40deg"   forearm-r="0deg"   interp="ease-out"
    k "0.5s"  upper-arm-r="0deg"                       interp="smooth"
    event "0.15s" "hit"
}
```

### 12.3 `machine`
```kdl
machine initial="idle" {
    state idle clip="idle" { go-to "walk" when="=$speed>0" }
    state walk clip="walk" { go-to "idle" when="=$speed==0"; go-to "jump" on="jump" }
    state jump clip="jump" { go-to "idle" after="clip-end" }
}
```

| Transition prop | Meaning |
|---|---|
| `after` | A time, a range (`"3s..6s"`) or `"clip-end"` |
| `when` | A boolean expression |
| `on` | An event name (from clips, scripts or other objects) |
| `prob` | Probability when several transitions are eligible |
| `blend` | Crossfade length (pose blending, snapped) |

---

## 13. Tiles (game output)

### 13.1 `tileset`
```kdl
tileset "village-ground" size=16 view="isometric" palette="morning" {     // B6
    terrain grass  { fill "=fbm(x*0.2,y*0.2)" material="foliage" }
    terrain dirt   { fill noise=0.3 material="soil" }
    terrain water  { water-surface animated=#true frames=4 }
    autotile grass over="dirt"  rules="blob-47"
    autotile water over="grass" rules="wang-16"
    tile "well" model="@well"            // a model prefab baked into a tile
}
```

| Element | Meaning |
|---|---|
| `size` | Tile size in px (square in side and top-down views; isometric uses `size` × `size/2`) |
| `terrain` | A procedurally textured terrain type |
| `autotile` | Generates every transition tile between two terrains |
| `rules` | `blob-47`, `wang-16`, `edge-9`, `corner-4` |
| `tile` | A single tile from a prefab, a model or a piece |
| `variants` | Number of seeded variations per tile (default 3) |
| `animated`, `frames` | Animated tiles (water, torches) |

### 13.2 `tilemap`
```kdl
tilemap "village" tileset="village-ground" size="40 30" {
    fill "grass"
    paint "dirt" path="@main-road" width=2
    paint "water" mask="=fbm(x*0.1,y*0.1)>0.7"
    stamp "@house" at="12 8" ; stamp "@house" at="22 10" seed=3
}
```
Tilemaps can be placed in scenes like any object, and exported (DIRECTOR.md).

---

## 14. Sequences and exports (summary)

```kdl
sequence "rain-short" fps=12 {
    shot "neon-street" duration="8s" { camera { k "0s" x=0; k "8s" x=24 } }
    transition "dither" duration="1s"
    shot "night-room" duration="6s"
}

export "hero-sheet" kind="sprite-sheet" source="@hero" {                    // B4
    clips "idle walk run jump attack-slash"
    directions 8
    frame-size 48 48
    layout "rows-by-clip"
    meta "json" "aseprite"
}
export "forest-layers" kind="layers" source="dusk-forest" frames="0..119"  // parallax export
export "village-tiles" kind="tileset" source="village-ground"
export "rain-loop" kind="video" source="rain-short" format="mp4" scale=4
```

**Export kinds:** `video` (GIF, MP4, WebM, APNG, WebP), `frames` (PNG sequence), `sprite-sheet`, `effect-sheet`, `tileset`, `layers`. See DIRECTOR.md.

**Reusing scenes in shots.** Inside a `shot`, `use-scene "cozy-room" without="dad kid cat"` instantiates another scene's content, minus the listed ids, so a story can revisit a set with a different cast (B8).

---

## 15. Validation, errors and versioning

### 15.1 Severity rules

| Problem | Severity | Behavior |
|---|---|---|
| KDL syntax error | Error | The file isn't loaded; the previous valid version stays in the preview |
| Unknown node name | Error | Suggests the closest name |
| Unknown property | Warning | Ignored |
| Value out of range | Warning | Clamped |
| Wrong value type | Error if structural, else warning | Warnings fall back to the default |
| Missing reference (`@id`, socket, pack name) | Error | Suggests the closest match |
| Reference cycle | Error | Shows the chain |
| Duplicate id in a scope | Error | |
| Ambiguous bare name across packs | Error | Asks for a qualified name |
| Prefab used in an unsupported view | Error | Lists the supported views |
| Non-integer scale | Error | |
| Time that isn't a whole frame | Warning | Rounded |
| Object without an id is referenced or animated | Warning | |
| Palette problems | See PALETTES.md §7 | |
| Piece problems | See PIECES.md §12 | |

Message format:
```
scenes/forest.kdl:14:5 error unknown node "pinetree". Did you mean "tree kind=\"pine\""?
scenes/forest.kdl:22:18 warning "roughness" out of range (0..1), clamped to 1.
```
`--strict` (CLI) turns warnings into errors.

### 15.2 Versioning
- `format <major>` is the first node of `project.kdl` and `pack.kdl`. Other files inherit it.
- **Minor changes** (new nodes or properties) are backward compatible and need no version change.
- **Major changes** ship with a migration: `pxe migrate` rewrites files in place, keeping comments (see CLI.md).
- Deprecated names keep working for one major version, with a warning and the suggested replacement.
- Packs declare the format they need: `pack "nature" { requires-format ">=1" }`. A mismatch is an error at `use`.

---

## 16. Behavior, events and simulation properties

Properties and nodes used by scripts ([SCRIPTING.md](SCRIPTING.md)), the director ([DIRECTOR.md](DIRECTOR.md)) and stateful simulation ([ANIMATION.md](ANIMATION.md)).

| Name | Where | Type | Meaning |
|---|---|---|---|
| `tags` | Any object | string (space-separated) | Labels scripts can query (`tags="puddle wet"`) |
| `overlap` | Any object | bool | Emits `overlap:start` / `overlap:end` events when its bounds touch another `overlap` object |
| `timeline` | `scene`, `shot` | node | Holds `marker` and `cue` children |
| `marker` | inside `timeline` | `marker "name" at="2s"` | A named point shown in the editor timeline |
| `cue` | inside `timeline` | `cue "name" at="1.4s" …` | A marker that also emits a `cue` event to scripts. Extra properties (`value=`, `line=`, `data=`) become the event data. Built-in cue names drive characters directly: `expression`, `talk`, `clip` (B7) |
| `warmup` | `scene` | time | Simulates stateful nodes before frame 0 so loops start mid-motion |
| `loop-blend` | `scene` | time | Dithered cross-fade from the last frames into the first, for stateful scenes that can't loop exactly |
| `checkpoint-every` | `project` | frames (default 60) | How often stateful nodes save checkpoints |
| `golden` | `scene` | `golden frames="0 30 60"` | Frames compared by `pxe bench` ([CLI.md](CLI.md)) |

```kdl
scene "street" view="perspective" palette="neon-night" warmup="2s" loop-blend="6f" {
    timeline {
        marker "car-enters" at="3s"
        cue "honk" at="3.5s"
    }
    puddle tags="puddle" overlap=#true
    golden frames="0 48 96"
}
```

---

## Appendix A. What the loader accepts today

The sections above describe the full format. This appendix lists the subset `px-format` loads now (2026-10-28). Everything else is reported as an unknown node or property, with a "Did you mean …?" hint. See `examples/` for working scenes.

### A.1 Scene children

| Node | Animatable props (defaults) | Other props | Notes |
|---|---|---|---|
| `param` | | §7.6 | Top level of the scene only |
| `wind` | | `strength` 1, `gust` 0.5, `speed` 16 | Field read by `wind(x)`, particles and `rope` |
| `ambient` | `darkness` 0 | `occlusion` 0..4, `key-light` 225 (degrees) | Darkness lowers every non-emissive pixel by tone steps; occlusion darkens pixels next to closer shapes; `key-light` is the direction dome shading is lit from |
| `camera` | `x` 0, `y` 0 | | Multi-property keys: `camera { k "0s" x=0; k "4s" x=24 }` |
| `layer "name"` | `fog` 0 | `parallax` 1, `fog-paint` | Children are drawn in this layer; layers can't nest. Fog moves tones into the fog paint's ramp, toward its tone |
| `group "name"` | `x y width height` 0 | `clip`, `round` | With `clip=#true`, children are clipped to the box (an ellipse if `round`). Clips nest up to 8 deep |
| `light-point` | `x y` (required), `reach` 40, `intensity` 1 | `color`, `shadows` | `intensity` is in tone steps at the center, falling off linearly to `reach` |
| `light-cone` | as `light-point` + `angle` 90°, `spread` 60° | `color`, `shadows` | 0° points right, 90° down. The outer part of the spread fades out |
| `light-rect` | `x y width height` (required), `reach` 40, `intensity` 1 | `color`, `shadows` | Windows, screens, signs |
| `reflection` | `x y width height axis` (required), `ripple` 1, `speed` 2, `tone-shift` -1, `opacity` 1 | `on` (ramp) | Mirrors the scene above `axis` into the box. With `on="water"`, only pixels of that ramp receive it, so floating objects stay on top |

`color` names a light color declared by `twin` in the palette (A.4). A pixel lit at least one step by a colored light switches to its ramp's twin. With `shadows=#true`, shapes marked `casts-shadow` block the light for anything behind them.

### A.2 Shapes

Every shape takes `paint` (`"ramp"` or `"ramp.tone"`), except `shade`, and the style props in A.3.

| Node | Animatable props (defaults) | Other props |
|---|---|---|
| `rect`, `ellipse` | `x y width height` | |
| `rounded-rect` | `x y width height`, `radius` 2 | |
| `line` | `x1 y1 x2 y2`, `width` 1 | Width ≤ 1.5 draws a pixel-perfect line, wider draws a capsule |
| `capsule` | `x1 y1 x2 y2`, `radius` 2 | |
| `polygon` | `x y` 0 | `points="0,0 8,0 4,6"` (at least 3, relative to `x y`) |
| `blob` | `x y` 0, `smooth` 3 | `circles="0,0,4 6,1,3"` (`x,y,radius`, smoothly merged) |
| `rope` | `x1 y1 x2 y2`, `sag` 0, `sway` 0, `width` 1 | Sags like a cable and bulges sideways by `sway` |
| `particles` | `x y width height` (spawn area), `vx vy gravity` 0, `life` 1s, `size` 1, `wind` 0 | `count` 20 (≤ 1024), `fade`. Stateless: each particle cycles through its life, so loops stay seamless when `life` divides the duration |
| `shade` | `x y width height`, `steps` -1 | `round`. Shifts the tones of whatever is below it |

### A.3 Style props (any shape)

| Prop | Kind | Meaning |
|---|---|---|
| `opacity` 1, `tone-shift` 0, `visible` 1, `contact-shadow` 0 | animatable | Opacity dithers; `visible` ≤ 0.5 hides the shape; `contact-shadow` draws a darkened ellipse under it |
| `shading` | `flat` \| `dome` \| `soft` | Form shading from the shape's distance field, lit from `ambient key-light`. `shading-steps` (default 1) sets the depth |
| `outline` | `none` \| `full` \| `selective` | `outline-paint` defaults to the palette's darkest tone; `selective` uses the shape's own darker tone on the lit side |
| `texture` | 0..1 | Clustered noise in tone steps; `texture-scale` 3 |
| `gradient` | `down` \| `up` \| `left` \| `right` \| `radial` | `gradient-steps` 2 (negative darkens). Solid bands with short dithered seams |
| `emissive` | bool | Ignores darkness and lights; keeps its tone in reflections |
| `casts-shadow` | bool | Blocks lights that have `shadows=#true` |
| `cleanup` | bool | Removes orphan pixels and notches from the shape's mask |

### A.4 Palette twins

```kdl
palette "night" {
    ramp "wall"      "#0d0b16" "#1b1828" "#2c2840"
    ramp "wall-warm" "#160c0d" "#2e1814" "#4f2c1e"
    twin "wall" warm="wall-warm"   // declares the light color `warm`
}
```

A twin must have as many tones as its base ramp. Up to 8 light colors per palette.

### A.5 Keyframes

`anim <prop>` works on every animatable prop above, with everything in §8: interpolations, relative times, `loop`, `blend`. A `replace` track also satisfies a required prop. References (`@id.prop`) see the animated value.
