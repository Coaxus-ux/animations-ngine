# Spike: cleanup quality (M0)

> Time-boxed, throwaway code (a Python script outside the repo). Question: do shapes generated from math, passed through the cleanup rules, look hand-placed? ([ROADMAP.md](../ROADMAP.md) M0, [RENDERING.md](../RENDERING.md) §4.3, §7)

## Setup
- Four generic 2D SDF shapes in a 40×40 cell: a **blob** (smooth union of circles), a **limb** (capsule), a **leaf** (two-circle intersection), a **rock** (noisy radius).
- One 5-tone green ramp, one outline color, light from the upper left.
- **Top row (raw):** threshold the SDF, 4-connected boundary as outline, tone = quantized light.
- **Bottom row (cleaned):** orphan removal and notch fill → pixel-perfect outline (drop L-corner pixels) → selective outline (lit edges use a dark ramp tone) → banded tones → merge tiny tone clusters.
- No reference crops from existing artworks were used. The evaluation is against standard pixel art rules: clean silhouettes, single-pixel lines without doubled corners, directional light, no pillow shading, no noise.

## Iterations

| Version | Change | Image |
|---|---|---|
| v1 | Light from the SDF gradient blended with depth | ![v1](img/cleanup-v1.png) |
| v2 | Dome normals from a height field; dithered band edges | ![v2](img/cleanup-v2.png) |
| v3 | Hard bands (no dither) and tone clusters under 3 px merged | ![v3](img/cleanup-v3.png) ![v3 limb, raw vs cleaned, zoomed](img/cleanup-v3-limb.png) |

## Findings
1. **Silhouette and outline cleanup works.** Raw 4-connected outlines double up on diagonals (clearly visible on the limb). The pixel-perfect rule leaves single-pixel diagonal runs. Orphan removal and notch fill remove stray pixels on the rock.
2. **Selective outline helps a lot.** Using a dark ramp tone on lit edges softens the upper-left contour and makes forms read as lit volumes instead of stickers.
3. **The shading model matters more than cleanup.** v1's gradient-plus-depth light produced pillow-like blotches. Dome normals (v2+) give consistent directional light.
4. **Dither on small forms reads as noise.** In v2, dithered band edges across a 10 px wide limb look like static, a clear "procedural" tell. Hard bands with small clusters merged (v3) look closest to hand-made.
5. **Remaining tells:** band edges still follow the math contour exactly, so steps are slightly irregular. The "regular steps" pass (RENDERING §7.3) should apply to band edges as well as outlines; it was not part of this spike.

## Verdict
**Go**, with these changes (already applied to [RENDERING.md](../RENDERING.md)):
- Form normals for 2D shapes come from a dome height field (§4.3).
- Dithered shading applies only where a band edge spans ≥ 8 px; smaller forms use hard bands (§4.3, §16 `shading=soft`).
- Noise clustering (min cluster size 3) also runs on form-shading tones (§7.4).

To verify during M2 with real Rust passes: regular steps on band edges, and side-by-side review against hand-made references.
