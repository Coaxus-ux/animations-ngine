# Palettes & Ramp Naming (draft v1)

> Companion to [SPEC.md](SPEC.md) and [CATALOG.md](CATALOG.md).

The palette is law: nothing in the engine is drawn in RGB. Every pixel stores a **ramp** (a material) and a **tone** (a step on that ramp). Light, darkness, mood and transitions are all operations on ramps and tones. RGB is resolved only at output.

---

## 1. Concepts

| Term | Meaning |
|---|---|
| **Ramp** | An ordered list of colors from dark to light for one material (wall, wood, skin…) |
| **Tone** | A position on a ramp |
| **Role** | A standard, semantic ramp name that prefabs and body plans default to (`wall`, `wood`, `skin`) |
| **Local name** | A palette's own descriptive ramp name (`plaster-blue`), mapped to roles with `alias` |
| **Group** | A set of interchangeable ramps; objects pick a member by seed (`books-cold`) |
| **Twin** | The ramp a pixel switches to when lit by a colored light (`wall` → `wall-warm`) |
| **Emissive ramp** | A ramp that ignores ambient darkness (screens, bulbs, lit windows) |
| **Light color** | A named color that lights use (`warm`, `cool`, `screen`) |

### 1.1 Ramp length and tone names
The standard ramp has **5 tones**:

| Index | Name | Use |
|---|---|---|
| 0 | `deep` | Core shadows, contact shadows |
| 1 | `shade` | Form shadow |
| 2 | `base` | The material's main color |
| 3 | `lit` | Lit side |
| 4 | `hi` | Highlights |

Ramps may have 2–8 tones. Tone names map proportionally, `base` is always the middle tone, and lighting clamps at the ends unless the ramp is chained (§4.3).

### 1.2 Referring to tones
- `wood` means the ramp at its `base` tone.
- `wood.shade` or `wood.lit` names a tone.
- `wood.0` … `wood.4` gives an absolute index.
- `wood.+1` / `wood.-1` is relative to `base`.

The dot is reserved for tones. A dash is part of the name: `skin-3` is the third skin ramp, while `skin.shade` is a tone of the current skin ramp. This replaces the earlier `hair-1` piece syntax.

---

## 2. Naming conventions

1. **English, lowercase, kebab-case.**
2. Pattern: **`<role>[-<variant>][-<n>]`**
   - **role:** what the surface is: `wall`, `wood`, `skin`, `cloth`, `books`
   - **variant:** a qualifier: `dark`, `light`, `warm`, `cold`, `side`, `lit`
   - **n:** an index in a series: `skin-3`, `books-cold-2`
3. **Groups** use the bare name; members add an index. `books-cold` is the group, `books-cold-1` … `books-cold-5` are its ramps.
4. **Twins** use the `-warm` / `-cool` / `-screen` variant matching their light color: `wall-warm`.
5. **Local names** describe the color (`plaster-blue`, `oak`, `navy-cloth`) and are connected to roles with `alias`. Prefabs only ever ask for roles.

---

## 3. The standard role set

Prefabs and body plans default to these names. Palettes must define every **tier 1** role. **Tier 2** roles fall back automatically when they're missing.

### 3.1 Tier 1: required (16)

| Role | Used by |
|---|---|
| `ink` | Absolute darkest color: deep shadow, unlit voids (1 tone allowed) |
| `outline` | Outlines; 2 tones minimum (dark side / light side, for sel-out) |
| `wall` | Room walls |
| `floor` | Floors |
| `wood` | Furniture, shelves, frames |
| `metal` | Lamps, appliances, brackets |
| `plastic` | Electronics cases, gadgets |
| `fabric` | Curtains, generic cloth, upholstery |
| `paper` | Paper, calendars, labels, posters |
| `glass` | Windows, glasses, screens when off |
| `foliage` | Plants |
| `skin` | Default skin |
| `hair` | Default hair |
| `warm` | Warm light (lamps, candles, heaters); emissive |
| `cool` | Cool light (moonlight, daylight through windows) |
| `screen` | Default screen glow; emissive |

### 3.2 Tier 2: standard with fallback

When a tier 2 role is missing, the engine uses its fallback. A fallback with a tone offset means "the same ramp, shifted": `wood.-1` is wood one step darker.

| Role | Fallback | | Role | Fallback |
|---|---|---|---|---|
| `wall-side` | `wall.-1` | | `cloth-red`, `cloth-orange`, `cloth-yellow`, `cloth-green`, `cloth-teal`, `cloth-blue`, `cloth-purple`, `cloth-pink`, `cloth-gray`, `cloth-cream`, `cloth-dark` | `fabric` |
| `ceiling` | `wall.-1` | | `fur-black`, `fur-white`, `fur-orange`, `fur-gray`, `fur-brown` | `hair` |
| `baseboard` | `wood.-1` | | `eye` | `ink` |
| `wood-dark` | `wood.-1` | | `screen-green` | `screen` |
| `wood-light` | `wood.+1` | | `screen-blue` | `screen` |
| `plastic-dark` | `plastic.-1` | | `glow-red` | `warm` |
| `ceramic` | `paper` | | `bulb` | `warm` |
| `stone` | `metal` | | `sky` | `cool` |
| `soil` | `wood-dark` | | `city-near` | `wall.-1` |
| `books-cold` (group) | `cloth-*` | | `city-far` | `sky.-1` |
| `books-warm` (group) | `cloth-*` | | `window-lit` | `warm` |
| `skin-1`…`skin-6` | `skin` | | `rain` | `glass.+1` |
| `hair-black`, `hair-brown`, `hair-blonde`, `hair-red`, `hair-gray`, `hair-white` | `hair` | | `steam`, `smoke` | `paper` |
| `highlight` | `paper.hi` | | | |

### 3.2.1 Pack roles (tier 2)
Roles used by the `nature`, `urban` and `water` packs (see [CATALOG.md](CATALOG.md)):

| Role | Fallback | | Role | Fallback |
|---|---|---|---|---|
| `bark` | `wood-dark` | | `brick` | `wall` |
| `grass` | `foliage.+1` | | `concrete` | `stone` |
| `leaves-autumn` | `cloth-orange` | | `asphalt` | `stone.-1` |
| `flower-*` (group) | `cloth-*` | | `neon-*` (group, emissive) | `screen` |
| `sand` | `paper.-1` | | `paint-*` (group) | `cloth-*` |
| `snow` | `paper.+1` | | `coral-*` (group) | `cloth-*` |
| `water` | `glass` | | `water-deep` | `glass.-1` |

Packs may add more roles, always with a fallback to an existing role (for example `banner-red` → `cloth-red`).

Skin ramps `skin-1` … `skin-6` go from lightest to deepest. Every shipped palette defines all six, so characters of any skin tone look right under every mood.

### 3.3 Tier 3: free
Anything else a palette defines is local and only used if a scene asks for it by name.

---

## 4. Light model on ramps

### 4.1 Ambient darkness
`ambient darkness=N` moves every non-emissive pixel **N tones down** its ramp.

### 4.2 Lights
Each light contributes **tone steps** at a pixel: intensity × falloff, rounded. The fractional part is dithered, which produces the dithered light edge.

### 4.3 Chaining (beyond the ends of a ramp)
A ramp can declare where it continues:
```kdl
ramp wall ... below="ink" above="paper"
```
Without chaining, tones clamp at 0 and at the top.

### 4.4 Tinting with twins
Colored light doesn't just brighten: past a threshold, the pixel switches to the ramp's **twin** for that light color, keeping the same tone index.
```kdl
twin wall warm="wall-warm" cool="wall-cool"
light-color warm ramp="warm" tint-at=1     // switch to twins after 1 step of warm light
```
This is how the benchmark's blue wall turns pinkish-tan inside the lamp's cone.

**Automatic twins.** If a ramp has no twin for a light color, the engine builds one at load time:
1. Mix each tone with the light color by `tint-strength` (default 0.35), in OKLab.
2. If the palette is `closed=#true` (the default), snap each result to the nearest existing palette color.
3. Otherwise, add the new colors to the palette.

Hand-written twins always win. Write them for the surfaces that matter most (in the benchmark: `wall`, `wood`, `paper`).

### 4.5 Emissive ramps
```kdl
emissive screen screen-green window-lit warm bulb glow-red neon-*
```
Emissive pixels ignore ambient darkness and are not darkened by shadows. A light source's own `glow` uses its emissive ramp.

---

## 5. Generating ramps (no color skills required)

> **Today (2026-11-01):** the editor's palette editor (F4 > Generate) builds a ramp from one base color with these rules in HSV (`px_core::hsv_ramp`: value spread by a contrast amount, hue turned toward cool for dark tones and warm for light ones, saturation falling toward the light end) and writes plain `ramp` lines; the `ramp-gen` node and OKLab steps are still planned.

Hand-picking 5 tones per ramp is an art skill. To avoid needing it, **`ramp-gen`** builds a ramp from a single base color using standard pixel art hue shifting:
- **Shadows** shift hue toward blue/purple and gain a little saturation.
- **Highlights** shift hue toward yellow and lose saturation.
- **Lightness** steps are perceptually even (OKLab).

```kdl
ramp-gen wood base="#3a3640" steps=5 hue-shift=12 contrast=0.8
group books-cold gen="#2b3f66 #2f5a5a #3e3a5e #4a5866 #5a4636" steps=4
```

| Param | Default | Meaning |
|---|---|---|
| `base` | — | The `base` tone color |
| `steps` | 5 | Ramp length |
| `hue-shift` | 10 | Degrees of hue rotation per step (toward cool below base, toward warm above it) |
| `contrast` | 1 | Lightness spread |
| `saturation` | 1 | Saturation multiplier |

You can tweak any generated tone afterwards with a hand-written `ramp`, which overrides the generated one.

---

## 6. Palette file format

```kdl
palette "night-blue" {
    meta mood="cozy night" based-on="benchmark" closed=#true

    // local ramps (5 tones, dark → light)
    ramp plaster-blue "#16283a" "#1f3649" "#2a4659" "#365669" "#46687a" below="ink"
    ramp-gen oak base="#3a3640" hue-shift=12
    group books-cold gen="#2b3f66 #2f5a5a #3e3a5e #4a5866" steps=4

    // roles → local ramps
    alias wall="plaster-blue" wood="oak"

    // twins and lights
    twin wall warm="wall-warm"
    light-color warm ramp="warm" tint-at=1
    light-color cool ramp="cool" tint-at=2
    emissive screen warm window-lit
}
```

*Today* the loader reads `ramp`, `twin` and `include "other"`, which copies another palette's ramps and twins in place, so a scene can be placed inside a host whose palette includes its own ([FORMAT.md A.4](FORMAT.md#a4-palette-twins)).

### 6.1 Imports
- **Lospec `.hex` / `.gpl`:** the importer clusters the colors into ramps by hue and sorts them by lightness. Then you name the ramps and alias them to roles. This step is a heuristic, so expect to adjust it. *Today* the editor imports both formats by this rule: colors with HSV saturation below 0.12 form `gray`; the rest are binned by hue into `red` (345–15°), `orange` (15–45°), `yellow` (45–70°), `green` (70–165°), `cyan` (165–200°), `blue` (200–260°), `purple` (260–300°) and `magenta` (300–345°), each sorted dark to light by value then luma and split into `family`, `family-2`, … past eight tones, in that order with `gray` last; roles are still named by hand.
- **Mapping file:** an imported palette plus a small KDL file of `alias` and `twin` lines.

---

## 7. Validation

| Check | Level |
|---|---|
| Missing tier 1 role | Error |
| Missing tier 2 role | Info ("using fallback `wood.-1`") |
| Unique color count above 64 | Warning (reusing colors across ramps is encouraged) |
| Adjacent tones too similar (ΔL in OKLab below a threshold) | Warning |
| `outline` not darker than every `base` tone it borders | Warning |
| Emissive ramp darker than its surroundings at `base` | Warning |
| Twins with a different length than their ramp | Error |

The editor's palette panel shows every ramp as a strip, with role badges, color count and warnings.

---

## 8. Palette operations (director and scripts)

| Operation | Effect |
|---|---|
| **Swap** | Instantly use another palette. Works because roles match |
| `palette-black` / `palette-white` | Step every ramp down (or up) one tone per N frames |
| `palette-swap` transition | Each pixel switches from palette A's color to palette B's color for the same role and tone, in a dithered pattern over time. No new colors are blended |
| **Time of day** | A keyframed `palette-swap` between moods (for example `dusk` → `night-blue`) |
| **Local override** | One object can use another palette's ramps (`palette-override="dusk"`) |

All operations work on role and tone indices, so the result always stays inside the palettes and keeps the pixel art look.

---

## 9. Shipped palettes

> **Today (2026-11-01):** the editor ships four small presets for new files (`basic-16`, `gameboy-4`, `gray-8`, `dusk-12`, in `crates/px-editor/src/assets/palettes/`), with ramps only and no roles yet. The palettes below are the planned ones.

Every shipped palette defines all tier 1 roles, `skin-1` … `skin-6`, the `books-*` groups, every `cloth-*` role, the exterior roles and twins for `wall`, `wood`, `paper` and `skin`.

| Palette | Mood | Character |
|---|---|---|
| `night-blue` | Cozy night (the benchmark) | Cold blues everywhere, cyan window, one warm accent |
| `dusk` | Sunset | Purple-magenta walls, orange sky, long warm light |
| `morning` | Soft day | Pale warm walls, soft blue shadows, cool window light |
| `rainy-gray` | Overcast | Desaturated gray-greens, low contrast, faint warm interiors |
| `winter-night` | Snowy night | Teal and white, very warm windows |
| `neon-night` | Rainy city night (B3) | Near-black blues and purples, saturated emissive `neon-*` ramps, wet reflections |
| `deep-sea` | Underwater (B5) | Teal-to-navy depth ramps, pale caustic highlights, warm `coral-*` accents |
| `spring`, `summer`, `autumn`, `winter` | Seasonal daylight | Share role names, so nature scenes can be batch-exported per season |
| `mono-4` | Handheld retro | One 4-tone green ramp aliased to every role (proves the system works under extreme limits) |

### 9.1 `night-blue` (tier 1, starting values)
These values are sampled from the benchmark mood. They're a starting point to tune once the renderer exists.

| Role | deep → hi |
|---|---|
| `ink` | `#0a121b` |
| `outline` | `#0f1b26` `#1a2a38` |
| `wall` | `#16283a` `#1f3649` `#2a4659` `#365669` `#46687a` |
| `floor` | `#111e2a` `#182936` `#213443` `#2b4050` `#384e5d` |
| `wood` | `#1d1f2a` `#2a2a33` `#3a3640` `#4d4650` `#625a63` |
| `metal` | `#1a2530` `#27343f` `#37474f` `#4b5c63` `#6a7d82` |
| `plastic` | `#1c2833` `#2a3a47` `#3b4f5c` `#506673` `#6b8290` |
| `fabric` | `#18262a` `#213535` `#2d4643` `#3c5953` `#4e6d64` |
| `paper` | `#3a4c5a` `#4f6372` `#687c89` `#8597a1` `#a6b5bc` |
| `glass` | `#1e4a63` `#2b6585` `#3d82a6` `#5aa3c4` `#86c7df` |
| `foliage` | `#142a26` `#1c3a31` `#27493b` `#335a45` `#44704f` |
| `skin` | `#3b2b33` `#5a3e43` `#7d5552` `#a06e63` `#c08b78` |
| `hair` | `#0e141c` `#18202b` `#232d3a` `#303b49` `#3f4b5a` |
| `warm` ✦ | `#6b3a2a` `#a85a3a` `#d98552` `#f2b07a` `#ffd8a8` |
| `cool` | `#2f4f6f` `#3f6a8f` `#5287ad` `#74a8c8` `#a2cde0` |
| `screen` ✦ | `#0f2a1a` `#1c4d2c` `#2d7a43` `#4fb062` `#9be39a` |
| `wall-warm` (twin) | `#4a3436` `#6b4744` `#8e5f55` `#b07c69` `#cf9c82` |

✦ = emissive.

This set has about 80 unique colors, which is above the 64 guideline. Tuning should merge near-duplicate tones across ramps (for example, `floor.deep` and `outline.0`).

---

## 10. Decisions in this doc
- 5-tone standard ramp with named tones (`deep`, `shade`, `base`, `lit`, `hi`).
- The dot means a tone (`wood.shade`); the dash is part of a name (`skin-3`).
- Prefabs ask for **roles**; palettes map local names to roles with `alias`.
- 16 required roles, everything else falls back.
- Colored light switches pixels to **twin** ramps; missing twins are generated and, by default, snapped to existing palette colors.
- `ramp-gen` builds ramps from one base color with hue shifting, so no color skills are required.
- Transitions never blend new colors; they dither between palette colors.
