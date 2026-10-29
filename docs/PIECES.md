# Pieces (draft v1)

> Companion to [SPEC.md](SPEC.md). Syntax rules shared with every KDL file are in [FORMAT.md](FORMAT.md); slots and tones in [PALETTES.md](PALETTES.md); body plans and templates in [CATALOG.md](CATALOG.md) and [ANIMATION.md](ANIMATION.md).

A **piece** is a small hand-written grid of characters for details that geometry does poorly: eyes, mouths, glasses, a logo, a rivet, a feather tip, a glyph of a pixel font. Pieces hold **slots, not colors**, so one piece works with any palette, character, creature or prop.

**Rule of thumb:** keep pieces at **16×16 or smaller**. Anything bigger should be built from shapes, SDF or `model` prefabs, which scale, light and vary better.

---

## 1. Where pieces live
- In `pieces/*.kdl`, in a pack's `pieces/` folder, or inline inside a `prefab` / `body-plan`.
- A file may hold any number of `piece`, `piece-set` and `font` nodes.
- Names resolve like every other name (FORMAT.md §1.4): project first, then packs, with qualified names such as `creatures:eye-slit`.

---

## 2. Anatomy

```kdl
piece "round-glasses" anchor="eyes" layer="over" pivot="3,1" {
    legend "#"=outline "o"=glass.lit
    variant "front" """
        #.##.#
        o#..#o
        """
    variant "side" """
        ##
        o#
        """
}
```

| Property | Default | Meaning |
|---|---|---|
| `anchor` | — | Default host anchor it attaches to (§6) |
| `pivot` | `"center"` | Pixel of the piece aligned to the anchor (§6) |
| `layer` | `"over"` | Draw order relative to the host part (§8) |
| `mirror` | `"none"` | Symmetry (§7) |
| `kind` | `"decal"` | `decal` (placed once), `pattern` (tiled over a region, §9) or `glyph` (font, §10) |
| `raw` | `#false` | `#true` skips the cleanup passes (§12) |
| `unlit` | `#false` | `#true` ignores scene lighting (like a printed light, or UI) |
| `when` | — | Visibility expression, for example `"=@self.state=='talk'"` |

---

## 3. Grid syntax

| Rule | Detail |
|---|---|
| Grid string | A KDL multi-line string (`"""`), dedented automatically. Single-row pieces may use a normal string: `"#"` |
| One character = one pixel | Any single Unicode character except space and tab |
| Spaces are not allowed | Use `.` for empty; avoids invisible mistakes. Error if found |
| Rows must have equal width | Shorter rows are an error (the message shows the row) |
| Origin | Top-left pixel is `0,0`; `x` grows right, `y` grows down |
| Max size | 64×64 (hard limit); a warning above 16×16 |

---

## 4. Legend

### 4.1 Default legend
Every piece inherits this legend, so simple pieces need none:

| Char | Means |
|---|---|
| `.` | `empty` (transparent: shows whatever is under it) |
| `#` | `outline` |
| `x` | `erase`: cuts the host's pixels (a mouth hole, an eye socket, a window cut-out) |
| `*` | `accent` |
| `@` | `ink` |
| `=` | `keep`: forces the host's own pixel to stay (protects it from other pieces) |

### 4.2 Custom legend
Short form: `legend "<char>"=<value> …`. A value is a slot, optionally with a tone: `skin.shade`, `hair.-1`, `glass.lit`, `metal.hi`.

Long form, for extra flags:
```kdl
legend {
    key "o" slot="glass" tone="lit" emissive=#true     // glowing visor, B4
    key "s" slot="scales" tone="+1"
    key "w" role="paper" tone="hi"                     // a palette role instead of a host slot
}
```

| Key prop | Meaning |
|---|---|
| `slot` | A slot of the host (body plan or prefab) |
| `role` | A palette role used directly (PALETTES.md §3), bypassing the host |
| `tone` | `deep`, `shade`, `base`, `lit`, `hi`, an index `0..n`, or a relative `+1` / `-1` |
| `emissive` | Ignores darkness and glows like an emissive ramp |
| `light` | Pixels are lit (default) or `unlit` |

### 4.3 Slots resolve through the host
Pieces name **slots** (`skin`, `hair`, `cloth-a`, `fur`, `scales`, `metal`…), and the host maps them to ramps.
- The same `eye-dot` piece is brown on one hero and green on another, and the same `scales` pattern is blue on a fish (B5) and green on a snake.
- If the host lacks a slot, it falls back: slot → `accent` → load error naming both the piece and the host.

**Universal slots** that every host provides: `outline`, `ink`, `accent`, `eye`, `glass`, `metal`.

---

## 5. Variants

### 5.1 Directional variants

| View | Variant names |
|---|---|
| `side` | `front`, `side`, `back` |
| 4-direction (top-down, isometric, game sheets) | `s`, `e`, `n`, `w` |
| 8-direction | `s`, `se`, `e`, `ne`, `n`, `nw`, `w`, `sw` |

- Side variants are authored facing **right**; facing left mirrors them automatically.
- Likewise `w`, `nw` and `sw` default to mirrors of `e`, `ne` and `se` unless they are written explicitly.

**Fallback order:**
1. An exact match.
2. The nearest direction (`se` → `s` or `e`, whichever is closer to the host's facing).
3. `side` ↔ `e`/`w`; `front` ↔ `s`; `back` ↔ `n`.
4. A variant named `default`.
5. The first variant.

### 5.2 State variants
Any name: `open`, `half`, `closed`, `smile`, `talk-a`, `talk-b`, `lit`, `broken`.
- Combine with a direction using `/`: `variant "side/closed"`.
- The host chooses the state with `state="closed"`, an expression (`state="=@hero.state=='hurt'?'pain':'open'"`) or a clip key (`k "0.2s" eyes="closed"`).

### 5.2.1 Size variants (`hd`)
A piece can carry a larger version for styles with `detail-scale` ≥ 1.5 ([RENDERING.md](RENDERING.md) §16.3) and for close-ups (B7). Combine it with other variant names using `/`: `variant "hd/side/closed"`. When there is no `hd` variant, the 1× piece is used and the validator reports it.

### 5.3 Rotation buckets
Pieces never rotate freely; that would destroy the pixels. When a piece is attached to a bone that rotates (a sword in a hand, B4), the engine picks the variant whose `angle` is closest:
```kdl
piece "sword" anchor="hand-r" pivot="1,7" {
    variant "0"  angle="0deg"  """ … """
    variant "45" angle="45deg" """ … """
    variant "90" angle="90deg" """ … """
}
```
Missing angles are generated with 90° rotations and mirrors when the grid allows it (multiples of 90°); otherwise the nearest authored angle is used.

---

## 6. Anchors, pivot and points

| Concept | Meaning |
|---|---|
| `anchor` | A named point on the host: body plan anchors (`eyes`, `crown`, `hand-r`, `fin-top`) or prefab sockets of kind `point` (`front`, `door`) |
| `pivot` | The piece pixel that sits on the anchor: `"x,y"`, or a keyword: `center`, `top-center`, `bottom-center`, `left-center`, `right-center` |
| `point` | Extra named points inside the piece, used by other systems |

```kdl
piece "wand" anchor="hand-r" pivot="0,5" {
    point tip x=4 y=0           // particles can emit from @hero.wand.tip
    variant "0" """ … """
}
```
An anchor can be overridden at use time: `piece "scar" anchor="cheek-l" offset="1,0"`.

---

## 7. Mirroring

| `mirror` | Behavior |
|---|---|
| `none` | As written |
| `x` | Write the left half; the engine reflects it to the right. `mirror-center=#true` when the width is odd and the middle column is shared |
| `y` | Write the top half |
| `xy` | Write the top-left quarter |

Mirroring is applied before variants are resolved, and it never mirrors text glyphs.

---

## 8. Layering

| `layer` | Draws |
|---|---|
| `behind` | Behind the whole host (a cape, a fish's far fin) |
| `under` | Just under the host part the anchor belongs to (hair behind the head) |
| `over` | Just over that part (default: eyes, glasses, decals) |
| `top` | Over the whole host (a helmet visor, a speech icon) |

- `z=<int>` breaks ties between pieces in the same layer.
- On body plans, the "part" is the anchor's bone, so a piece on `hand-r` follows the hand's draw order. In 3/4 or side facings that means it correctly hides behind the body when the arm is on the far side.

---

## 9. Patterns (`kind="pattern"`)

A pattern piece tiles over a **region** of the host instead of being placed once. The region is clipped by the host's volume, so stripes stay on the sweater and scales stay on the fish.

```kdl
piece "stripes-h" kind="pattern" {
    legend "a"=cloth-b
    variant "default" """
        aaaa
        ....
        ....
        """
}
```
Use:
```kdl
outfit top="sweater" pattern="stripes-h" pattern-on="cloth-a"     // B1
fish "parrotfish" { pattern "scales-small" on="body" }              // B5
```

| Use-time prop | Meaning |
|---|---|
| `on` | A slot (`cloth-a`), a bone (`body`) or a prefab region |
| `offset` | Shifts the tiling phase |
| `follow` | `world` (default for prefabs) or `bone` (default for bodies: the pattern moves with the part) |
| `density` | 0..1; drops tiles randomly by seed (worn paint, patchy fur) |

---

## 10. Fonts (`font` and `kind="glyph"`)

> **Today (2026-10-29):** `font` with `glyph` children and the `text` shape load and render (FORMAT.md appendix A.7). Glyphs use `#` and `.` only (no legend or slots yet), and a glyph's rows can also be written on one line, separated by spaces. `core-kit` doesn't exist yet; the built-in `5x7` font (`crates/px-format/fonts/5x7.kdl`) covers printable ASCII and is the default, and `bold`, `italic` and inline `[b]` `[i]` `[size=N]` tags style text.

Pixel text is just pieces. A font is a piece set where each glyph is a piece:
```kdl
font "tiny-3x5" height=5 spacing=1 baseline=5 {
    glyph "A" """
        .#.
        #.#
        ###
        #.#
        #.#
        """
    glyph "B" """ … """
}
```
Use:
```kdl
neon-sign text="OPEN" font="neon-5x7" color="screen" flicker=0.1    // B3
sign text="INN" font="tiny-3x5" on="@tavern.front"                     // B6
```
- Glyphs use `#` as the text slot (`ink` by default; the host can recolor it).
- Missing glyphs render as a box and produce a warning.
- `core-kit` ships `tiny-3x5`, `small-5x7`, `neon-5x7` and `title-9x11` (a serif display font for title cards, B8), all with uppercase, digits and basic punctuation.

---

## 11. Animated pieces

A variant can hold several frames:
```kdl
piece "torch-flame" anchor="top" unlit=#true {
    legend "a"=warm.hi "b"=warm.base "c"=warm.shade
    variant "default" fps=8 mode="loop" {
        frame """
            .a.
            aba
            bcb
            """
        frame """
            a..
            .ba
            bcb
            """
    }
}
```

| Prop | Meaning |
|---|---|
| `fps` | Frame rate of the piece (independent of the scene fps; snapped to scene frames) |
| `mode` | `loop`, `pingpong`, `once`, `manual` |
| `frame=` (at use time) | Drives the frame from an expression or a clip: `frame="=on2(cycle(1s)*4)"` |
| `phase` | Offsets the start; defaults to a seeded random value, so repeated torches don't flicker in sync |

---

## 12. Cleanup and validation

Pieces go through the same cleanup passes as generated shapes (pixel-perfect lines, orphan removal, sel-out) **unless** `raw=#true`. Pieces are already hand-placed, so in practice only sel-out (outline tinting by light) and lighting apply.

| Check | Severity |
|---|---|
| Space or tab inside a grid | Error |
| Rows of different widths | Error |
| Character not in the legend (custom or default) | Error |
| Slot missing on the host and no fallback | Error (at use) |
| Pivot outside the grid | Error |
| Piece larger than 16×16 | Warning |
| Animated variants with different frame sizes | Error |
| Variant name unknown to the host's facings | Warning (falls back, §5.1) |
| Orphan single pixels in a `pattern` | Info |

---

## 13. Piece sets and kits

```kdl
piece-set "face-dot" {
    legend "e"=eye "m"=skin.-1
    piece "eyes"  anchor="eyes" mirror="x" { variant "open" "e"; variant "closed" "m" }
    piece "mouth" anchor="mouth"           { variant "idle" "m"; variant "open" "x" }
}
```
- A set shares its legend with every piece inside it.
- Body plans reference sets as defaults (`face="face-dot"`), so a whole style can be swapped in one line (`face="face-big-eyes"`).
- Packs ship coherent sets per style, so characters within a style always match.

---

## 14. Pieces across body plan templates

| Template | Standard anchors for pieces | Typical pieces |
|---|---|---|
| `biped` | `crown`, `forehead`, `eyes`, `nose`, `mouth`, `ear`, `cheek-l/r`, `neck`, `chest`, `back`, `waist`, `hand-l/r`, `foot-l/r` | Faces, eyewear, hair details, headwear, held items, belts, capes |
| `quadruped` | `head-top`, `eyes`, `nose`, `mouth`, `ear-l/r`, `collar`, `back`, `flank`, `paw-*`, `tail-tip` | Eyes, whiskers, antlers, horns, collars, saddles, spots |
| `bird` | `crown`, `eyes`, `beak`, `throat`, `wing-tip-l/r`, `tail` | Beaks, crests, wing tips, eye rings |
| `fish` | `eyes`, `mouth`, `gill`, `fin-top`, `fin-bottom`, `fin-tail`, `body` | Eyes, gill lines, fin tips, scale patterns, lures |
| `serpent` | `head-top`, `eyes`, `tongue`, every segment `seg-N`, `tail-tip` | Eyes, tongues, rattles; patterns repeat along segments (`follow="bone"`) |
| `multi-leg` | `head`, `eyes`, `mandibles`, `back`, `leg-N-tip` | Eyes (clusters), shells, spots |
| `machine` | Sockets declared by the plan: `front`, `rear`, `door`, `hub-N`, `light-N`, `panel-N` | Headlights, tail lights, rivets, panel lines, decals, wheel hubs |

---

## 15. Pieces on props (decals)

Any prefab socket of kind `point` or `area` can receive pieces:
```kdl
bag on="floor" { decal "logo-cat" on="front" at="50% 40%" }          // B1
car "taxi" { decal "taxi-sign" on="roof"; decal "light-head" on="light-1" emissive=#true }   // B3
house "inn" { decal "sign-mug" on="front" at="70% 30%" }             // B6
```
Decals are clipped to the host's silhouette when placed in an `area` socket, and they follow the host's lighting.

---

## 16. Standard piece library

`core-kit` and `creatures` ship these pieces. The "Benchmarks" column marks which benchmarks use them (see [BENCHMARKS.md](BENCHMARKS.md)).

### 16.1 Faces (set `face-dot`, also `face-big-eyes`, `face-line`)
| Piece | Variants | Benchmarks |
|---|---|---|
| `eyes` | `open`, `half`, `closed`, `happy`, `angry`, `sad`, `surprised` | B1 B3 B4 |
| `eyes-side` | Same states, side view | B1 B3 B4 |
| `mouth` | `idle`, `smile`, `open`, `o`, `frown`, `talk-a`, `talk-b`, `grit` | B3 B4 |
| `brows` | `neutral`, `up`, `angry`, `worried` | B4 |
| `nose-bump` | Side profile nose | B1 B3 |
| `blush` | — | — |

### 16.2 Eyewear
| Piece | Benchmarks |
|---|---|
| `round-glasses` | B1 |
| `square-glasses`, `sunglasses`, `goggles`, `monocle`, `visor` (emissive) | B3 (`sunglasses`), B4 (`visor`) |

### 16.3 Hair details (on top of the parametric hair mass)
| Piece | Benchmarks |
|---|---|
| `fringe-straight`, `fringe-side`, `cowlick`, `hair-clip`, `braid-end`, `bun-pin` | B1 B4 |
| `beard-short`, `beard-long`, `mustache`, `sideburns`, `stubble` (pattern) | B3 |

### 16.4 Headwear
| Piece | Benchmarks |
|---|---|
| `cap`, `beanie`, `hood-edge` | B3 |
| `helmet-round`, `helmet-horned`, `crown`, `circlet` | B4 |
| `wizard-hat`, `headband`, `straw-hat`, `diving-mask` | B5 (`diving-mask`), B6 (`straw-hat`) |

### 16.5 Clothing patterns (`kind="pattern"`)
| Piece | Benchmarks |
|---|---|
| `stripes-h` | B1 |
| `stripes-v`, `dots`, `checker`, `plaid`, `zigzag`, `chainmail` | B4 (`chainmail`) |
| `knit` | B1 |

### 16.6 Clothing details
| Piece | Benchmarks |
|---|---|
| `collar`, `buttons`, `zipper`, `pocket`, `belt-buckle` | B1 B3 B4 |
| `scarf-end`, `cape-edge`, `shoulder-pad`, `bracer` | B4 |
| `raincoat-trim`, `apron` | B3 B6 |

### 16.7 Held items and accessories (with rotation buckets)
| Piece | Benchmarks |
|---|---|
| `sword`, `shield-round`, `bow`, `staff`, `torch` | B4 |
| `umbrella` (with an open/closed state), `phone` (emissive screen), `bag-strap` | B3 |
| `lantern`, `book`, `mug`, `fishing-rod`, `bucket` | B1 B5 B6 |
| `hoop-earring`, `stud-earring`, `necklace` (with `hd` variants) | B7 |

### 16.8 Decals and signage
| Piece | Benchmarks |
|---|---|
| `logo-cat`, `logo-star`, `logo-circle` | B1 B3 |
| Fonts `tiny-3x5`, `small-5x7`, `neon-5x7` | B3 B6 |
| `arrow`, `warning`, `poster-blocks`, `graffiti-tag` | B3 |
| Shop icons: `sign-mug`, `sign-bread`, `sign-anvil`, `sign-fish` | B6 |
| `crack`, `stain`, `moss-patch`, `rust` (patterns) | B2 B3 B6 |

### 16.9 Creature details
| Piece | Templates | Benchmarks |
|---|---|---|
| `eye-slit`, `eye-round`, `eye-glow` | quadruped, serpent, fish | B1 B2 B5 |
| `whiskers`, `ear-tuft`, `nose-cat`, `nose-dog` | quadruped | B1 |
| `antlers-small`, `antlers-large`, `horns`, `hooves` | quadruped | B2 |
| `beak-short`, `beak-long`, `crest`, `wing-tip`, `feather-tail` | bird | B2 |
| `gill-lines`, `fin-tip`, `lure`, `scales-small` / `scales-large` (patterns), `bubble` (animated) | fish | B5 |
| `tongue-fork`, `rattle`, `serpent-bands` (pattern) | serpent | — |
| `mandibles`, `eye-cluster`, `shell-spots` (pattern) | multi-leg | B5 (crab) |

### 16.10 Machine and vehicle details
| Piece | Benchmarks |
|---|---|
| `light-head`, `light-tail` (emissive), `wheel-hub`, `grille`, `mirror`, `taxi-sign` | B3 |
| `rivets` (pattern), `panel-lines` (pattern), `vent`, `antenna-light` | B3 |

### 16.11 Nature details
| Piece | Benchmarks |
|---|---|
| `flower-daisy`, `flower-tulip`, `mushroom`, `grass-tuft` | B2 B6 |
| `firefly` (animated, emissive), `leaf-fall` (animated) | B2 |
| `starfish`, `shell`, `coral-bit`, `seaweed-tip` | B5 |
| `cobble` (pattern), `roof-tile` (pattern), `brick-small` (pattern) | B6 |

### 16.12 Counts per benchmark

Pieces that need **new hand-writing** beyond the standard library:

| Benchmark | Library pieces used | New pieces needed |
|---|---|---|
| B1 cozy room | ~10 | 0 |
| B2 forest | ~10 | 0 |
| B3 neon street | ~18 | 0–2 (shop-specific signs) |
| B4 hero sheet | ~15 | 0–3 (a signature weapon or emblem) |
| B5 underwater | ~10 | 0 |
| B6 isometric village | ~12 | 0–2 (shop icons) |

---

## 17. Authoring tips
1. **Use few tones:** 2–3 per piece. Light and sel-out add the rest.
2. **Outline only where the piece meets something.** Use `#` on edges that face the background, and leave inner edges unoutlined.
3. **Prefer slots over roles,** so the piece recolors with the host. Use roles only for fixed colors (a red warning sign).
4. **Write one direction, mirror the rest:** `side`/`e` plus `mirror="x"` covers most cases.
5. **Check it in two palettes.** The editor's piece panel previews every piece in the active palette and in `mono-4`. If it reads in `mono-4`, it reads everywhere.
