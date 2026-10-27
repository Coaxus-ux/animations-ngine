# Rendering Pipeline (draft v1)

> Companion to [SPEC.md](SPEC.md). Covers how one frame becomes pixels, for every view and every kind of scene. Palette and ramp rules live in [PALETTES.md](PALETTES.md); crate layout in [ARCHITECTURE.md](ARCHITECTURE.md); time and rigs in [ANIMATION.md](ANIMATION.md).

Examples refer to the benchmarks in [BENCHMARKS.md](BENCHMARKS.md): **B1** cozy night room, **B2** forest at dusk, **B3** rainy neon street, **B4** game hero sprite sheet, **B5** underwater coast, **B6** isometric village tiles.

---

## 1. Principles

| Rule | Consequence |
|---|---|
| Everything is indexed until the very end | Light, fog, glow, reflections and transitions change **ramp + tone**, never blend RGB |
| One pixel = one texel, always | No supersampled color, no anti-aliasing, no non-integer scaling |
| **Regenerate, never scale** | A character farther away in B3 is *rebuilt* at 14 px tall, not shrunk from 38 px |
| Every stage is a pure function of (scene, seed, frame) | Except stateful nodes, which use checkpoints (see ARCHITECTURE §3.5) |
| The same buffers for 2D and 3D | Props and characters from either path go through identical lighting, cleanup and resolve |

---

## 2. Buffers

Each **canvas** (one per layer, plus the scene canvas) holds these parallel buffers:

| Buffer | Content | Notes |
|---|---|---|
| `material` | Ramp id (role-resolved) | `0` = empty |
| `tone` | Tone index on that ramp | Before lighting this is the *form tone* |
| `depth` | Depth value | Larger = farther. 2D objects get depth from layer + `z` + y-sort |
| `normal` | Quantized normal (3D path) or pseudo-normal (2D SDF) | Drives form shading, sel-out and lighting facing |
| `object` | Object id | For outlines, selection, per-object cleanup and effects masks |
| `flags` | `emissive`, `casts-shadow`, `receives-light`, `reflective`, `no-outline` | One bit each |

Scene-level extra buffers:

| Buffer | Content |
|---|---|
| `light-steps` | Signed tone steps added by lighting |
| `light-color` | The dominant light color id per pixel (for twins) |
| `index` | Final palette color index (after resolve) |

There is no alpha channel. Partial transparency is **dither opacity**: pixels are written or skipped by a dither threshold (§9).

---

## 3. Frame pipeline

```
 1  Evaluate graph at frame f      values, poses, particles, camera
 2  Rasterize each layer           2D path (§4) or 3D path (§5) → layer canvases
 3  Per-object cleanup             §7 (all rules except sel-out)
 4  Composite layers               parallax offsets, depth test, y-sort (§6)
 5  Lighting                       ambient, lights, shadows, twins (§8)
 6  Reflections & refraction       puddles, water, glossy floors (§8.7)
 7  Effects                        fog, light rays, caustics, glow (§10)
 8  Outlines                       full / sel-out, needs lighting result (§7.6)
 9  Overlay particles              foreground rain, snow, bubbles
10  Resolve                        (ramp, tone) → palette index → RGB (§13)
11  Palette transitions            dither swaps, fades (index space)
12  Upscale & encode               nearest-neighbor integer, GIF/PNG/MP4/sheets
```

Steps 2–4 for **static** layers are cached and skipped on later frames (ARCHITECTURE §3.3). In B1, the room, shelves and books are drawn once, so only the rain, lamp flicker and characters are redrawn.

---

## 4. 2D rasterization

### 4.1 Primitives

| Primitive | Algorithm | Rules |
|---|---|---|
| `rect` | Direct fill | Integer corners |
| `polygon` | Scanline | Top-left fill rule (shared edges are never drawn twice) |
| `ellipse` | Midpoint ellipse | Symmetric by construction; even and odd diameters both supported |
| `line` | Bresenham + pixel-perfect pass (§7.2) | Width 1–3; wider lines become polygons |
| `path` | Polyline/Bézier → sampled → Bresenham segments | Used for cables, tails, rope, L-system branches |

Samples are always taken at **pixel centers**, so coverage is binary.

### 4.2 2D SDF shapes
Shapes are combined as signed distance functions: `circle`, `capsule`, `rbox`, `triangle`, `segment`, `arc`. The operators are `union`, `subtract`, `intersect`, `smooth-union k` and `round r`.

- **Fill:** a pixel is inside when `d ≤ bias`. `bias` (default 0) fattens or thins shapes by sub-pixel amounts without blurring.
- **Stroke:** a pixel is drawn when `|d| ≤ width/2`.
- **Pseudo-normal:** the SDF gradient plus a height profile (`flat`, `dome`, `cylinder`, `bevel`) turns the 2D shape into a lit volume. B2 tree crowns use `dome`; B1 shelves use `bevel`.

### 4.3 Form shading (2D)
`form tone = base + round(k · dot(n, key-light))`, clamped to the ramp.

- `key-light` is the scene's key light direction (default upper-left), independent of real light sources.
- `shade=flat` skips this step. `shade=soft` uses hard bands. `shade=dither` dithers band edges (§9), but only where a band edge spans at least 8 px; on smaller forms it falls back to hard bands, because dither there reads as noise ([spike](spikes/cleanup.md)).
- **Anti-pillow rule:** form shading follows the light direction, never the distance to the silhouette. Uniform shading toward every edge (pillow shading) is forbidden.
- **Normals for 2D shapes** come from a dome height field, `h = sqrt(clamp(-sdf / R, 0, 1))`, so forms read as rounded volumes lit from one side ([spike](spikes/cleanup.md)).

### 4.4 Textures and patterns
Patterns (planks, bricks, scales, bark, fur, noise) are sampled in **object space**, so they move with the object and never "swim". They write `material` and/or `tone` offsets. Noise-based textures always go through noise clustering (§7.4).

### 4.5 Bitmaps (pieces)
Pieces are pasted at integer positions with integer transforms only (flip, rotate 90°). Their slots resolve to the owner's ramps. See [PIECES.md](PIECES.md).

---

## 5. The 3D-to-pixel path

Used by **`model` prefabs** (props) and by **body plans on path B** (characters). It is the only way to get top-down, isometric and true perspective rotations, and the way to get 4/8-direction sprite sheets (B4).

### 5.1 Geometry sources

| Source | Use | Backend |
|---|---|---|
| Primitives | `box`, `cylinder`, `sphere`, `capsule`, `cone`, `torus`, `wedge`, `plane` | Triangle rasterizer |
| Builders | `extrude` (2D shape → solid), `lathe` (profile → round solid), `loft` along a path | Triangle rasterizer |
| Skinned rigs | Body plan volumes bound to bones | Triangle rasterizer |
| 3D SDF | Blended organic shapes (smooth unions), terrain, rocks, coral | Ray marcher |

Both backends write the **same buffers** (§2) at native resolution.

### 5.2 Cameras

| Camera | Projection | Used by |
|---|---|---|
| `side` | Orthographic, looking along −Z | Matching 2D side scenes (B1, B2, B5); side-view sprites |
| `top-down` | Orthographic, pitch 30–90° (default 60°) | RPG maps, top-down sprites |
| `isometric` | Orthographic dimetric, **2:1 pixel ratio** (26.565°), yaw 45° | B6 tiles and buildings |
| `perspective` | Pinhole, integer focal setup, horizon and vanishing points snapped to pixels | B3 street |

### 5.3 Rasterizer rules
- **Native resolution:** 1 texel = 1 pixel.
- **Optional 2× coverage vote:** render at 2× and downsample by **majority object/material** (never by averaging color). This gives cleaner silhouettes for thin parts like swords, tails and antennas.
- **Snapping:** the camera moves in whole pixels, and every object's origin is snapped to the camera's texel grid in view space. This removes pixel crawl when things translate.
- **Quantized rotation:** model yaw is rendered at fixed steps (`directions=4|8|16`) for characters. Props declare their allowed angles. This prevents shimmer from arbitrary rotations.

### 5.4 Toon banding to ramps
- `form tone = base + band(dot(n, key-light))`. The band thresholds are the ramp's tone count; `shade=dither` dithers between bands.
- Optional **contact darkening:** −1 tone where depth shows nearby occluders (cheap ambient occlusion, sampled on a few depth neighbors).

### 5.5 Outlines from buffers

| Line | Detected where |
|---|---|
| Silhouette | `object` differs from the neighbor's, or the neighbor is empty |
| Depth crease | Depth jump above `crease-depth` between neighbors of the same object |
| Normal crease | Normal angle above `crease-angle` (default 50°) |

The detected lines feed the same outline stage as 2D (§7.6), so sel-out works for 3D props too.

### 5.6 Characters on the 3D path
Body plan volumes are skinned to the skeleton. Posing comes from [ANIMATION.md](ANIMATION.md). Each direction renders with the same camera, so a B4 sheet is just *pose × direction × frame*. Pieces (eyes, buckles, emblems) are pasted **after** rasterization at projected anchor positions, choosing the piece variant by direction (`front`, `side`, `back`, `three-quarter`).

---

## 6. Views, layers, depth and parallax

### 6.1 Views
`view` is a scene property. Prefabs declare the views they support.

| View | 2D prefabs | `model` prefabs | Body plans | Parallax | Typical use |
|---|---|---|---|---|---|
| `side` | ✅ native | ✅ via `side` camera | Path A (2D) or B | ✅ | B1, B2, B5, platformers |
| `perspective` | ✅ only perspective-aware prefabs (street, facade, room box, road) | ✅ | Path B, or path A regenerated at the depth's pixel height | ✅ (depth layers) | B3 |
| `top-down` | ⚠️ only prefabs with a top-down variant | ✅ | Path B | — | RPG maps |
| `isometric` | ❌ | ✅ | Path B | — | B6 |

Using a prefab in a view it doesn't support is an error, with a suggestion (`"lamp" has no isometric variant; try "lamp-model"`).

### 6.2 Perspective helpers (2D)
The scene defines a `horizon` and one or two `vanishing-point`s. Perspective-aware prefabs project their geometry toward them. Objects placed at depth `d` get a target pixel height from the projection and are **regenerated at that height** (procedural content makes this free). In B3, pedestrians far down the street are rebuilt at 12–18 px instead of being scaled.

### 6.3 Layers and depth
- Layers are composited back to front.
- Inside a layer, the depth buffer decides visibility. Ties are resolved by `z`, then by `y-sort` when enabled (base y; default on in `top-down` and `isometric`).
- Each layer has `affects-light`, `receives-light` and `casts-shadow` defaults that objects can override.

### 6.4 Parallax
`offset(layer) = round(camera × parallax)`, computed **per layer and rounded once**. All pixels of a layer always move together. Layers with `parallax ≠ 1` render to their own canvas, so they're cached independently.

### 6.5 Atmosphere
A layer's `atmosphere` (0..1) shifts its tones toward a target ramp (`sky` by default) by a number of steps. This gives automatic depth in B2's hills and B3's far buildings. It is a ramp operation, not a color blend.

---

## 7. Cleanup passes

These run **per object** before compositing (except sel-out, which runs after lighting). Each one can be configured or disabled per object (`cleanup { orphans #false }`). Order: noise clustering → orphans → notches → steps → pixel-perfect lines → contrast guard → outline.

### 7.1 Orphan removal
A pixel whose 8 neighbors contain **no pixel of the same material and object** is replaced by its neighbors' majority material and tone (or emptied, if empty space wins).

### 7.2 Pixel-perfect lines
For traced 1 px strokes as an ordered pixel sequence `p[0..n]`: remove `p[i]` when `p[i−1]` and `p[i+1]` are diagonal neighbors (`|dx| = 1` and `|dy| = 1`). This removes L-shaped corners and keeps diagonals at 1 px.

### 7.3 Regular steps (jaggies)
Silhouette edges are traced as run lengths of horizontal and vertical segments.
- Inside a curve segment, runs must be **monotonic** (non-increasing, then non-decreasing): 3-2-1-1-2-3 is fine; 1-3-1-3 is not.
- A run that breaks monotonicity by more than 1 is smoothed by moving single edge pixels.
- Intentionally sharp corners (where the shape's tangent changes by more than 60°) are preserved.

### 7.4 Noise clustering
After any noise-driven material or tone, **and after form shading**:
1. Apply a 3×3 **mode filter** (majority) `cluster-passes` times (default 2).
2. Flood-fill to find clusters and remove those smaller than `min-cluster` pixels (default 3), merging them into a neighbor.

The result reads as clumps (moss, bark, fur, gravel) instead of static.

### 7.5 Notch fill and contrast guard
- **Notch fill:** an empty pixel with 3 or more of its 4 orthogonal neighbors in the same object is filled.
- **Contrast guard:** two *different* objects that touch with the same material and tone (books on a shelf, B6 tiles of the same kind) get a separation: the later one shifts −1 tone on the shared edge, unless `no-outline` is set.

### 7.6 Outline
| Mode | Rule |
|---|---|
| `none` | No outline |
| `full` | Every object pixel 4-adjacent to non-object becomes `outline` ramp |
| `sel-out` | Same edge pixels, but colored by lighting: on the shadow side → `outline`; on the lit side → the object's own ramp at `.shade` (or no outline when lit by 2+ steps) |
| `inner` | Adds depth and normal creases (§5.5) as lines in the object's `.deep` tone |

The outline is drawn **inside** the silhouette by default (keeps sizes exact). `outline-side=outside` grows the shape by 1 px.

### 7.7 Pixel coherence
The engine refuses to scale rasterized results by non-integer factors. Bitmaps (pieces) only take integer transforms. A mismatch is a render error, not a silent resample.

---

## 8. Lighting on ramps

### 8.1 Inputs
- The pixel's form tone and normal.
- `ambient darkness A` (tone steps).
- A list of lights: `point`, `cone`, `rect`, `line` (neon tubes), `directional` (sun/moon), `area` (sky light through a window).

### 8.2 Per-light contribution
`steps_i = intensity_i × falloff_i(distance) × mask_i × facing_i × visibility_i`

| Term | Definition |
|---|---|
| `falloff` | `smooth` (smoothstep to `reach`) or `inverse` (inverse-square, clamped at 1 near the source) |
| `mask` | The cone's angle with a soft edge of `spread-soft` degrees; a rect/line uses distance to the shape |
| `facing` | `max(0, dot(n, dir-to-light))` mixed with `wrap` (0..1) so flat 2D surfaces still catch light |
| `visibility` | Shadow term (§8.5): 0 or 1, dithered at penumbra edges |

### 8.3 Quantization
`total = form-tone − A + Σ steps_i`
- The integer part is applied directly.
- The fractional part is compared to a world-anchored dither threshold (§9) to decide +1.

This is what produces dithered light edges in B1's lamp cone and B3's neon spill.

### 8.4 Colored light and twins
- Each pixel keeps the **dominant** colored light (largest contribution). Colors never mix.
- If the dominant light's steps reach its `tint-at`, the pixel's material switches to the ramp's **twin** for that light color, keeping the tone index (PALETTES §4.4).
- Where two colored lights overlap (B3: pink neon next to cyan neon), the boundary is dithered by comparing their contributions against the threshold.

### 8.5 Shadows

| Path | Method |
|---|---|
| 2D | Per light, march from each receiving pixel toward the light over an **occluder mask** (pixels with `casts-shadow`, at depth ≤ the receiver). Hits set `visibility = 0`. Penumbra width grows with distance and is dithered |
| 2D contact | Objects on `line` sockets get a −1 tone ellipse under their footprint |
| 3D | A low-resolution shadow map per directional or point light, with results snapped to the pixel grid |

### 8.6 Emissive pixels
Emissive pixels keep their form tone, ignore `A` and shadows, and can't be darkened. A light with `glow` adds a halo of its own color through the glow effect (§10.5).

### 8.7 Reflections and refraction

| Feature | Rule |
|---|---|
| Planar reflection | Materials flagged `reflective` (puddles, wet asphalt, polished floors, still water) sample the lit scene mirrored about a `reflection-y` line |
| Tone mapping | Reflected pixels keep their source ramp, shifted `reflect-darken` steps (default −1). **Emissive sources stay emissive**, which is how B3's neon signs appear in puddles at full color |
| Ripples | Each reflected row shifts horizontally by an integer from noise (`ripple`, animated). Rain drops spawn expanding ring masks |
| Masking | Only pixels inside the reflective object's shape receive reflections |
| Refraction (water) | Below a water surface (B5), the underwater scene samples with per-row integer offsets from the wave function, plus an atmosphere shift toward the `water` ramp by depth |

---

## 9. Dithering

### 9.1 Patterns
- Bayer 2×2, 4×4 (default) and 8×8.
- Ordered patterns: `checker`, `lines-h`, `lines-v`, `diagonal`, `cross-hatch`, `cluster` (clustered dot).
- Blue-noise tiles for organic surfaces only (foliage, water).
- White noise is never used except for explicit effects (TV static).

### 9.2 Uses
Light edges and fractional light steps, gradients (sky, fog), dither opacity (ghosts, glass, steam), palette transitions, shading band edges.

### 9.3 Anchoring

| Anchor | Threshold coordinate | Used for |
|---|---|---|
| `world` (default for lighting, fog, sky) | World position | Patterns stay put when the camera pans: no "swimming" |
| `layer` | Layer-space position | Parallax layers, so each layer's pattern moves with it |
| `object` | Object-local position | Moving sprites (B4 hero, B3 car), so the pattern travels with the body |

### 9.4 Temporal stability
- Thresholds never change over time. Only the values being dithered change.
- **Hysteresis:** an animated value must cross a threshold by `h` (default 0.1 step) before a pixel flips back. This prevents flickering pixels on slow animations.
- Values animated "on 2s / on 3s" only update every 2 or 3 frames, so dithers change less often.

---

## 10. Effects

All effects write ramp, tone or light steps, never RGB.

| Effect | How it works | Benchmarks |
|---|---|---|
| **Water surface** | A wave line from a sum of 2–4 sines (integer heights); crest pixels use `water.hi` as foam; dither opacity below the line | B5 |
| **Caustics** | Animated cellular noise, warped, thresholded to thin bright lines and applied as +1 light step on surfaces below water | B5 |
| **Light rays** | Angled bands from a source edge (surface, window, canopy gaps), drawn as +1 light step with dither opacity and slow noise flicker | B5, B2 |
| **Fog / mist** | Depth- or height-based shift toward the `fog` ramp (atmosphere) plus an optional dithered fog layer that drifts | B2, B3 |
| **Glow** | Dilate the emissive mask by `radius` (1–3 px), add +1 light step in the emissive color, with a dithered outer ring. No blur | B3 neon, B2 fireflies |
| **Weather** | Rain streaks (1×n lines), snow (1–2 px), splashes and ripple rings, bubbles (B5) | B1, B3, B5 |
| **Wind** | A global wind field (noise over time) that sways foliage, grass, cloth, hair and rope | B2 |
| **Screens** | Static, scanlines, scrolling code inside screen sockets | B1 |

---

## 11. Compositing order (summary)

| # | Stage | Space |
|---|---|---|
| 1 | Far layers → near layers (parallax-offset) | Layer → scene |
| 2 | Objects within a layer by depth, `z`, y-sort | Layer |
| 3 | Lighting (lights can be limited to layers with `affects`) | Scene |
| 4 | Reflections and refraction | Scene |
| 5 | Fog, light rays, caustics, glow | Scene |
| 6 | Outlines (sel-out) | Scene |
| 7 | Foreground particle overlays | Scene |
| 8 | Resolve to palette indices | Scene |
| 9 | Palette transitions | Index |
| 10 | Upscale, optional output-only post (CRT scanlines, pixel grid), encode | RGB |

---

## 12. Camera

- Position is always an **integer**. Smooth pans compute a fractional position and floor it per frame, optionally "on 2s".
- **Zoom** is integer only (×1, ×2, ×3…).
- **Shake** uses integer offsets from seeded noise.
- **Rotation:** none in 2D views. On the 3D path, only in quantized steps.
- **Aspect:** the output canvas is fixed per project. Shots with other aspects are letterboxed with the `ink` ramp.

---

## 13. Resolve and output

1. **Resolve:** role → local ramp → color at tone → palette index. A palette change only invalidates this stage, so palette edits are near-instant in the editor.
2. **Color space:** palette colors are sRGB. OKLab is used only at palette load (ramp generation, auto-twins, nearest color).
3. **Upscale:** nearest-neighbor by the integer `scale`.
4. **Encoding:**
   - **GIF:** global palette from the scene palette. Error if it exceeds 256 colors.
   - **PNG:** indexed when the palette fits.
   - **MP4/WebM:** encode at ≥4× scale; prefer `yuv444p`, because chroma subsampling smears single-pixel colors.
   - **Sprite sheets, tilesets, parallax layer sets:** see [DIRECTOR.md](DIRECTOR.md).

---

## 14. Determinism

| Rule | Why |
|---|---|
| Transcendental math uses a deterministic implementation, not the platform libm | `sin`, `pow` and `exp` can differ between OSes |
| RNG is counter-based: `hash(seed, object-id, stream, index)` | No hidden RNG state; parallel-safe |
| Maps that affect output are ordered (insertion-ordered) | Hash map iteration order never changes pixels |
| Parallel work writes disjoint regions or reduces in a fixed order | Thread count never changes results |
| Thresholds and dither patterns are constants | Same input → same pixels |

Determinism is enforced by tests (ARCHITECTURE §8).

---

## 15. Performance budget

Canvas sizes: **256×256 = 65,536 px**, **320×180 = 57,600 px**. Targets are per frame on one core of a modern laptop. Static layers are cached, so typical animated frames cost much less than the full total.

| Stage | Budget (full redraw) | Notes |
|---|---|---|
| Graph evaluation (values, poses, particles) | 1 ms | Thousands of particles |
| 2D raster + cleanup (animated objects) | 2 ms | |
| 3D triangle path | 0.5 ms per 48 px character | B4 sheets render hundreds of frames quickly |
| 3D SDF ray march | 8–15 ms full screen | Static terrain/rocks cached; animated SDF kept small |
| Lighting (8 lights) + 2D shadows | 3 ms | |
| Reflections + effects | 2 ms | |
| Resolve + upscale ×4 | 1 ms | |
| **Typical animated frame** | **≈ 5–10 ms** | B1, B2, B3 after caching |

**Targets:**
- **Preview:** ≥ 30 fps redraw for scenes like B1–B3.
- **Export:** ≥ 100 frames/s on 8 cores for stateless scenes.
- **Memory:** about 1 MB per canvas, so 10–20 layers stay well under 50 MB.

---

## 16. Style presets

A **style** is a bundle of rendering rules. It is independent of the **palette** (which sets the mood) and of the **content** (what is in the scene). The same scene can render as moody dithered pixel art, flat cartoon or 1-bit without edits. This is what B9 tests.

### 16.1 Style settings

| Setting | Values | Default (`moody`) | Affects |
|---|---|---|---|
| `outline` | `none` \| `sel-out` \| `full` \| `inner` | `sel-out` | §7.6 outline pass |
| `outline-weight` | 1 \| 2 | 1 | Outline thickness in pixels |
| `outline-ramp` | role | `outline` | Ramp used by `full` outlines |
| `shading` | `flat` \| `banded` \| `soft` | `soft` | Form shading (§4.3, §5.4): `flat` = base tone only, `banded` = hard tone bands, `soft` = bands with dithered edges where a band edge spans ≥ 8 px, hard bands on smaller forms |
| `tones` | 2..5 | 5 | Max tones used per ramp (others snap to the nearest used tone) |
| `dither` | `off` \| `bayer-2` \| `bayer-4` \| `pattern` | `bayer-4` | §9: gradients, light edges, fog |
| `dither-strength` | 0..1 | 0.6 | How wide dithered transition bands are |
| `light-mode` | `ramp` \| `emissive-only` \| `none` | `ramp` | `emissive-only` keeps glowing pixels but drops light falloff |
| `color-limit` | int | — | Max unique colors on screen; extra tones are merged (with a validator report) |
| `cleanup` | `strict` \| `normal` \| `loose` | `normal` | Thresholds of the cleanup passes (§7) |
| `pixel-scale` | 1 \| 2 \| 3 | 1 | Chunky pixels: render at 1/n resolution and upscale by n |
| `detail-scale` | number | 1 | **Regenerates** content at a different size (see §16.3) |

### 16.2 Shipped presets

| Preset | Look | Key settings |
|---|---|---|
| `moody` | Default: soft light, dithered gradients, selective outlines | as table above |
| `cartoon` | Flat colors, thick dark outlines, no dither | `outline=full`, `outline-weight=1`, `shading=banded`, `tones=3`, `dither=off` |
| `one-bit` | Two colors, all shading by dither | `tones=2`, `color-limit=2`, `dither=bayer-4`, `dither-strength=1`, `outline=none` |
| `handheld` | 4-color handheld console look | `color-limit=4`, `tones=4`, `dither=bayer-2`, palette `mono-4` recommended |
| `crisp-hd` | Higher resolution, finer detail, minimal dither | resolution 480×270, `detail-scale≈1.875`, `dither-strength=0.3` |
| `chunky` | Big pixels, bold shapes | `pixel-scale=2`, `shading=banded`, `tones=4` |

Custom presets are KDL `style` nodes ([FORMAT.md](FORMAT.md) §5.2) and can be shipped in packs.

### 16.3 Changing resolution without scaling pixels
Scenes are authored in logical pixels for their project resolution. When a style or export uses a different resolution, `detail-scale` multiplies every **size parameter** (positions, widths, heights, radii, body heights) and the content is **regenerated** at the new size. Pixels are never resampled. Generators use the extra pixels for more detail: more leaves, finer brick, bigger faces.

Limits:
- Pieces are bitmaps. At `detail-scale` ≥ 1.5, a piece uses its `hd` variant if it has one ([PIECES.md](PIECES.md)); otherwise it stays at 1× and the validator reports it.
- Text uses the font's size variants.

### 16.4 Rules
1. **A style never changes content or layout.** Same objects, same positions (scaled), same animation timing.
2. **Palettes and styles combine freely.** `one-bit` forces at most 2 colors, picking the darkest and lightest roles unless the palette declares a 2-color mapping.
3. **Prefabs may read style values** (for example, `tree` draws fewer leaf clusters at `tones=2`), but must render under every shipped preset.
