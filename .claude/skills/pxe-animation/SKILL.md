---
name: pxe-animation
description: Create or change a pixel-art animation, scene, title card or sequence with this repo's engine (KDL files rendered by `pxe`). Use when asked to make, animate, illustrate or export something as a GIF/PNG/MP4 here, to add an example under `examples/`, or to fix how a scene looks or moves. Covers the scene design, the KDL idioms that work today, the render-and-look loop, and what must be updated before the change is done.
---

# Making an animation with the engine

A scene is a KDL file. `pxe` renders it deterministically, frame by frame, into palette-indexed pixels. You write shapes, expressions of time `t`, lights and text; you never draw pixels by hand unless you choose a `pixels` layer.

Authoritative references, in the order to consult them:

1. `docs/FORMAT.md` appendix A: the nodes and properties the loader accepts **today**. The rest of FORMAT.md is the design, and much of it is not built. If appendix A does not list it, it does not load.
2. `docs/SHORTHAND.md`: bare coordinates, `repeat=`, `motif`, `scatter`, `part`, moving groups, the shared kit.
3. `examples/`: working scenes. Find the one closest to the request and read it before writing.

## 1. Design before typing

An animation needs a subject and a change, not just the requested words or object.

- **Text alone is not an animation.** When asked for a quote, title or message, build a scene that carries its meaning (figures, weather, light, a place) and let the text ride on top. A text-only card was rejected outright in this repo; the version with two figures, rain and a sunrise was accepted.
- **Pick one change the viewer can feel:** night to dawn, apart to together, storm to calm, small to revealed. Tie every animated value to that arc.
- **Write the beats as times** before any shape: what is on screen at 0 s, when each thing starts and ends, when the text is readable, what the last held frame is.
- **Leave room.** Decide where text sits and where the subject sits so they never overlap; 320×180 at 12 fps is the house format for showcase scenes.

## 2. Skeleton

```kdl
// One or two lines saying what the scene shows.

use "parts/<name>/<name>-palette.kdl"

scene <name> palette=<palette> width=320 height=180 fps=12 duration="20s" seed=5 background=night.deep {
    timeline { marker judgment at="0.5s" }

    group storm "=1 - smoothstep(8.2, 10.8, t)"
    group dawn "=smoothstep(10.5, 16, t)"

    ambient darkness="=1 - @dawn.x"
    rect 0 0 320 141 paint=night.shade gradient=down gradient-steps=2
}
```

- `examples/<name>.kdl` holds exactly one scene or sequence. Everything it places goes in `examples/parts/<name>/`.
- Add `loop=#true` only for a seamless loop; otherwise the scene holds its last frame.
- Drawing order is file order: sky, far things, ground, subject, weather, lights, text.

## 3. Palette

Every pixel is a ramp plus a tone. Declare ramps dark to light, five tones each, so the tone names `deep shade base lit hi` all exist:

```kdl
palette quote {
    include studio
    ramp night "04050c 0a0d1e 131935 202950 344178"
    ramp sun "c8501e f08a32 ffb84e ffe08a fff6d0"
}
```

- `paint=night` is the base tone; `paint=night.lit` names a tone.
- `include` another palette to place a scene that uses it (a placed scene must use the host's palette or one it includes).
- Reuse `examples/parts/trailer/studio-palette.kdl` for text inks (`ink`, `paper`, and the accents `gold teal rose violet`).

## 4. Time: named values first

Declare every phase of the story once, as an empty `group` whose `x` is an expression, and read it everywhere with `@name.x`. This keeps timings in one place and makes retiming a one-line edit.

```kdl
group accuse "=smoothstep(0.4, 1.1, t) * (1 - smoothstep(7.6, 8.6, t))"
group stride "=step(9, t) * step(t, 12) * sin(tau * t * 1.5)"
```

Useful functions: `smoothstep(a, b, t)` for a 0→1 ramp between two times, `step(edge, x)`, `min max clamp lerp round`, `sin(tau * t)`, `osc(period, lo, hi)`, `wiggle(amp, freq)` for trembling or drifting, `pulse`, `chance`. Any numeric property accepts `"=…"`.

## 5. Building things

**Figures.** A small character is capsules and ellipses inside a moving group whose origin is at the feet. Pose it with the named values:

```kdl
group accuser x="=64 + 166 * @approach.x" y=140 {
    capsule 0 "=-14 + 8 * @kneel.x" "=5 * @stride.x" 0 radius=1.6 paint=coat.base
    capsule 0 "=-15 + 8 * @kneel.x" 0 "=-26 + 8 * @kneel.x" radius=3.6 paint=coat.lit
    ellipse -3 "=-36 + 8 * @kneel.x" 8 8 paint=skin.lit
    capsule arm 1 "=-25 + 8 * @kneel.x" "=2 + 11 * @accuse.x" "=-15 + 8 * @kneel.x - 10 * @accuse.x" radius=1.4 paint=coat.hi
}
```

Blend poses by adding terms that are each multiplied by a 0..1 value. A head that drops a pixel, a tremble from `round(wiggle(0.7, 3))`, an arm that lowers: small moves carry the emotion at this size.

**Light and mood.**

- `ambient darkness=N` lowers every non-emissive pixel by N tone steps. Animate it to move between night and day.
- `light-cone` and `light-point` raise tones where they reach. A visible beam is a separate low-`opacity` polygon; opacity dithers.
- `emissive=#true` ignores darkness and lights. Use it for suns, glows and all text.
- Fade a layer of the sky in with `opacity="=0.8 * @dawn.x"` on stacked rects.
- `reflection 0 141 320 39 axis=140 ripple=1 opacity=0.45` mirrors what is above the axis: wet ground, water.
- `particles` make rain (`vy=170 streak="0.05s"`), dust or sparks (`vy=-7 fade=#true`).

**Text.** `text "WORDS" x y size=2 align=center paint=paper.hi`. `reveal="=(t - 0.4) * 14"` types it at 14 characters per second; `scatter="=…"` animated to 0 assembles it from loose pixels, and animated up blows it away. For a shadow, draw the same text first, one pixel down-right, in a dark paint. Hold finished text at least 2 seconds before it leaves.

**Reusable pieces.** A scene with `param`s can be placed many times with different values, each on its own clock:

```kdl
use-scene verse start=judgment end="8.5s" opening="THE SINS YOU" verdict="EVIL" accent=rose verdict-size=5
```

`start` and `end` take times or marker names. Use a `part` for a static piece with its own origin, a `motif` for nodes that must read the scene's `@ids`.

**Camera.** `camera "=76 * @push.x" "=36 * @push.x" zoom="=1 + @push.x"` pushes in; zoom re-rasterizes, so nothing blurs. It moves everything, text included, so move it only when no text is on screen.

**Sequences.** For separate shots cut together, a file with `sequence name { shot scene duration="2s" …; transition "black" duration="0.75s" }` (FORMAT A.8). Prefer one continuous scene when a character must carry across the cut.

## 6. Look at it, every time

Never judge a scene from its text. Build once, then render the frames at the story's beats and read the PNGs:

```bash
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
cargo build --release --locked -p px-cli
target/release/pxe render examples/<name>.kdl --scale 2 --frame 70 --out <scratch>/f70.png
```

Frame number = seconds × fps. Check at least: the opening, each line of text fully shown, each transition midpoint, the final frame. Look for unreadable text, a subject lost in darkness, things showing through that should be hidden, and overlaps. Fix and re-render until each beat reads on its own. Then export:

```bash
target/release/pxe export examples/<name>.kdl --scale 3 --out out/<name>.gif
```

`out/` is ignored by git. Send the GIF to the user.

## 7. Pitfalls that cost time here

| Symptom | Cause | Fix |
|---|---|---|
| Text goes dim at night | `ambient darkness` darkens it | `emissive=#true` on every text |
| A sun or glow visible before it should rise | Emissive shapes ignore darkness and sit above the horizon | Start it fully below the ground line and draw the ground after it |
| Subject invisible | Dark ramps plus `darkness=2` | Use `darkness=1`, lighter paints (`.lit`, `.hi`) on the subject, and a brighter sky tone behind it |
| Param error on a placed scene | `x y scale flip id attach seed k repeat` are reserved | Name the param something else (`verdict-size`) |
| `@lamp-2` does not resolve | `-` before a digit is a minus | Build ids as `"lamp$k"` |
| First value read as an id | A plain string first on a shape is its id | Write coordinates as numbers or `"=…"` |
| Text gone before it can be read | Leave time too close to the reveal | Lengthen the placed scene and move its leave time |
| `--frame` shows scattered dots | That frame is inside a scatter transition | Pick a frame in the hold |

## 8. Finishing a new or changed example

1. Add or update its row in the examples table in `README.md`.
2. Record golden hashes (about 7 minutes; run it in the background):
   ```bash
   PX_BLESS=1 cargo test -p px-cli --test golden --locked
   ```
   `git status` must show only that example's file changed under `crates/px-cli/tests/golden/`.
3. Run `tools/check.sh` (20 to 30 minutes; background). It must end with `All checks passed.` A frame over 33.3 ms fails the bench step.
4. Report what was rendered and checked. Commit only when asked, in plain English, with no agent attribution of any kind (D-005).

## Repo rules that apply to scene work

- Engine code stays generic: put scene-specific content in KDL, never in a crate.
- English in files, comments and commit messages.
- Files stay under the size limits (docs 1000 lines; split a big scene into parts).
- `CONTRIBUTING.md` has the full contribution rules; `CLAUDE.md` has the always-on ones.
