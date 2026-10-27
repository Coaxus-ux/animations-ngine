# Scripting Reference (draft v1)

> Companion to [SPEC.md](SPEC.md). Format details in [FORMAT.md](FORMAT.md), time model in [ANIMATION.md](ANIMATION.md), editor integration in [EDITOR.md](EDITOR.md).

Scripts add **behavior** to nodes. They never contain the scene: the scene (KDL) contains scripts. Anything visual or reusable belongs in prefabs or Rust nodes. Anything per-pixel belongs in the expression language.

---

## 1. Attaching scripts

A script attaches to any node as a child. Extra properties become the script's `args`.

```kdl
character "kitty" body="cat" {
    script "cat-jump.ts" height=14 cooldown="4s..8s"
}
scene "forest-dusk" view="side" {
    script "fireflies-sync.ts"          // scene-level script
}
```

| Rule | Detail |
|---|---|
| Location | `scripts/` in the project, or a pack's `scripts/` folder |
| Owner | The node the script is attached to (`ctx.self`) |
| Multiple | A node may have several scripts; they run in file order |
| Reuse | One file can be attached to many nodes; each attachment is a separate **instance** with its own seed and state |
| Language | TypeScript or JavaScript. Types are stripped on load, keeping line and column positions |

---

## 2. The two script kinds

| | **Value script** | **Event script** |
|---|---|---|
| Declared with | `export default value(...)` | `export default events({...})` |
| Memory | None (pure function of the frame) | Persistent, JSON-serializable `state` |
| Runs | When a frame is rendered, in any order | Every simulation step, from frame 0 (or the warm-up) |
| Scrubbing | Instant, any frame | Replays from the nearest checkpoint |
| Can change | Owner's properties, for this frame only | Properties, states, events, spawned objects |
| Use for | Derived motion, sync, procedural values | Reactions, chains of events, agents, game-like logic |

**Rule of thumb:** use a value script whenever the behavior can be written as "given the time, compute X". Use an event script only when something has to *remember* what happened.

---

## 3. Entry points

### 3.1 Value script
```ts
import { value } from "engine";

export default value((ctx) => {
  return { intensity: 1 + 0.1 * ctx.math.sin(ctx.t * 6) };
});
```
The returned object is merged into the owner's properties for that frame. Keys and value types are checked against the owner's schema.

### 3.2 Event script
```ts
import { events } from "engine";

export default events({
  emits: ["jumped"],                    // declared for generated types
  state: { jumps: 0 },                  // initial state (JSON only)
  init(ctx) {},                         // once, at the first simulated frame
  tick(ctx) {},                         // every simulation step
  on: {
    "state:enter"(ctx, ev) {},
    "landed"(ctx, ev) {},
  },
});
```

| Entry point | Called |
|---|---|
| `init(ctx)` | Once, at the first simulated frame (after the warm-up starts) |
| `tick(ctx)` | Every step (default: once per frame at the scene fps; `step="2f"` on the `script` node to run less often) |
| `on[name](ctx, ev)` | When an event with that name reaches this instance |

Handlers always run in a fixed order: by script node order in the file, then by event emission order. Deferred work uses **named** handlers, never closures, so the state stays serializable:
```ts
ctx.after("0.4s", "land", { height: 12 });   // calls on.land later
```

---

## 4. The context object (`ctx`)

### 4.1 Time and identity
| Member | Type | Notes |
|---|---|---|
| `f` | int | Current frame |
| `t` | number | Current time in seconds |
| `dt` | number | Step duration in seconds (event scripts) |
| `fps` | int | Scene fps |
| `duration` | number | Scene/shot duration in seconds |
| `seed` | int | This instance's seed (global seed + node id + script index) |
| `args` | object | Properties given on the `script` node, parsed with units (`"4s"` → seconds) |
| `params` | object | The scene's exposed params (`$rain`) |
| `self` | `Obj` | Handle to the owner node |

### 4.2 Randomness and math
| Member | Notes |
|---|---|
| `rand()` | Next number in [0,1) from this instance's stream. `Math.random` is redirected here |
| `rand.range(a, b)`, `rand.int(a, b)`, `rand.pick(list)`, `rand.chance(p)` | Helpers on the same stream |
| `hash(...values)` | Stateless random in [0,1) from any values (safe in value scripts) |
| `noise(x, y?, z?)` | Seeded smooth noise, same as the expression language |
| `math.*` | Deterministic `sin`, `cos`, `pow`, `exp`… (the `Math` object is redirected here too) |
| `units("3s")` | Converts unit strings to seconds; `frames("3s")` converts to frames |

In value scripts, `rand()` restarts every frame from a fixed seed. Use `hash(i, f)` for per-object variation.

### 4.3 Reading the scene
| Member | Returns |
|---|---|
| `get(id)` | `Obj` handle by id (`get("lamp")`) |
| `query({ prefab?, body?, tag?, layer?, within? })` | List of `Obj`. `within: { x, y, r }` or `{ area: "window" }` |
| `obj.props` | Read-only evaluated properties at the current frame |
| `obj.pos` | `{ x, y, z }` in scene pixels |
| `obj.bounds` | `{ x, y, w, h }` |
| `obj.socket(name)` | Socket position(s) (`get("desk").socket("top")`) |
| `obj.state` | Current state-machine state, if any |
| `obj.tags` | Tags from the KDL `tags="…"` property |

Value scripts may read objects that come **earlier** in the dependency graph. A dependency cycle is a load error that names both nodes.

### 4.4 Writing (event scripts)
| Member | Effect |
|---|---|
| `obj.set(props)` | Sets properties until changed again. Setting placement (`on`, `at`) re-attaches the object to another socket, which is how hand-offs and pick-ups work (`ball.set({ on: "pip.mouth" })`, B8) |
| `obj.tween(props, duration, interp?)` | Animates properties; pixel-snapped; `interp` as in keyframes |
| `obj.setState(name)` | Switches the object's state machine (fires `state:exit` / `state:enter`) |
| `obj.play(clip, { channel?, loop? })` | Plays a clip on a character channel (see [ANIMATION.md](ANIMATION.md)) |
| `emit(name, payload?, { to? })` | Sends an event: to everyone, to an `Obj`, or to a tag (`to: "#school"`) |
| `after(time, handler, payload?)` | Calls a named handler later |
| `spawn(prefab, props) → Obj` | Creates an object (it can be a prefab from any loaded pack) |
| `despawn(obj)` | Removes a spawned object |
| `state` | This instance's persistent state object |
| `log(...)`, `watch(name, value)` | Console output and live values in the inspector |

Value scripts have only `return`. Calling a write method in a value script is an error.

### 4.5 Built-in events
| Event | Payload | Sent when |
|---|---|---|
| `state:enter` / `state:exit` | `{ obj, state }` | A state machine changes state |
| `clip:end` | `{ obj, clip, channel }` | A non-looping clip finishes |
| `clip:marker` | `{ obj, clip, marker }` | A clip passes a named marker (`contact`, `hit`) |
| `cue` | `{ name, data }` | A timeline cue is reached ([ANIMATION.md](ANIMATION.md)) |
| `overlap:start` / `overlap:end` | `{ a, b }` | The bounds of two objects with `overlap=#true` start or stop touching |
| `loop` | `{ count }` | The scene wraps around in a loop |

---

## 5. Generated types

The engine writes TypeScript declarations into `.engine/types/` every time scenes, packs or scripts change:

| File | Contents |
|---|---|
| `engine.d.ts` | The `engine` module: `value`, `events`, `Ctx`, `Obj` |
| `scenes/<scene>.d.ts` | Union of real ids, the props of each object by its prefab schema, state names, params |
| `packs/<pack>.d.ts` | Prefab and body plan props, clips, sockets, anchors |
| `events.d.ts` | All event names declared in `emits`, with their payload types |

With these, `ctx.get("lamp").set({ intensiti: 1 })` is a red squiggle in VS Code. A `tsconfig.json` pointing at `.engine/types` is created by `new` (see [CLI.md](CLI.md)).

---

## 6. Sandbox and determinism

| Allowed | Not allowed |
|---|---|
| `import` of other project/pack scripts and of `"engine"` | File system, network, environment variables |
| Pure JS/TS (classes, arrays, maps) | `Date.now()` (it returns the scene's fixed epoch) |
| `Math.*` (redirected to deterministic versions) | Unseeded randomness (`Math.random` is redirected) |
| `console.log` (goes to the editor console) | `setTimeout`, `setInterval`, async/Promises (use `after`) |
| | npm packages (a vendored pure-TS file in `scripts/` works) |

**Guarantees:**
- Same project, seed and engine version → identical frames on every machine.
- Each instance has isolated globals; scripts can't share hidden state. They talk through events and object properties.
- `state` must be JSON-serializable. Functions, cycles or class instances inside it are a load error.

---

## 7. Checkpoints, loops and warm-up

- Event scripts are simulated from frame 0. The engine saves a **checkpoint** (all script states, spawned objects, set properties) every 60 frames (`checkpoint-every` in `project.kdl`).
- Scrubbing resumes from the nearest earlier checkpoint.
- Changing a script file invalidates all checkpoints for that scene. Changing a KDL property invalidates only checkpoints after the first frame it affects.
- `warmup="4s"` on the scene runs the simulation before frame 0. It is used so loops start mid-motion (fish already schooling, fireflies already scattered).
- The loop solver ([DIRECTOR.md](DIRECTOR.md)) warns when an event-driven scene's state at the end doesn't match the start. Fix it with value scripts, cyclic state, or a dithered `loop-blend`.

---

## 8. Performance limits

| Limit | Default | When exceeded |
|---|---|---|
| Time per value-script call | 2 ms | Warning in the console |
| Script time per frame (all instances) | 16 ms | Warning; preview drops to step-through |
| Hard timeout per call | 1 s | Script disabled, error with stack |
| Memory per instance | 16 MB | Script disabled, error |
| `state` size | 256 KB | Checkpoint error |

**Patterns:**
- **Many similar agents** (200 fish, 80 fireflies): attach **one** script to the group and drive members through `query`. Don't attach 200 scripts.
- **Value scripts that never read `t`/`f`** are detected and cached as static.
- **Per-pixel work** never goes in scripts. Use expressions or nodes ([NODES.md](NODES.md)).

---

## 9. Debugging

- **Errors** show both locations: `scripts/cat-jump.ts:14:9` and the attaching node `scenes/room.kdl:31 (character "kitty")`. Both are clickable in the editor console.
- **Last good frame:** on an error, the preview keeps the last good render with a red banner.
- **Logs** are tagged with frame and instance: `[f=128 kitty#cat-jump] jumped 14`.
- **`watch(name, value)`** shows live values next to the owner's properties in the inspector.
- **Step mode:** pause and step frame by frame. Since everything is deterministic, a bug at frame 300 reproduces exactly every time.

---

## 10. Examples

### 10.1 B1 — Cat jumps onto the TV, and the pendant lamp sways
```kdl
character "kitty" body="cat" tags="pet" { script "cat-jump.ts" target="tv.top" }
pendant-lamp "lamp" { script "sway-on-event.ts" event="landed" }
```
```ts
// cat-jump.ts
export default events({
  emits: ["landed"],
  init(ctx) { ctx.after(ctx.rand.range(3, 6), "jump"); },
  on: {
    jump(ctx) { ctx.self.play("jump"); },
    "clip:marker"(ctx, ev) {
      if (ev.marker === "contact") ctx.emit("landed", { x: ctx.self.pos.x });
    },
    "clip:end"(ctx) { ctx.self.setState("sit"); },
  },
});

// sway-on-event.ts
export default events({
  on: {
    landed(ctx) {
      ctx.self.tween({ swing: 0.6 }, "0.2s", "ease-out");
      ctx.after("0.2s", "settle");
    },
    settle(ctx) { ctx.self.tween({ swing: 0 }, "2s", "smooth"); },
  },
});
```

### 10.2 B3 — A passing car splashes the puddles
```kdl
car "taxi" overlap=#true { script "car-pass.ts" speed=40 }
puddle tags="puddle" overlap=#true   // ×N
```
```ts
export default events({
  tick(ctx) {
    const x = (ctx.t * ctx.args.speed) % 320 - 32;
    ctx.self.set({ x: Math.round(x) });
  },
  on: {
    "overlap:start"(ctx, { b }) {
      if (b.tags.includes("puddle"))
        ctx.spawn("splash", { x: b.pos.x, y: b.pos.y, size: 3, life: "0.5s" });
    },
  },
});
```

### 10.3 B5 — A fish school avoids the diver
```kdl
group "school" tags="school" { repeat n=60 layout="area" { character body="fish" } }
character "diver" body="biped" pose="swim" { script "diver-path.ts" }
group "school" { script "school.ts" avoid="diver" radius=24 }
```
```ts
// school.ts — one script steers all members (simple boids)
export default events({
  state: { v: {} as Record<string, [number, number]> },
  tick(ctx) {
    const fish = ctx.query({ tag: "school", body: "fish" });
    const diver = ctx.get(ctx.args.avoid).pos;
    for (const f of fish) {
      let [vx, vy] = ctx.state.v[f.id] ?? [1, 0];
      const dx = f.pos.x - diver.x, dy = f.pos.y - diver.y;
      const d2 = dx * dx + dy * dy;
      if (d2 < ctx.args.radius ** 2) { vx += dx / 8; vy += dy / 8; }
      // cohesion and alignment omitted for brevity
      ctx.state.v[f.id] = [vx, vy];
      f.set({ x: f.pos.x + Math.round(vx), y: f.pos.y + Math.round(vy), flip: vx < 0 });
    }
  },
});
```

### 10.4 B4 — Hero attack combo (sprite sheet)
Timeline cues stand in for player input, and the export captures each state as sheet rows.
```kdl
character "hero" body="biped" facing="side" {
    machine initial="idle" { state idle clip="idle"; state attack-1 clip="slash"; state attack-2 clip="slash-up"; state attack-3 clip="spin" }
    script "combo.ts" window="0.25s"
}
timeline { cue "attack" at="1s"; cue "attack" at="1.4s"; cue "attack" at="1.8s" }
```
```ts
export default events({
  state: { step: 0, buffered: false },
  on: {
    cue(ctx, ev) {
      if (ev.name !== "attack") return;
      if (ctx.state.step === 0) { ctx.state.step = 1; ctx.self.setState("attack-1"); }
      else ctx.state.buffered = true;              // queued for the combo window
    },
    "clip:end"(ctx) {
      if (ctx.state.buffered && ctx.state.step < 3) {
        ctx.state.step += 1; ctx.state.buffered = false;
        ctx.self.setState(`attack-${ctx.state.step}`);
      } else { ctx.state.step = 0; ctx.self.setState("idle"); }
    },
  },
});
```

### 10.5 B2 — Fireflies that drift into synchronized blinking (pure value script)
```kdl
scatter n=80 area="clearing" { firefly { script "firefly-sync.ts" sync-by="8s" } }
```
```ts
// No memory needed: each fly's phase converges to the shared phase over time.
export default value((ctx) => {
  const own = ctx.hash(ctx.self.id) * 2 * Math.PI;
  const k = Math.min(1, ctx.t / ctx.args["sync-by"]);   // 0 → 1
  const phase = own * (1 - k);                          // drifts to 0
  const on = Math.sin(ctx.t * 4 + phase) > 0.6;
  return { brightness: on ? 1 : 0 };
});
```

### 10.6 B6 — Village lights turn on house by house at dusk (value script)
```kdl
scene "village" view="isometric" { param daylight 1 min=0 max=1 }
house tags="house" { script "dusk-lights.ts" }   // ×N
```
```ts
export default value((ctx) => {
  const threshold = 0.2 + 0.4 * ctx.hash(ctx.self.id);   // each house has its own moment
  return { "windows-lit": ctx.params.daylight < threshold ? 1 : 0 };
});
```
KDL property names are kebab-case, so in TS they are written as quoted keys (`"windows-lit"`). The generated types include them.
