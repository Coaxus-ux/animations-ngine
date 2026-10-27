# Editor (draft v1)

> Companion to [SPEC.md](SPEC.md). Format in [FORMAT.md](FORMAT.md), scripts in [SCRIPTING.md](SCRIPTING.md), command line in [CLI.md](CLI.md).

The editor is a desktop app (Rust + egui) inspired by Pixel Composer. Its job is to **see, tweak and assemble**; the KDL files remain the source of truth. Everything the editor does, it does by editing those files, and you can keep writing them in VS Code at the same time.

---

## 1. Layout

```
┌──────────────┬───────────────────────────────┬───────────────┐
│ PROJECT      │                               │  INSPECTOR    │
│ LIBRARY      │           PREVIEW             │               │
├──────────────┤                               │               │
│ HIERARCHY    │                               │               │
├──────────────┴───────────────────────────────┴───────────────┤
│ TIMELINE                                                      │
├───────────────────────────────────────────────────────────────┤
│ NODE GRAPH · PALETTE · PIECE · RIG · CONSOLE   (tabbed dock)  │
└───────────────────────────────────────────────────────────────┘
```
Every panel can be docked, tabbed or floated. Layouts are saved per user, not per project.

---

## 2. Panels

### 2.1 Preview
| Feature | Detail |
|---|---|
| Zoom | Integer only (1×–16×); fit-to-window picks the largest integer |
| Pan | Space+drag or middle mouse |
| Pixel grid | Toggle; shown from 4× zoom |
| Selection | Click selects the object under the cursor (via the object-id buffer); Alt+click cycles through overlapping objects |
| Move gizmo | Drag moves by whole pixels and writes `x`/`y` (or re-sockets when dropped on a socket) |
| Overlays | Sockets, anchors, bounds, light shapes, camera frame, safe area, sprite-sheet cell grid |
| Onion skin | Previous/next frames tinted, for clips |
| Compare | Split view: two seeds, two palettes, or before/after cleanup |
| Pick | Hover shows ramp, tone, material slot and owning object |
| View badge | Shows the scene `view` (`side`, `perspective`, `top-down`, `isometric`) |

### 2.2 Inspector
Shows the selected object's parameters, grouped as its schema declares (Shape, Materials, Light, Animation…).

| Element | Behavior |
|---|---|
| Value field | Typed editor: number drag, slider for ranged params, dropdown for enums, ramp picker for slots |
| **◆ toggle** | Makes the parameter animatable: the first click adds a keyframe at the current frame; later edits on another frame add keys |
| Kind menu (⋯) | Switch between fixed, range, expression, param, reference, keyframed |
| Expression field | Autocomplete for functions, `$params` and `@refs`, plus a mini plot of the value over the scene duration |
| Seed 🎲 | Rerolls this object's seed; Shift+click locks it |
| Reset | Back to the schema default (removes the property from KDL) |
| Source link | Jumps to the KDL line in your external editor |
| Watches | Live `watch()` values from scripts |

### 2.3 Timeline
| Track type | Content |
|---|---|
| Property | Keyframes for one animated property |
| Clip | Clips playing on a character's channels (posture, body, arms, head, face) |
| State | Colored bars with the state machine's state per frame (read-only, computed) |
| Cue | Named markers that emit `cue` events |
| Camera | Camera keys for the current shot |
| Sim | Checkpoint markers and simulation progress for event scripts/feedback |
| Audio | Waveform for beat-syncing (later milestone) |

Controls: play/pause, step ±1 frame, loop region, playback at scene fps or **preview on2/on3**, ruler in seconds or frames. Drag keys to retime; right-click a key to change interpolation (`step`, `linear`, `smooth`, `ease-in`, `ease-out`, `bounce`).

### 2.4 Hierarchy
The scene tree: layers → objects → children.
- Toggles per row: visible, lock, solo.
- Drag to reorder (changes draw order inside a layer), reparent, or drop onto a socket.
- Disable writes `/-` in front of the node; enable removes it.
- Spawned objects (from event scripts) appear in italics and are read-only.

### 2.5 Project and library
- **Project:** scenes, sequences, palettes, pieces and scripts of the project, plus its pack dependencies.
- **Library:** browses every loaded pack (`core-kit`, `interior`, `nature`, `urban`, `water`, `creatures`, user packs).
  - Thumbnails are rendered live with the current palette and view.
  - Filter by category, tag, supported `view`, static/animated.
  - Drag a prefab into the preview or hierarchy: inserts a KDL node with only the required params.
  - **Fork to project:** copies a pack prefab into `prefabs/` so it can be edited (pack files are read-only).

### 2.6 Node graph
Pixel Composer-style graph of the selected object or prefab.
- Double-click a prefab to open its internal graph; breadcrumbs to go back up.
- KDL prefabs are editable; Rust prefabs and pack prefabs are read-only (fork first).
- Wires show live previews (thumbnail for canvases, sparkline for values).
- Tunnels (send/receive) are shown as labeled stubs.
- **Group to prefab:** select nodes → new prefab file with chosen params exposed as `input`s.

### 2.7 Palette panel
- Every ramp as a strip of tones, with role badges (`wall`, `skin-3`…), group membership and twins.
- `ramp-gen` controls (base color, hue-shift, contrast) with live update.
- Warnings from palette validation ([PALETTES.md](PALETTES.md) §7) and the unique color count.
- Quick swap: try any palette on the current scene without changing the scene file; "apply" writes `palette=`.

### 2.8 Piece editor
- Shows the text grid of a `.piece` (or `piece` node) with its legend.
- Click cells to paint with legend characters; the editor writes the grid text back. Painting is optional: typing in VS Code is equally valid.
- Tabs per variant; mirror preview for `mirror=#true`; pivot and anchor overlay.
- Live preview of the piece on a chosen character and palette.

### 2.9 Rig viewer
For body plans: skeleton, bone volumes, anchors and slots.
- Switch facing, posture and clip; scrub clips.
- Sliders for build params (`height`, `bulk`, proportions); writes to the character or the body plan.
- Shows IK targets and contact points.
- Works for every template: `biped`, `quadruped`, `bird`, `fish`, `serpent`, `multi-leg`, `machine`.

### 2.10 Console
- Errors, warnings, info and script logs; each line links to the KDL/TS location.
- Filters by level, file, object; script logs are tagged with frame and instance.
- Errors keep the preview on the last good frame with a red banner.

---

## 3. Round-trip rules (editor ↔ KDL)

The editor patches files **in place** using a format-preserving KDL parser. Comments, blank lines, property order and unknown nodes survive every edit.

| The editor may | The editor never |
|---|---|
| Change literal values | Rewrite an expression it didn't create (it edits the expression *text* instead) |
| Add or remove properties | Touch TypeScript files |
| Add, edit and remove `anim` blocks | Edit pack files (fork to project first) |
| Insert, delete, move and reparent nodes | Reorder or reformat lines it didn't change |
| Toggle `/-` | Change units you wrote (`"3s"` stays seconds, even if dragged in frames) |
| Edit palettes, pieces and param defaults | |

**Node identity.** Edits are applied to nodes by id when they have one (`desk-lamp "lamp"`), otherwise by path (`layer room / shelf[0]`). Giving ids to objects you plan to tweak keeps edits robust when you also rearrange code by hand.

**Saving.** Changes are written to disk immediately (debounced ~300 ms), so the file is always the truth. Undo/redo is kept by the editor as text patches and works across both manual and editor edits.

**External changes.** When a file changes on disk (VS Code, git), the editor reloads it. If the editor had a pending edit on the same node, it merges by node; on a real conflict it asks which version to keep.

### 3.1 Bake
**Bake** turns procedural results into explicit KDL. It is one-way.

| Bake | Result |
|---|---|
| A `repeat`/`scatter` | Explicit child nodes with resolved values |
| Ranges and seeded values | Fixed values |
| An event-script simulation | Keyframes on the affected properties |
| A procedural character pose | A pose clip file |

The original node is kept right above, disabled with `/-`, so you can revert by hand.

---

## 4. Hot reload

| Change | What reloads |
|---|---|
| KDL scene or prefab | Only nodes whose inputs changed; static caches are kept where possible |
| Palette | Final color resolution only (no re-render of geometry) |
| Piece | Characters using it |
| TypeScript | The script's instances; checkpoints for that scene are invalidated |
| Pack added/updated | Library and every scene using it |

Target: the preview updates in under a second after saving.

---

## 5. Debug views

Toggle from the preview toolbar or with `F1`–`F10`.

| View | Shows |
|---|---|
| Material | Each ramp as a flat false color |
| Tone | Tone index as grayscale |
| Light | Light steps per pixel (positive warm, negative cold) |
| Depth | Depth buffer |
| Emission | Emissive pixels only |
| Object id | Each object as a flat color |
| Rig | Bones, volumes, anchors, IK targets |
| Sockets | All sockets and anchors with names |
| Cache | Static (cached) vs animated nodes |
| Cleanup diff | Pixels changed by cleanup passes |
| Palette usage | Colors used per frame, and colors never used |

---

## 6. Shortcuts

| Action | Shortcut |
|---|---|
| Play / pause | Space |
| Step frame | ← / → |
| Jump to start / end | Home / End |
| Add keyframe on hovered param | K |
| Toggle ◆ on hovered param | Shift+K |
| Reroll seed of selection | R |
| Set loop region from selection | L |
| Zoom in / out | + / − |
| Fit to window | 0 |
| Toggle pixel grid | G |
| Toggle onion skin | O |
| Debug views | F1–F10 |
| Open selection in external editor | Ctrl/Cmd+E |
| Open node graph of selection | Enter |
| Back up one graph level | Backspace |
| Disable / enable node (`/-`) | Ctrl/Cmd+/ |
| Duplicate node | Ctrl/Cmd+D |
| Bake selection | Ctrl/Cmd+B |
| Export dialog | Ctrl/Cmd+Shift+E |
| Command palette | Ctrl/Cmd+P |

---

## 7. Projects, scenes and sequences

- One window per project; open scenes as tabs.
- **Sequence view:** a strip of shots with transitions; click a shot to edit its scene and camera.
- Exposed params (`param`) are shown in a **Params** strip above the timeline for quick tweaking.
- Packs are listed in `project.kdl`; the project panel shows missing or outdated packs.

---

## 8. Export dialog

| Option | Values |
|---|---|
| Target | Scene, sequence, character, tileset generator |
| Format | GIF, MP4, WebM, APNG, WebP, PNG sequence, **sprite sheet**, **tileset**, **parallax layers** |
| Range | Whole scene, loop region, or **auto loop** (loop solver) |
| Scale | Integer, nearest-neighbor |
| Palette | Scene palette or any other |
| Variations | N seeds or a list of palettes → N files |
| Sprite sheet | Clips × directions (4 or 8) × frames; layout rows/cols; padding; metadata JSON (frames, durations, pivots, anchors) |
| Tileset | Tile size, autotile rule set (16 or 47 tiles), PNG + metadata |
| Parallax layers | One PNG/animation per layer with its parallax factor in metadata |

Presets are saved as `export` nodes in `project.kdl`. **Copy as CLI** puts the equivalent `export` command on the clipboard ([CLI.md](CLI.md)).
