# Shorter scene files

Seven rules make KDL scenes shorter. A load-time pass in `px-format` (`crates/px-format/src/expand/`) rewrites them into plain nodes before the loader reads the scene, so the engine never sees them and a scene written either way renders the same pixels (D-027).

```kdl
motif pier x {
    rect $x 112 12 26 paint=stone.base texture=0.3 texture-scale=2 shading=dome outline=selective casts-shadow=#true
    rect "=$x-2" 134 16 4 paint=stone.shade outline=selective
}
motif bay x {
    pier $x
    rope "=$x+10" 116 "=$x+66" 116 sag=-9 width=5 paint=stone.base texture=0.3 texture-scale=2
}

scene "viaduct" palette="stone" width=320 height=180 {
    // 20 lines become 2 calls
    bay "=-40 + $k*64" repeat=6
    pier 344
    // 44 lines become 1
    rect "=-60 + $k*10" 112 6 2 paint=stone.shade repeat=44
}
```

## 1. Bare strings

KDL v2 reads `paint=moon.hi`, `shading=dome` and `paint=$color` as strings, with no quotes. Times, expressions and anything with spaces stay quoted: `"6s"`, `"=t * 2"`.

## 2. Bare values fill coordinates

| Kinds | Slots, in order |
|---|---|
| `rect ellipse rounded-rect shade particles group light-rect reflection` | `x y width height` |
| `line capsule rope` | `x1 y1 x2 y2` |
| `blob polygon text use-scene light-point light-cone camera` | `x y` |

`layer`, `ambient` and `pixels` keep named props. Fewer values fill the first slots; the rest stay named or take their defaults.

The first string decides what the rest mean:

| Node | First string | Example |
|---|---|---|
| `text`, `use-scene` | Always the words or the scene | `text "=" 1 2 paint=ink` |
| Any other kind, plain string | The id | `rect "body" -58 -25 58 20 paint=$color` |
| Any other kind, `"=…"` or `$name` | A coordinate | `rect "=@a.x + 5" 2 3 4 paint=red` |

A range or percent is a plain string, so as a first value it would read as an id: write `ellipse x="5..25" y=44 width="8..14" height=6`. Inside a motif, `blob $x $y` is always a coordinate, whatever the argument holds.

## 3. `repeat=N`

Writes the node `N` times, in place, with `$k` from 0 to `N`-1. It works on shapes, `group`, `use-scene` and motif calls. `$k` reaches everything inside the node, and the nearest `repeat` wins.

- `N` is a whole number from 1 to 4096 known at load. When a param sets the count, use `use-scene count=`.
- It can't go on `layer`, `pixels`, `camera`, `ambient`, `param`, `wind`, `timeline` or `anim`.
- An id used by every copy is an error, as usual. Build one: `"lamp$k"` gives `lamp0`, and `@lamp2.x` reads it. Avoid `"lamp-$k"`: an expression can't reference `@lamp-2`, because `-` before a digit is a minus.

## 4. `motif`

A named block of nodes, at the top level of any file. Calling it writes those nodes at that spot in the drawing order.

```kdl
motif window x hair="hair" coat="coat" {
    rect "=$x-1" -20 12 11 paint=ink
    use-scene "passenger" "=$x+4" -6 scale=0.7 hair=$hair coat=$coat
}

scene "carriage" palette="train" width=72 height=36 {
    window -54
    window -42 hair=brass
}
```

| Header | Meaning |
|---|---|
| `motif name a b` | `a` and `b` are required params |
| `motif name a b=3` | `b` is optional, with a default |

| Call | Meaning |
|---|---|
| `name 1 2` | Bare values fill the required params in order, except a param named `paint` |
| `name 1 b=4` | Optional params and `paint` go by name |

- **What a body sees.** Its own params. A `$name` it doesn't bind is left for the scene's params, and `@id.x` resolves in the scene that calls it.
- **Sharing.** A motif is visible through `use`, like a scene. Names are global across a load, and a duplicate is an error naming both files: put a shared motif in one file, or give it a family prefix such as `train-wheel`.
- **Nesting.** A body can call motifs, up to 8 deep.
- **Not in a body:** `layer`, `pixels`, `camera`, `ambient`, `param`, `wind`, `timeline`, `motif`, or an `anim` directly in the body. An `anim` inside a shape is fine.
- **Not as a name:** any node kind or format keyword. **Not as a param:** `k` or `repeat`.

## 5. `scatter`

Writes its children `N` times, like `repeat`, and gives every copy its own seeded values. Use it for things placed at random, such as stars, pebbles or reeds, where the exact spots don't matter.

```kdl
scatter 22 seed=1 x="-20..330" y="-10..62" tone="base|base|hi" {
    rect $x $y 1 1 paint=moon.$tone emissive=#true
}
scatter 67 seed=3 step="0..2" h="14..28" {
    polygon "=-70 + $k*7 + $step" "=124-$h" points="5,0 10,$h 0,$h" paint=pine.base
}
```

| Prop | Meaning |
|---|---|
| The first value | How many copies, 1 to 4096 |
| `seed` | Which shuffle, a whole number (0 by default). Change it to get another layout |
| `name="a..b"` | `$name` is a whole number from `a` to `b`, both included, when both ends are whole; otherwise a fraction from `a` up to `b` |
| `name="a\|b\|c"` | `$name` is one of the choices: a number when it reads as one, text otherwise. Repeat a choice to make it likelier |

- `$k` counts the copies, so a row can be spaced evenly and jittered: `"=-70 + $k*7 + $step"`.
- Every value comes from the seed, the value's name and range, and the copy's number. The same text always scatters the same way, and writing other shapes before or after it never moves it.
- Each value is drawn on its own. To tie two together, derive one in an expression, as the pine's top `"=124-$h"`.
- The `scatter` node disappears at load: picking a copy selects the child line inside it, and editing that line changes every copy.
- A body can hold shapes, groups, placed scenes and motif calls, but not `layer`, `pixels`, `camera`, `ambient`, `param`, `wind`, `timeline`, `anim` or another `scatter`. A `scatter` can't take `repeat`.
- Unlike the other rules, `scatter` doesn't reproduce a hand-placed layout: turning a written run into a `scatter` moves its shapes.

## 6. `part`

A named piece drawn around its own (0, 0), at the top level of any file. A call places it like a shape, and parts place other parts, so small pieces add up to big ones: atoms (a pane, a post) make molecules (a window, a street lamp), molecules make organisms (a house, a block), and a scene places those.

```kdl
part lamp height=20 paint=iron {
    rect 0 "=-$height" 1 $height paint=$paint
    rounded-rect -1 "=-$height-4" 3 4 paint=glass.hi emissive=#true
    anchor top 0 "=-$height-2"
}
part bus-stop {
    lamp 0 0
    bench 12 0
    slot
}

scene street palette=town width=320 height=180 {
    bus-stop 40 150 id=stop
    bus-stop 200 150 scale=0.75 flip=#true { walker 4 0 }
    light-point attach=stop.top reach=40
}
```

| Header | Meaning |
|---|---|
| `part name a b=3` | As a motif: `a` is required, `b` optional with a default |

| Call | Meaning |
|---|---|
| `name 40 150` | The first two bare values are `x y`, where the part's (0, 0) goes; without them it sits at (0, 0) |
| `name 40 150 6` | Further bare values fill the required params in order, except `paint` |
| `scale=`, `flip=`, `seed=` | Size, mirror on x around its (0, 0), and the root of its randomness |
| `id=`, `attach=` | Names the placed part, and hooks it onto another object (FORMAT A.6) |
| `name 40 150 { … }` | The children go where the body says `slot` |

- **Body.** Written in the part's own coordinates, with `$param` as in a motif. It runs on the clock of the scene that places it and reads that scene's `param`s and timeline markers.
- **`slot`.** Marks where the children of a call are drawn, in the part's coordinates; it can sit inside a `group` or be passed on to an inner part (`house 0 0 { slot }`). The children see the caller's `$` values. A call with children to a part without `slot` is an error.
- **`anchor name x y`.** A named point with no pixels. From outside, `attach=stop.top` hooks onto the anchor `top` of the part placed with `id=stop`; a dotted path reaches into inner parts that have ids. It works in a scene too.
- **Ids are the part's own.** Two calls never clash, and `@id.x` inside a body reads the body's objects only. A piece that must read the scene's objects (`@sun.x`) is a `motif`.
- **Limits.** `x`, `y`, `scale`, `flip`, `id`, `attach`, `seed`, `k` and `repeat` can't be param names. A body can't hold what a motif's can't. Motifs and parts nest 8 deep together, and a scene places at most 4096 parts and scenes. A part call takes `repeat` and can sit in a `scatter`.
- **Under the hood** a call becomes a `use-scene` of a scene made for it with the host's header (D-031), so everything in FORMAT A.6 about placed scenes holds. `pxe timeline` leaves parts out.
- **Previewing.** A file of parts has no scene to open. Add one that places them, as `examples/kit-street.kdl` does for the town kit.

When to use which:

| | `motif` | `part` | `use-scene` |
|---|---|---|---|
| Own origin, `scale`, `flip`, `id`, `attach` | no | yes | yes |
| Children (`slot`) | no | yes | no |
| Reads the scene's `@id`s | yes | no | no |
| Own clock, loop, `start`/`speed` | no | no | yes |
| Header (palette, size, duration) | none | none | required |

## 7. Groups that move their children

A `group` with `x` or `y` and no `clip` moves everything inside it: its children's coordinates count from the group's point, and they follow it when it is animated. Moving groups nest.

```kdl
group bob y="=round(abs(sin(tau * t))) - 1" {
    rounded-rect coat -4 -26 9 14 paint=$coat
    ellipse head -4 -37 10 10 paint=skin
}
```

- `@coat.y` still reads what is written (-26); `attach=coat` lands on the moved point.
- A group with `clip=#true` is a window: its box and its children stay in scene coordinates. A clip group or a `reflection` can't go inside a moving group; put the moving group inside the clip.
- A group with no `x` or `y` moves nothing, as before.

## The shared kit

`examples/kit/` holds motifs that several scenes share and parts to build scenes from. Add a motif there only when it replaces definitions in at least two scenes. Add a part when it is a piece a new scene would start from, at the level it belongs to: an atom draws shapes, a molecule places atoms, an organism places molecules. Name everything `kit-…`, since motif and part names are global.

| File | Motifs |
|---|---|
| `kit/sky.kdl` | `kit-star x y paint=… visible=1`, a 1×1 emissive dot; `kit-twinkle x y period paint=… duty=0.15`, one that blinks off with `pulse`; `kit-flicker x y paint=… odds=0.85 rate=4 cycle=8 salt=0`, one shown by `chance` |
| `kit/town.kdl` | Parts, each drawn from the middle of its base. Atoms: `kit-pane`, `kit-sill`, `kit-shutter`, `kit-flowers`, `kit-door`, `kit-wall width height`, `kit-gable width half`, `kit-chimney` (`smoke=0` turns its smoke off), `kit-trunk`, `kit-crown` (sized with `scale`), `kit-post`, `kit-lantern`, `kit-picket`. Molecules: `kit-window` (slot, e.g. `kit-flowers`), `kit-tree`, `kit-fence count=6`, `kit-street-lamp` (anchor `top`, a warm light). Organisms: `kit-house` (slot on the roof line), `kit-block` (slot, lamp `id=lamp`) |
| `kit/town-palette.kdl` | `kit-town`, the ramps those parts paint with by default; use it or `include` it |

`paint` is always named, because every palette names its ramps its own way. Scatter them for a sky:

```kdl
use "kit/sky.kdl"
scatter 26 seed=1 x="-23..342" y="-19..82" tone="base|lit|hi" { kit-star $x $y paint=star.$tone }
```

## Substitution

| Where | Rule | Example |
|---|---|---|
| A whole value | Takes the argument's type | `y=$y`, `paint=$tint` |
| Inside `"=…"` | Numbers, `=` values and times only; negatives and `=` values go in parentheses | `"=$x-2"`, `osc($period, 0, 1)` |
| Inside any other string | Plain text | `points="0,0 $w,0 0,$h"`, `paint=$ramp.3` |

- A name is the same token as in expressions: `-` continues it only before a lowercase letter. `$x-2` and `$x-$r` subtract; `$x-r` is one name.
- A substituted expression with only constants and arithmetic becomes a number. Anything using `t`, `i`, `n`, `rand`, `$` or `@` stays an expression.
- Write constants first and `@ref` last, as `"=$x-2.75 + @sway.x"`, so float results match a handwritten `"=50.25 + @sway.x"` exactly.
- `x`, `y`, `w`, `h`, `i`, `n`, `u`, `t` and `f` are engine variables. A bare `x` in the body of a motif with a param `x` is a warning: you meant `$x`.

## Errors

All are reported at the node, or at the outermost motif call when the problem is inside a body, because that's the line in the file you're editing.

| Problem | Message says |
|---|---|
| More bare values than slots | The slots the kind takes |
| A slot given bare and by name | Which slot |
| `rect $w 2 3 4` with no motif binding `$w` | Write `"=$w"` |
| A bad call: missing, extra, unknown or repeated argument, or children | The motif's signature |
| `motif` or `part` not at the top level, or a name defined twice | Where the other one is |
| Children on a call to a part with no `slot`, or `slot` outside a part | The part's signature |
| Depth over 8, or a scene past 65,534 nodes | The limit |

Every loaded file is expanded, so a bad call in a scene you didn't select still stops the load.

## In the editor

Nothing writes the expanded nodes back, so shorthand survives every edit (D-014).

| Action | What happens |
|---|---|
| Pick a motif or part shape | Selects the call |
| Pick a `repeat` copy | Selects the template line; the status says how many copies change |
| Inspector | Bare values show as rows named after their slot or motif param. Typing a number or `=expression` writes that value in place; a plain string is refused in a first slot |
| Recolor a call | Writes `paint=` if the motif declares `paint`. Otherwise it refuses and points at the motif |
| Recolor a `repeat` | Writes `paint=` on the template line |
| Edit a motif or part body | Changes every call. A body has no pick of its own |
