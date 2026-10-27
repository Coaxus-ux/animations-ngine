# Animation (draft v1)

> Companion to [SPEC.md](SPEC.md). Covers time, modulators, keyframes, rigs, clips, gaits, state machines, secondary motion, particles, wind, simulation and loops. Shots, cameras and export live in [DIRECTOR.md](DIRECTOR.md). Syntax details are in [FORMAT.md](FORMAT.md), and the script API is in [SCRIPTING.md](SCRIPTING.md).

Everything here is scene-agnostic: the same systems drive a cat's tail (B1), kelp (B5), a pedestrian (B3) and a game hero (B4).

---

## 1. Time model

| Rule | Detail |
|---|---|
| **Integer frames** | Internal time is always an integer frame `f`. Seconds (`t`) are derived: `t = f / fps` |
| **Units** | `"36f"` frames, `"3s"` seconds, `"1.5s"` (rounded to the nearest frame), `"2b"` beats (later, with audio sync) |
| **Default rate** | 12 fps (set in `project.kdl`, overridable per sequence) |
| **Hold rate** | `on1` / `on2` / `on3`: content updates every 1, 2 or 3 frames. Settable per object, per clip or per expression (`on2(...)`) |
| **Pure by default** | `frame = render(scene, seed, f)`. Only stateful nodes (§9) break this, and they use checkpoints |
| **Local time** | Each object can shift or scale its time: `time-offset="0.5s"`, `time-scale=0.8`, `time-hold="on3"` |

`time-offset="=rand(i)*2s"` inside a `repeat` is the standard way to desync copies (trees, fish, pedestrians).

---

## 2. Modulators

Modulators are value generators you can connect to any numeric parameter, either as an expression or as a child `mod` node in the editor.

| Modulator | Params | Typical use |
|---|---|---|
| `sin` / `tri` / `saw` / `square` | `period`, `amp`, `phase` | Sway, pulse, blink |
| `wiggle` | `amp`, `freq`, `octaves` | Flicker, jitter, hand tremor |
| `noise` | `scale`, `speed`, `seed` | Organic drift (fog, clouds) |
| `pulse` | `period`, `width` | Neon flicker (B3), firefly glow (B2) |
| `random-hold` | `min`, `max`, `hold` | Values that jump and hold (window lights switching on) |
| `curve` | keyframed 0..1 curve, `period` | Custom loop shapes |
| `counter` | `start`, `step`, `every` | Frame counters, scrolling text |
| `wavetable` | list of values, `period` | Hand-made step sequences ("1,1,2,1,0") |
| `envelope` | `attack`, `hold`, `release` | One-shot events (a lightning flash) |

All modulators are **pure functions of time** and loop-aware: their `period` is reported to the loop solver (§11).

---

## 3. Keyframes

```kdl
anim intensity {
    k "0s"   1
    k "5s"   1 interp="step"
    k "5.2s" 0
    k "6s"   1 interp="smooth"
}
```

| Feature | Detail |
|---|---|
| **Targets** | Any property, including nested ones: `anim "light-cone.reach"` |
| **Interpolation** | `step`, `linear`, `smooth`, `ease-in`, `ease-out`, `ease-in-out`, `bounce`, `overshoot` |
| **Pixel snapping** | Position-like values snap to whole pixels after interpolation; other values snap to their type (int, enum, tone) |
| **Hold rate** | `hold="on2"` per track |
| **Extrapolation** | `before` / `after` = `hold` (default), `loop`, `ping-pong`, `continue` |
| **Relative tracks** | `relative=#true`: keys are offsets added to the base value (stack with modulators) |
| **Value + modulator** | The final value = base or keyframe + modulator, which lets you keyframe a lamp off while it keeps flickering |

---

## 4. Timing tools

| Tool | Effect | Example |
|---|---|---|
| `delay` | Shift a child's time | A car enters 2 s later (B3) |
| `stagger` | Each repeated copy gets `i × step` offset | Kelp stalks swaying in a wave (B5) |
| `remap` | Map time through a curve: slow motion, freeze, reverse | A hero's attack anticipation (B4) |
| `loop` | Repeat a time range of a child | A 1 s neon flicker loop in a 12 s scene |
| `ping-pong` | Loop forward/backward | Hanging sign swing |
| `freeze` | Hold one frame | Pose for a title card |
| `trigger` | Start a child on an event (from a state machine or script) | A bird takes off when the deer moves (B2) |

---

## 5. Rigs

### 5.1 Body plan templates

A **template** is a reusable skeleton family. A **body plan** is a template plus proportions, volumes and defaults. `human-adult`, `human-child` and `cat` (see [CATALOG.md](CATALOG.md)) are body plans built from templates.

| Template | Bones (core) | Volumes | Standard anchors | Locomotion |
|---|---|---|---|---|
| `biped` | pelvis, spine, chest, neck, head, shoulder/upper-arm/forearm/hand ×2, thigh/shin/foot ×2 | Ellipse head, rounded-box torso, capsule limbs | `crown`, `eyes`, `ear-l/r`, `mouth`, `torso`, `back`, `hand-l/r`, `foot-l/r`, `seat`, `weapon` | Walk, run, jump, climb |
| `quadruped` | pelvis, spine ×2–3, chest, neck, head, ears ×2, front leg ×2 (3 seg), hind leg ×2 (3–4 seg), tail chain | Ellipse body, circle/ellipse head, capsule legs, chain tail | `head-top`, `eyes`, `nose`, `mouth`, `collar`, `back`, `paw-*`, `tail-tip`, `saddle` | Walk, trot, gallop, sit |
| `bird` | body, neck chain, head, beak, wing ×2 (3 seg), tail fan, leg ×2 | Ellipse body, triangle beak, wing fans | `eyes`, `beak`, `back`, `feet` | Hop, glide, flap |
| `fish` | spine chain (4–8), head, tail fin, dorsal, pectoral ×2 | Tapered chain, fin triangles | `eyes`, `mouth`, `tail-tip` | Swim wave |
| `serpent` | spine chain (8–24), head, jaw | Capsule chain tapering to tail | `eyes`, `mouth`, `tail-tip` | Slither wave |
| `multi-leg` | body segments ×1–3, legs ×4–8 (2–3 seg), antennae | Ellipse segments, thin capsule legs | `eyes`, `back`, `leg-*` | Alternating tripod/wave gait |
| `machine` | User-defined joints: `hinge`, `slider`, `wheel`, `piston`, `rotor` | Boxes, cylinders, wheels | User-defined | Wheels roll by distance, rotors spin |

Templates define **bone names, hierarchy, default lengths (in head units), joint limits and standard anchors**. Plans fill in proportions and looks. `machine` covers cars (B3), carts, boats, robots, windmills (B6) and fans.

### 5.2 Defining a new creature

```kdl
body-plan "fox" template="quadruped" {
    build size=26 bulk=0.4 leg-length=0.8 tail-length=1.3 ear-size=1.4
    volume head shape="triangle-round" stretch=1.3      // pointier snout
    volume tail thickness=1.6 taper=0.3                 // bushy
    slots fur="fur-orange" fur-accent="cloth-cream"
    pattern "socks" mask="paw-*" slot="fur-accent"
    inherit clips="quadruped/*"                         // gait, sit, look, breathe…
}
```

- Every template ships its **generic clips** (§6), so a new creature moves on day one.
- `volume`, `bone` and `anchor` override or add parts. `remove bone="tail-*"` handles a tailless creature.
- New templates can be written in KDL too; they declare bones, joint limits and gait parameters.

### 5.3 Poses and IK

| Feature | Detail |
|---|---|
| **Pose** | A set of bone angles (degrees), optional per-bone pixel offsets and piece variants |
| **FK** | Default: angles propagate down the hierarchy |
| **IK** | Two-bone IK (arms, legs) and chain IK (tails, necks, tentacles) toward a `target` |
| **Targets** | A socket (`keyboard.keys:30%`), another object's anchor, a world point or a path |
| **Ground contact** | `foot-lock`: feet stay planted on a line socket during locomotion |
| **Look-at** | Head and eyes aim at a target within joint limits |
| **Joint limits** | Per template; poses are clamped |
| **Snapping** | After posing, joint positions snap to whole pixels before rasterizing, so limbs don't shimmer |

---

## 6. Clips

A clip is a reusable animation for a template. There are two kinds.

### 6.1 Pose-keyframe clips
```kdl
clip "wave" template="biped" channel="arms" length="1s" loop=#true {
    pose "0s"   upper-arm-r=-150 forearm-r=-20
    pose "0.5s" upper-arm-r=-150 forearm-r=30
    pose "1s"   upper-arm-r=-150 forearm-r=-20
}
```

### 6.2 Procedural clips (generators)
Generated from parameters at render time; no pose authoring.

| Generator | Params | Output |
|---|---|---|
| `gait` | `speed`, `stride`, `bounce`, `arm-swing`, `lean`, `style` | Walk/run cycles for any legged template |
| `breathe` | `period`, `amount` | Torso/belly 1 px rise |
| `blink` | `interval` range, `length` | Eye piece variants |
| `look` | `target`, `speed`, `idle-scan` | Head/eye aim |
| `sway` | `amount`, `period`, `chain` | Tails, antennae, hanging things |
| `type` | `targets`, `rate`, `jitter` | Alternating hand IK on a line socket |
| `swim` / `slither` | `wavelength`, `amplitude`, `speed` | Spine wave (fish, serpent, eel) |
| `flap` | `rate`, `glide-ratio` | Bird wing cycle |
| `jump` | `height`, `anticipation`, `hang`, `land-squash` | Arc + squash and stretch |
| `attack` | `kind` (`slash`, `thrust`, `punch`, `cast`), `wind-up`, `follow-through` | Game action clips (B4) |
| `hit` / `knockback` | `strength`, `direction` | Reactions |

Every clip declares its `template`, `channel`, `length`, `loop` and the bones it owns. A `quadruped` gait runs on a fox, a cat, a horse or a dog.

### 6.3 Channels and layering

| Channel | Examples | Blend |
|---|---|---|
| `posture` | `stand`, `sit-chair`, `sit-edge`, `crouch`, `lie` | Base pose |
| `locomotion` | `gait`, `swim`, `flap`, `slither` | Replaces legs/spine bones |
| `body` | `breathe`, `lean`, `stretch` | Additive |
| `arms` | `type`, `wave`, `attack`, `carry` | Replaces owned bones |
| `head` | `look`, `nod` | Replaces owned bones |
| `face` | `blink`, `talk`, expressions | Piece variants |
| `secondary` | Tails, hair, ears, cloth (§8) | Simulated after all others |

Higher channels override only the bones they own. Additive clips add angles. `weight` (0..1, animatable) cross-fades clips. Cross-fades are computed in pose space and then snapped, so they never produce blurry in-between pixels.

---

## 7. Gaits and locomotion

The gait system turns **movement** into **leg motion**, so characters walk along paths without hand-made cycles.

| Template | Gait model | Key params |
|---|---|---|
| `biped` | Two-phase walk, flight-phase run | `stride`, `bounce`, `arm-swing`, `lean`, `heaviness` |
| `quadruped` | Walk (4-beat), trot (diagonal pairs), gallop (rotary) | `gait`, `stride`, `spine-flex` |
| `multi-leg` | Tripod or wave gait | `phase-pattern` |
| `fish` | Traveling sine along the spine | `wavelength`, `tail-amp`, `burst` |
| `serpent` | Lateral undulation following a path | `wavelength`, `amplitude` |
| `bird` | Hop on the ground; flap/glide in the air | `flap-rate`, `glide-ratio` |
| `machine` | Wheels rotate by distance traveled | `wheel-radius` |

```kdl
character "walker" body="human-adult" {
    move path="sidewalk" speed="1.2 px/f" gait="walk"
}
```

- **Distance-driven:** cycle phase = distance traveled / stride, so feet never slide.
- **Foot lock:** planted feet stay fixed on the ground line.
- **Speed changes** blend walk → run automatically past a threshold.
- **In place:** `in-place=#true` animates the gait without moving, for sprite sheets (B4).

---

## 8. Secondary motion

### 8.1 Chain and rope sims
One verlet chain solver drives everything that dangles or trails:

| Use | Benchmarks |
|---|---|
| Tails, ears, antennae | B1 (cat), B2 (deer) |
| Hair, capes, scarves | B4 (hero cape) |
| Kelp, seaweed, vines | B5, B2 |
| Cables, ropes, chains, hanging signs | B1, B3, B6 |
| Curtains, flags, banners | B6 |

| Param | Meaning |
|---|---|
| `segments` | Chain resolution |
| `stiffness`, `damping` | Spring behavior |
| `gravity` | Direction and strength (negative for kelp buoyancy) |
| `wind` | Response to the wind field (§8.2) |
| `anchor` | Fixed bone or point |
| `mode` | `sim` (stateful, physically correct) or `procedural` (pure, a loop-friendly sine approximation) |

`procedural` mode is the default for loops because it is pure and loops exactly. `sim` is for one-shot shots and reactions.

### 8.2 Wind fields
A scene-wide **wind field** affects plants, hair, cloth, particles, rain angle and smoke at once:

```kdl
wind strength=0.4 direction=-10 gusts="=wiggle(0.3,0.2)" turbulence=0.2
```

- Anything with `wind` > 0 responds: trees and grass (B2), kelp (B5, using a `current` field), flags (B6), rain (B3).
- Fields can be local: `wind area="window"`, or `current` underwater.
- Wind is a pure function of position and time, so it never breaks looping.

---

## 9. Particles and groups

### 9.1 Particles
| Param | Meaning |
|---|---|
| `emitter` | `point`, `line`, `area`, `path`, `shape` |
| `rate`, `burst` | Spawn amount |
| `life`, `speed`, `direction`, `spread` | Motion |
| `forces` | Gravity, wind, attract, turbulence |
| `look` | Pixel (1 px), streak (rain), sprite (piece or small SDF), tone-over-life ramp |
| `collide` | Ground line, socket lines (rain splashes on a sill) |
| `mode` | `pure` (particles derived from seed + spawn time, loopable) or `sim` (stateful) |

**Pure particles** are the default. Each particle's state is computed from its spawn seed and its age, so they can be rendered at any frame and they loop. This covers rain (B1, B3), fireflies (B2), bubbles and marine snow (B5) and dust.

### 9.2 Flocks, schools and crowds
| System | Behavior | Use |
|---|---|---|
| `school` | Boids: separation, alignment, cohesion, obstacle avoidance | Fish (B5), birds (B2) |
| `swarm` | Loose orbit around a target | Fireflies (B2), insects |
| `crowd` | Agents on paths with spacing, each one a full character with a gait | Pedestrians (B3), villagers (B6) |

Boids are stateful (`sim`). For loops, `school` offers `loop-mode="orbit"`: agents follow closed noise-perturbed paths that loop exactly while still looking like a school.

---

## 10. State machines

```kdl
machine initial="idle" {
    state idle clip="breathe" {
        go-to "look" after="3s..6s" weight=2
        go-to "groom" after="5s..9s" weight=1
    }
    state look clip="look" duration="1.5s" { go-to "idle" }
    state groom clip="groom" duration="2s" { go-to "idle" }
    on "door-opened" go-to="look"         // event from a script or another machine
}
```

| Feature | Detail |
|---|---|
| **Transitions** | `after` (time or range), `when` (expression), `on` (event), `weight` (random choice, seeded) |
| **Blend** | `blend="0.25s"` cross-fades clips on entry |
| **Sub-machines** | A state can contain a machine (e.g. `combat` → `attack-1`/`attack-2`) |
| **Any-state** | `any on="hit" go-to="hurt"` |
| **Determinism** | Choices use the object's seed; the machine is stateful and checkpointed (§10.1) |
| **Loop mode** | `loop=#true` forces the machine to return to its initial state by the end of the loop length |

### 10.1 Stateful simulation and checkpoints
Feedback nodes, `sim` chains, boids, state machines and event scripts depend on the past.

| Rule | Detail |
|---|---|
| **Replay** | To render frame `f`, the engine simulates from the nearest checkpoint before `f` |
| **Checkpoints** | Saved every N frames (default 24) in memory, optionally on disk |
| **Invalidation** | Changing a parameter invalidates checkpoints after the earliest affected frame |
| **Fixed step** | Simulations always step once per frame, never by wall-clock time |
| **Warm-up** | `warm-up="5s"`: simulate before frame 0 so the scene starts settled (rain already falling, smoke already risen) |

---

## 11. Perfect loops

### 11.1 Loop solver
Every time-dependent node reports its **period**. The solver computes the scene loop length as the least common multiple of all periods, in frames.

| Situation | Solver behavior |
|---|---|
| All periods divide nicely | Loop length = LCM (e.g. 1 s, 3 s, 4 s → 12 s) |
| LCM too long (above `max-loop`, default 30 s) | Suggests the nearest period adjustments ("change `sway` 2.9s → 3s") or auto-snaps periods when `loop-snap=#true` |
| Random-hold / blink intervals | Sampled from a seeded sequence that repeats over the loop length |
| Pure particles | Spawn schedule wraps: particles alive at the end continue into the start |

### 11.2 Making stateful things loop
| Technique | Used for |
|---|---|
| **Procedural mode** | Chains, schools, particles switch to pure equivalents |
| **Warm-up + cross-dissolve** | Simulate 2× loop length, then dither-dissolve the tail into the head over `blend` frames |
| **Return-to-start** | State machines and crowds are constrained to end in their initial state and position |
| **Wrap spaces** | Crowds and clouds that exit one side re-enter the other (positions computed modulo the width) |

The validator reports which nodes are not loop-safe in the current shot.

---

## 12. Directions for game sprites

| Feature | Detail |
|---|---|
| **Direction sets** | `dirs=1` (side only), `2` (left/right by flip), `4` (N, E, S, W), `8` (+ diagonals) |
| **2D path (A)** | Native facings: `front`, `side`, `back`. 4 directions = front, back, side, flipped side. True 8 directions need path B |
| **3D path (B)** | Any number of directions from one 3D-primitive rig rendered per angle |
| **Flip rules** | `flip-ok=#true` mirrors the side view for the opposite direction; asymmetric pieces (an eye patch, a sword hand) can declare `no-flip` and need their own variant |
| **Per-direction pieces** | Pieces may provide `front`, `side`, `back`, `front-side`, `back-side` variants |
| **In-place clips** | Locomotion clips are rendered in place for sheets; root motion is exported as metadata |
| **Consistency** | All directions share the same seed, palette and pixel snapping, so frames line up across the sheet |

Example (B4):
```kdl
sprite-set "hero" body="hero-biped" dirs=8 {
    clip idle   frames=4  hold="on3"
    clip walk   frames=8
    clip run    frames=8
    clip jump   frames=6  loop=#false
    clip attack kind="slash" frames=6 loop=#false
}
```
Export layout and metadata are defined in [DIRECTOR.md](DIRECTOR.md) §7.
