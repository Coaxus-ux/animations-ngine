# Benchmarks (draft v2)

> Companion to [SPEC.md](SPEC.md). Uses content from [CATALOG.md](CATALOG.md), palettes from [PALETTES.md](PALETTES.md) and styles from [RENDERING.md](RENDERING.md) §16.

The engine is validated against **nine deliberately different benchmarks** plus a rotating set of **hold-out scenes** (§12). They exist to keep the design general.

**Rules**
1. **No benchmark-specific engine code.** If a benchmark needs something new, it must arrive as a generic feature (documented in SPEC/CATALOG/FORMAT with uses beyond that benchmark) or as pack content (KDL). Code review rejects any engine code that names a benchmark, scene or prefab.
2. **Benchmarks must include the engine's weak spots**, not only its strengths. B7 (close-up faces), B8 (multi-shot story with interacting characters) and B9 (other art styles) exist because they are hard.
3. **Pass criteria are measurable wherever possible** (§1). Taste is judged by a written rubric, never by "looks right to me".

| # | Benchmark | Content type | View | Style | Output |
|---|---|---|---|---|---|
| B1 | Cozy night room | Interior ambience, small characters | `side` | `moody` | Loop |
| B2 | Forest at dusk | Nature, organic generators, panning | `side` + parallax | `moody` | Loop + parallax layers |
| B3 | Rainy neon street | Urban, depth, colored lights | `perspective` | `moody` | Loop |
| B4 | Game hero + effects | Game assets, action | `side` + `top-down` 8 dir | `moody` | Sprite sheets + effect sheet |
| B5 | Underwater coast | Two light environments, swarms | `side` (split) | `moody` | Loop |
| B6 | Isometric village | Tiles, game maps | `isometric` | `moody` | Tileset + tilemap + loop |
| B7 | Character close-up | Large character, faces, dialogue | `side` (bust) | `moody` | Loop + clip |
| B8 | Short story | Shots, transitions, continuity, interaction | `perspective` + `side` | `moody` | MP4 sequence |
| B9 | Style switch | Same scenes, six art styles | as source | all presets | Stills + loops |

---

## 1. Automatic checks (all benchmarks)

Run by `pxe bench` in CI on every change ([CLI.md](CLI.md)). A benchmark passes only if every applicable check passes **for its main seed and for 5 random seeds**.

| Check | Metric | Threshold |
|---|---|---|
| **Palette compliance** | Pixels whose color is not in the active palette (and style `color-limit`) | 0 |
| **Orphan pixels** | Isolated pixels not owned by an intentional point source (stars, sparks, particles, pieces) | 0 |
| **Line quality** | L-corners on 1 px outlines and lines | 0 |
| **No resampling** | Non-integer scale or rotation operations reported by the renderer | 0 |
| **Determinism** | Indexed-buffer hash on two runs, two operating systems | identical |
| **Loop seam** (loops) | Pixel difference between last→first frame vs. the largest difference between consecutive frames | seam ≤ largest |
| **Foot planting** (walking rigs) | World-space drift of a planted contact (foot, paw, leg tip) | ≤ 1 px |
| **Pivot stability** (sheets) | Pivot drift across frames and directions | 0 px |
| **Validator** | Errors / warnings on load | 0 errors, warnings listed |
| **Preview speed** | Median frame time at the project resolution | ≤ 33 ms (30 fps) unless the benchmark sets another budget |
| **Authoring budget** | Lines of scene KDL (excluding shared prefab and cast files), hand-written pieces | per benchmark |

### 1.1 Human rubric (taste)
Two reviewers score 1–5 on each axis. A benchmark passes with an average **≥ 4** and no axis below 3.

| Axis | Question |
|---|---|
| Readability | Can you tell what every object and character is at 1× zoom? |
| Hand-made feel | Would a pixel artist accept this as hand-placed? Any "procedural" tells? |
| Motion | Does motion feel deliberate (timing, holds, weight), with no jitter or sliding? |
| Mood | Does the lighting/palette create the intended feeling? |
| Variety | Across the 5 seeds, are the results different yet equally good? |

---

## B1 — Cozy night room

**Description.** A dark blue bedroom-study at night: a person typing at a CRT, a child sitting on an old TV looking at rainy city windows, a black cat reaching up, shelves full of books, a warm arm lamp, a red heater. This is the user's original reference image.

| | |
|---|---|
| **Palette / style** | `night-blue` / `moody` |
| **Packs** | `core-kit`, `interior`, `creatures` |
| **Body plans** | `human-adult`, `human-child`, `cat` |
| **Authoring budget** | ≤ 60 lines, ≤ 2 pieces |

```kdl
scene "cozy-room" view="side" palette="night-blue" duration="12s" loop=#true {
    room corner="left" wall-texture="speckle"
    window x=140 y=60 { sky kind="night"; city density=0.7; rain; window-rain }
    shelf x=0 levels=4 books="dense" top-clutter=0.6
    desk "desk" x=10 { crt-monitor on="desk.top" { code-scroll }; keyboard "kb" on="desk.top" }
    chair "chair" x=30 facing="back"
    desk-lamp mount="clamp" x=190 on=#true
    heater x=8 on=#true
    tv "tv" x=120
    character "dad" body="human-adult" on="chair.seat" { target hand-l="kb.keys:30%" hand-r="kb.keys:70%"; machine initial="typing" }
    character "kid" body="human-child" posture="sit-edge" on="tv.top" { look at="window" }
    character "cat" body="cat" posture="rear-up" { target paws="tv.side-r" }
}
```

**Stresses:** warm twin tinting inside a cone on a cold scene, emissive screens and windows, ambient darkness, many static objects (static caching), sockets and IK targets, small characters (20–40 px) with clip layering.

**Specific checks:**
- The lamp cone switches every lit surface to its `-warm` twin; no pixel outside the cone does.
- Swapping to `dusk` or `rainy-gray` needs 0 scene edits and still passes §1.

**Hand-written pieces:** `round-glasses`, a bag logo.

---

## B2 — Forest at dusk

**Description.** A side-scrolling forest at sunset: layered hills fading into fog, pines and oaks swaying, tall grass with gust waves, fireflies, a deer grazing, birds crossing the sky, a small stream in the foreground. The camera pans and the scene wraps so the pan loops.

| | |
|---|---|
| **Palette / style** | `dusk` / `moody` |
| **Packs** | `core-kit`, `nature`, `creatures` |
| **Body plans** | `deer`, `songbird` (in a `flock`) |
| **Authoring budget** | ≤ 40 lines, 0 pieces |

```kdl
scene "forest-dusk" view="side" palette="dusk" duration="16s" loop=#true seamless-x=#true world-width=512 {
    sky kind="dusk"; celestial kind="sun" y=150 glow=0.6
    hills layers=3 fog=0.6
    layer mid parallax=0.5 {
        repeat n=7 layout="row" { tree preset="pine" height="60..90" wind=0.3 }
    }
    layer near parallax=1 {
        ground surface="grass" edge="tufts"
        tree preset="oak" x=40; tree preset="oak" x=200
        grass density=0.8 wind=0.4 flowers=0.1
        stream y=230 width=14 rocks=0.3
        character "deer" body="deer" machine="graze"
    }
    flock count=6 member="songbird" bounds="sky"
    fireflies count=25 area="near" pulse="2s..4s"
    camera { k "0s" x=0; k "16s" x=512 interp="linear" }
}
export "forest-layers" kind="layers" source="forest-dusk" seamless-x=#true
```

**Stresses:** L-system vegetation, terrain layers with fog, a shared wind field, emissive particles with point lights, `quadruped` gait, `flock` with checkpoints, world wrap (`seamless-x`: each layer is generated on a cylinder of `world-width × parallax` pixels), camera pan with world-anchored dither, parallax layer export.

**Specific checks:**
- Wrap seam: the column at x = 0 and x = `world-width` match on every layer.
- Dither stability: during the pan, dither patterns move exactly with their layer (no "swimming").
- The exported layers reassemble into the render pixel for pixel.

**Hand-written pieces:** none.

---

## B3 — Rainy neon street

**Description.** A night street in one-point perspective: buildings converge to a vanishing point, neon signs flicker (one with a broken letter), wet asphalt and puddles reflect the neon, rain falls in depth layers, pedestrians with umbrellas walk toward and away from the camera, a car passes with headlights, steam rises from a vent.

| | |
|---|---|
| **Palette / style** | `neon-night` / `moody` |
| **Packs** | `core-kit`, `urban`, `creatures` |
| **Body plans** | `human-adult` (seeded variations), `car` |
| **Authoring budget** | ≤ 50 lines, ≤ 1 piece |

```kdl
scene "neon-street" view="perspective" vanishing-point="128,110" palette="neon-night" duration="10s" loop=#true {
    sky kind="night" stars=0
    street lanes=2 wet=0.8 markings="dashed"
    repeat n=4 side="left"  { building preset="shop-row" lit-windows=0.4 }
    repeat n=4 side="right" { building preset="apartment" lit-windows=0.3 }
    neon-sign content="NOODLES" color="neon-pink" on="building-l-2.sign" broken-letters="O" flicker=0.2
    neon-sign content="24H" color="neon-cyan" on="building-r-1.sign"
    scatter n=6 area="street" { puddle }
    rain layers=3 density=0.7
    steam-vent on="sidewalk-r" at="40%"
    repeat n=5 { character body="human-adult" path="sidewalk-l" gait="walk" { piece "umbrella" anchor="hand-r" } }
    character body="car" path="lane-1" clip="drive" timing="3s..5s"
}
```

**Stresses:** perspective helpers (converging facades, depth sorting), `facade-gen` variety, neon text as emissive and as colored light, puddle reflections, rain splashes driving ripples, several colored lights tinting the same surfaces, characters **regenerated** at each depth (a far pedestrian is rebuilt at 14 px, never shrunk), a `machine` rig with light emitters.

**Specific checks:**
- Every pedestrian height matches the perspective scale for its depth ±1 px, and each one passes the orphan and line checks at its own size.
- Each neon color appears (as its twin) on walls within its light radius and in at least one puddle reflection.
- Walkers pass the foot-planting check.

**Hand-written pieces:** `umbrella`.

---

## B4 — Game hero + effects

**Description.** A game-ready playable character, a small adventurer with a sword and a cape, plus the effects the game needs around it. Deliverables: a side-view platformer set, a top-down set in 8 directions, two palette variants, and an effect sheet.

| | |
|---|---|
| **Palette / style** | `morning` + `palette-override` variants `team-red`, `team-blue` (project palettes) / `moody` |
| **Packs** | `core-kit`, `creatures` |
| **Body plans** | `hero` (a `biped` instance) |
| **Authoring budget** | ≤ 40 lines, ≤ 2 pieces |

```kdl
sprite-set "hero" body="hero" dirs=8 {
    clip idle frames=4 hold="on3"
    clip walk frames=8
    clip run frames=8
    clip jump frames=6
    clip fall frames=2
    clip attack frames=6 { vfx "slash-trail" socket="weapon" arc=140 }
    clip hurt frames=3
    clip death frames=8
}
export "hero-side" target="hero" views="side" {
    sheet layout="rows-by-clip" pivot="bottom-center"
    meta formats="aseprite,godot,unity"
    variants palette-override="team-red team-blue"
}
export "hero-topdown" target="hero" views="top-down" { sheet layout="rows-by-direction" pivot="bottom-center" }
export "hero-fx" kind="effect-sheet" {
    vfx "slash-trail"; vfx "impact" sparks=0.6; vfx "dust-puff"
}
```

**Stresses:** `model` path for 8-direction rendering, gait generator at game speeds, rope cloth (cape) responding to motion, held pieces with rotation buckets in every direction, clip channels (attacking while running), palette variants, effect prefabs synced to clip sockets, sprite and effect sheet export.

**Specific checks:**
- Pivot stability = 0 px across every clip, direction and variant.
- Palette variants differ **only** in color: identical alpha masks per frame.
- The exported sheets import into Godot and Aseprite with named animations using their metadata only (automated import test).
- Silhouette distinctness: the 8 directions' silhouettes are pairwise different (except mirrored pairs).

**Human rubric extra:** the same character is recognizable in every direction at 1× (28 px).

**Hand-written pieces:** `sword`, optionally a face detail.

---

## B5 — Underwater coast

**Description.** A split view of a coastline: above the waterline, a bright sky, gulls and the shore with lapping foam. Below it, a blue-green world with kelp swaying, light rays and caustics on the sand, a school of fish turning together, a crab walking sideways, and bubbles rising from rocks.

| | |
|---|---|
| **Palette / style** | `deep-sea` / `moody` |
| **Packs** | `core-kit`, `nature`, `water`, `creatures` |
| **Body plans** | `fish` (in a `flock`), `crab`, `gull` |
| **Authoring budget** | ≤ 40 lines, 0 pieces |

```kdl
scene "coast" view="side" palette="deep-sea" duration="12s" loop=#true warmup="4s" {
    sky kind="day"; clouds layers=2 coverage=0.3 drift=0.1
    flock count=3 member="gull" bounds="sky"
    shore x=180 slope=0.4 wave-cycle="4s"
    ocean horizon=90 swell=0.3 split-view=#true sun-glint=0.5
    layer under {
        seabed surface="sand" ripples=0.5
        caustics scale=12 speed=0.3
        light-rays count=4 angle=-10
        scatter n=9 area="seabed" { kelp height="40..80" sway=0.5 }
        scatter n=4 area="seabed" { coral kind="branch" }
        rock-gen x=60 size=20 { bubbles source="top" rate=1.5 }
        flock count=14 member="fish" bounds="under" loop-mode="orbit"
        character body="crab" path="seabed" gait="wave"
        underwater-fog density=0.4
    }
}
```

**Stresses:** split-view water surface (refraction offset, depth tint), caustics as an animated light mask, dithered light rays, rope (kelp) driven by a current field, `flock` looping (`loop-mode="orbit"`), `multi-leg` gait, two light environments in one frame, emitters on sockets.

**Specific checks:**
- Underwater softening uses only dither and tone shifts: the blur-filter count reported by the renderer is 0.
- The school passes the loop seam check (orbit mode).
- The crab passes foot planting.

**Hand-written pieces:** none.

---

## B6 — Isometric village

**Description.** A small isometric island village: grass, dirt paths and water tiles with autotiled edges, houses with smoking chimneys, trees, a well, villagers walking the paths, chickens pecking, water shimmering. Delivered as a reusable tileset and tilemap, plus an animated scene.

| | |
|---|---|
| **Palette / style** | `morning` / `moody` |
| **Packs** | `core-kit`, `nature`, `urban`, `creatures` |
| **Body plans** | `human-adult` (model path, 4 iso directions), `chicken` |
| **Authoring budget** | ≤ 40 lines, 0 pieces |

```kdl
tileset "village-ground" size=16 view="isometric" palette="morning" {
    terrain grass { fill "=fbm(x*0.2,y*0.2)" material="grass" }
    terrain dirt  { fill noise=0.3 material="soil" }
    terrain sand  { fill noise=0.2 material="sand" }
    terrain water { water-surface animated=#true frames=4 }
    autotile grass over="dirt"  rules="blob-47"
    autotile sand  over="water" rules="blob-47"
    autotile grass over="sand"  rules="wang-16"
}
scene "village" view="isometric" palette="morning" duration="8s" loop=#true {
    terrain-map "island" size="24x24" levels="water sand grass" seed=7
    tilemap tileset="village-ground" source="island"
    path-tiles from="house-1.door" to="well" terrain="dirt"
    scatter n=4 area="grass" { building preset="cottage" roof="gable" { emitter preset="smoke" on="roof.chimney" } }
    scatter n=12 area="grass" { tree preset="oak" billboard=#true }
    well x=12 y=12
    repeat n=3 { character body="human-adult" path="dirt" gait="walk" }
    scatter n=5 area="grass" { character body="chicken" machine="peck-wander" }
}
export "village-tiles" kind="tileset" source="village-ground" meta formats="tiled,godot"
```

**Stresses:** isometric depth sorting, `model` path buildings, billboards for `shape` trees, autotile generation with terrain transitions, animated tiles, `terrain-map` noise, path following on a tile mask, characters in 4 iso directions, Tiled/Godot export.

**Specific checks:**
- All 47 blob cases and 16 Wang cases exist, and every edge pixel matches its neighbor in every case (automated seam test).
- Depth order: no pixel of an object farther back is drawn over a nearer one (checked from the depth buffer).
- Exported files open in Tiled and Godot and re-render identically (automated import test).
- 5 seeds give 5 valid, connected islands.

**Hand-written pieces:** none.

---

## B7 — Character close-up

**Description.** A dialogue close-up: a shopkeeper (bust, head about 48 px tall) behind a counter talks to the viewer, raises an eyebrow, looks surprised, then laughs. Hair and earrings sway, steam rises from a cup, and the shop behind is pushed back with dithered fog. This benchmark targets what the engine is worst at: large characters and faces.

| | |
|---|---|
| **Palette / style** | `morning` / `moody` |
| **Packs** | `core-kit`, `interior`, `creatures` |
| **Body plans** | `human-adult` with `framing="bust"` and the face rig ([CATALOG.md](CATALOG.md) §7.2.1) |
| **Authoring budget** | ≤ 50 lines, ≤ 2 pieces |

```kdl
scene "shopkeeper" view="side" palette="morning" duration="8s" loop=#true {
    layer back fog=0.5 { shelf levels=3 books="sparse" clutter=0.6; window x=200 { sky kind="day" } }
    counter y=200 { mug on="counter.top" at="80%" steam=0.5 }
    character "keeper" body="human-adult" framing="bust" facing="front" height=150 {
        hair style="long" volume=0.7 sway=0.4
        face eyes="almond" brows=0.6 nose="button" freckles=0.3
        piece "hoop-earring" anchor="ear-l"; piece "hoop-earring" anchor="ear-r"
        timeline {
            cue "talk" at="0s" line="Welcome back! Rainy day, huh?"
            cue "expression" at="3s" value="thinking"
            cue "expression" at="4.2s" value="surprised"
            cue "expression" at="5.5s" value="happy"
            cue "clip" at="5.5s" value="laugh"
        }
    }
}
```

**Stresses:** face rig (parametric features, expression blending, mouth shapes from text), large-form shading and cleanup at a size where "procedural tells" are most visible, `hd` piece variants, rope secondary motion (hair, earrings), face/head/body channel layering, dithered depth fog.

**Specific checks:**
- Mouth shape timing matches the `line` letter-to-shape rules within ±1 frame.
- Expression transitions pass through no frame with orphan pixels or broken outlines.
- Cleanup passes on the face report 0 orphan pixels and 0 L-corners at 48 px head size.

**Human rubric extra:** a blind test where 5 viewers name the expression of 6 stills. Pass if at least 4 out of 6 are named correctly by most viewers.

**Hand-written pieces:** `hoop-earring` (with an `hd` variant).

---

## B8 — Short story

**Description.** A 30-second, three-shot story. **(1)** Title card "RAINY DAY" over the street at dusk. A woman walks a dog on a leash toward a door while the light shifts from dusk to night. **(2)** Close-up as she smiles. **(3)** Inside the room: she sits down, drops the leash, throws a ball, and the dog fetches it and brings it back. The same woman and dog appear in every shot at different sizes and views.

| | |
|---|---|
| **Palette / style** | `dusk` → `neon-night` (palette-swap over time), `night-blue` / `moody` |
| **Packs** | `core-kit`, `interior`, `urban`, `creatures` |
| **Body plans** | `human-adult` (cast prefab `mara`), `dog` (cast prefab `pip`) |
| **Authoring budget** | ≤ 120 lines including the cast file, ≤ 1 piece |

```kdl
// cast.kdl — characters defined once, reused in every shot
prefab "mara" { character body="human-adult" { hair style="bun"; outfit top="coat" bottom="pants"; face eyes="round" } }
prefab "pip"  { character body="dog" { covering pattern="two-tone" } }

sequence "rainy-day" fps=12 {
    shot "street" duration="10s" view="perspective" palette="dusk" {
        anim palette { k "2s" "dusk"; k "8s" "neon-night" interp="palette-swap" }
        text "title" content="RAINY DAY" font="title-9x11" y=40 { anim opacity { k "0s" 1; k "2s" 1; k "3s" 0 interp="dither" } }
        mara "mara" path="sidewalk-r" gait="walk"
        pip "pip" path="sidewalk-r" follow="mara" offset=-14 gait="trot"
        rope "leash" from="mara.hand-l" to="pip.collar" sag=0.3
        camera follow="mara" lead=20
    }
    transition "dither" duration="1s"
    shot "smile" duration="4s" view="side" { mara framing="bust" { timeline { cue "expression" at="1s" value="happy" } } }
    transition "cut"
    shot "room" duration="14s" view="side" palette="night-blue" {
        use-scene "cozy-room" without="dad kid cat"
        mara "mara" on="chair.seat"
        pip "pip" x=150
        ball "ball" on="mara.hand-r"
        script "fetch.ts"            // throw → pip runs to ball → picks it up (on="pip.mouth") → returns
    }
}
```

**Stresses:** the director (shots, cuts, dither transition), palette-swap over time, title text from a pixel font, **character continuity** (one cast prefab regenerated at 38 px full body, 150 px bust, and seated), camera follow, **two rigs interacting** (leash rope between two moving sockets, a ball handed between sockets via `on=`), event scripts, reusing a whole scene inside a shot.

**Specific checks:**
- Continuity: `mara` and `pip` use identical palette roles, outfit and piece choices in every shot (checked from the resolved scene graph).
- The leash endpoints stay attached to their sockets (0 px gap) in every frame.
- The ball is always attached to exactly one socket or in free flight; it never teleports (max per-frame jump ≤ its speed).
- The palette swap never shows colors outside the two palettes.
- The MP4 export plays at the declared fps with crisp pixels (nearest-neighbor upscale check).

**Hand-written pieces:** none required (optionally a shop sign logo).

---

## B9 — Style switch

**Description.** B2 (forest) and B1 (room) are rendered under every shipped style preset with **zero scene edits**: only the style changes (`pxe export … --style <name>`).

| Preset | Resolution | Expectation |
|---|---|---|
| `moody` | 256×256 | Reference |
| `cartoon` | 256×256 | Flat 3-tone, full outlines, no dither |
| `one-bit` | 256×256 | 2 colors, shading by dither only |
| `handheld` | 256×256 | 4 colors with `mono-4` |
| `crisp-hd` | 480×270 | Content regenerated at `detail-scale` 1.875; more leaves, finer brick, no resampled pixels |
| `chunky` | 256×256 (pixel-scale 2) | Bold, simplified shapes |

**Stresses:** style presets ([RENDERING.md](RENDERING.md) §16), `color-limit` enforcement, `detail-scale` regeneration, prefabs that degrade gracefully at 2–3 tones, outline modes, piece `hd` variants.

**Specific checks:**
- Unique color count ≤ the style's `color-limit` in every frame.
- Layout invariance: each object's bounding box matches the `moody` render × `detail-scale` within 1 px.
- `crisp-hd` contains no 2×2 blocks duplicated from a 1× render (resampling detector).
- All loops still pass the seam check in every style.
- Authoring budget: **0** scene lines changed.

**Human rubric extra:** each output must look like an intentional work in its style, not a filtered version of `moody`.

---

## 12. Hold-out scenes

Known benchmarks get optimized for. Hold-outs test whether the engine is general **for scenes nobody planned**.

**Process**
1. At the end of every milestone, draw **2 prompts** at random from the pool below (or have someone else write new ones). The person authoring the scenes must not have tuned the engine for them.
2. Author each scene using **only the docs and existing packs**, within **2 hours** per scene. New KDL prefabs, pieces and palettes in a project pack are allowed. **Engine code changes are not.**
3. Record the result: passes §1 checks? Rubric score? What was missing?
4. Every missing capability becomes a **generality gap** issue. It is fixed generically and the scene joins a regression set (not the benchmark list).
5. Track the **hold-out pass rate** per milestone. It should rise over time. If it doesn't, the engine is being shaped around B1–B9.

**Prompt pool (seed list; extend freely)**

| Prompt | Likely stresses |
|---|---|
| Desert market at noon, top-down, crowds | Top-down view, harsh light, crowds |
| Spaceship corridor with flickering lights and a robot | Machine rigs, emissive panels, perspective |
| Snowy mountain village at night | Snow, warm windows, terrain |
| Haunted library with floating books and a ghost | Particles on paths, transparency via dither |
| Rooftop garden at sunrise with laundry in the wind | Cloth ropes, wind, dawn palette |
| Dragon flying over clouds | Serpent + bird composite rig, parallax sky |
| Train passing through countryside | Long `machine`, fast parallax, seamless-x |
| Cave platformer tileset with torches | Side-view autotiles, animated tiles, fire |
| 96 px boss spider sprite sheet | Large `multi-leg`, effects, sheets |
| Vending machine close-up with buttons and coins | Close-up props, text, small interactions |

---

## 13. Feature coverage matrix

● = the benchmark **stresses** the feature (it has a specific check). ○ = it **uses** the feature lightly.

| Feature | B1 | B2 | B3 | B4 | B5 | B6 | B7 | B8 | B9 |
|---|---|---|---|---|---|---|---|---|---|
| Palette, ramps, roles | ● | ● | ● | ● | ● | ● | ● | ● | ● |
| Twin tinting by colored light | ● | ○ | ● |  |  |  | ○ | ○ | ○ |
| Emissive ramps | ● | ● | ● | ○ | ○ |  |  | ○ | ○ |
| Ambient darkness | ● | ○ | ● |  |  |  |  | ● | ○ |
| Cleanup passes | ● | ● | ● | ● | ● | ● | ● | ● | ● |
| Style presets |  |  |  |  |  |  |  |  | ● |
| `detail-scale` / regenerate at size |  |  | ● |  |  |  | ○ | ● | ● |
| `shape` path (2D/SDF) | ● | ● | ○ | ○ | ● | ○ | ● | ● | ● |
| `model` path (3D → pixel) |  |  | ○ | ● |  | ● |  |  |  |
| L-systems / `tree-gen` |  | ● |  |  | ○ | ○ |  |  | ○ |
| Terrain layers / parallax |  | ● |  |  | ○ |  | ○ | ○ | ○ |
| World wrap (`seamless-x`) |  | ● |  |  |  |  |  |  |  |
| Water surface (reflection, refraction) |  | ○ | ● |  | ● | ○ |  |  |  |
| Particles / emitters | ○ | ● | ● | ○ | ● | ○ | ○ | ○ | ○ |
| Effects (`vfx`) |  |  |  | ● |  |  |  |  |  |
| Rope / chain | ○ |  |  | ● | ● |  | ● | ● |  |
| `flock` |  | ● |  |  | ● |  |  |  |  |
| Wind / current field |  | ● | ○ |  | ● |  |  |  | ○ |
| `facade-gen` |  |  | ● |  |  | ○ |  | ○ |  |
| Text / pixel fonts |  |  | ● |  |  |  |  | ● |  |
| Tiles / autotile / tilemap |  |  |  |  |  | ● |  |  |  |
| `biped` | ● |  | ● | ● |  | ○ | ● | ● | ○ |
| `quadruped` | ○ | ● |  |  |  |  |  | ● |  |
| `bird` |  | ○ |  |  | ○ | ○ |  |  |  |
| `fish` |  |  |  |  | ● |  |  |  |  |
| `multi-leg` |  |  |  |  | ● |  |  |  |  |
| `machine` |  |  | ● |  |  |  |  |  |  |
| Face rig (close-ups) |  |  |  |  |  |  | ● | ● |  |
| Gait generator |  | ● | ● | ● | ○ | ○ |  | ● |  |
| IK targets / sockets | ● |  | ○ | ● |  |  | ○ | ● |  |
| Two-rig interaction |  |  |  |  |  |  |  | ● |  |
| State machines | ● | ○ |  | ● |  | ○ | ○ | ● |  |
| Event scripts |  |  |  | ○ |  |  |  | ● |  |
| Pieces (incl. `hd`) | ○ |  | ○ | ● |  |  | ● |  | ○ |
| Keyframes / modulators | ● | ● | ● | ● | ● | ● | ● | ● | ● |
| Camera pan / follow, anchored dither |  | ● |  |  |  |  |  | ● |  |
| Director: shots, transitions |  |  |  |  |  |  |  | ● |  |
| Palette swap over time / overrides | ○ |  |  | ● |  |  |  | ● | ○ |
| View: `side` | ● | ● |  | ● | ● |  | ● | ● | ● |
| View: `perspective` |  |  | ● |  |  |  |  | ● |  |
| View: `top-down` |  |  |  | ● |  |  |  |  |  |
| View: `isometric` |  |  |  |  |  | ● |  |  |  |
| Loop solver | ● | ● | ● |  | ● | ● | ● |  | ● |
| Sprite / effect sheet export |  |  |  | ● |  |  |  |  |  |
| Tileset / tilemap export |  |  |  |  |  | ● |  |  |  |
| Parallax layer export |  | ● |  |  |  |  |  |  |  |
| Sequence / MP4 export |  |  |  |  |  |  |  | ● |  |

**Features stressed by only one benchmark:** style presets, world wrap, effects, tiles, `fish`, `multi-leg`, `machine`, two-rig interaction, director, `top-down`, `isometric`, sheet/tileset/layer/sequence export. Each is a generic capability documented with uses beyond its benchmark, and most appear in the hold-out pool (§12), which is where their generality is tested.

---

## 14. Process

1. **Each benchmark is a scene file in the repo** (`benchmarks/b1-cozy-room.kdl` …) with `golden` frames, rendered by CI.
2. **Automatic checks** (§1) run on every change. Golden-image diffs on indexed buffers must be reviewed and accepted explicitly (`pxe bench --update`).
3. **Seed sweep:** every benchmark renders with 5 random seeds; the automatic checks must pass for all of them.
4. **Style sweep:** B1 and B2 render under every style preset (that is B9). Other benchmarks run the sweep periodically as a warning, not a gate.
5. **Human rubric** (§1.1) at every milestone that unlocks or changes a benchmark.
6. **Hold-outs** (§12) at the end of every milestone.
7. **Milestones:** [ROADMAP.md](ROADMAP.md) marks which benchmark each milestone must pass.

## 15. Known risks

| Benchmark | Risk | Mitigation |
|---|---|---|
| B4 | 8-direction readability at 28 px through the `model` path | Silhouette distinctness check; per-direction `hd`/piece overrides allowed |
| B7 | Faces look generic or "procedural" at 48 px | Face rig parameters per character, expression blind test, `hd` pieces for signature details |
| B8 | Interacting rigs drift apart (leash gaps, ball teleports) | Socket constraints solved after IK, continuity checks every frame |
| B9 | Prefabs designed for 5 tones collapse at 2 tones | Every prefab must render under every shipped style; B9 is a CI gate |
| B2 | Wrapping parallax layers at non-integer widths | `world-width × parallax` must be an integer; the validator enforces it |
