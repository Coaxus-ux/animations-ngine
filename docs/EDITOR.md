# Editor (draft v1)

> Companion to [SPEC.md](SPEC.md). Format in [FORMAT.md](FORMAT.md), scripts in [SCRIPTING.md](SCRIPTING.md), command line in [CLI.md](CLI.md).

The editor is a desktop app whose every pixel, icons included, is drawn by the engine itself (D-012), inspired by Aseprite and Pixel Composer. Its job is to **paint, see, tweak and assemble**; the KDL files remain the source of truth. Everything the editor does, it does by editing those files, and you can keep writing them in VS Code at the same time.

---

## 0. Today: `pxe edit`

> **Today (2026-11-01):** `cargo pxe edit [file.kdl] [--size WxH] [--scale N]` opens the editor; without a file it opens on the Home tab (recent files, New, Open). Several files are open at once as tabs under the menu bar (D-016); files are chosen with the system's own dialogs (D-017). View > Detail 1x–4x redraws the canvas at that detail (D-025) while tools, hover and picking keep working in scene pixels; frames rendered during playback are kept packed in memory (up to 320 MB) so later loops play without rendering, and playback follows the clock, skipping frames when a render falls behind instead of slowing down. Sections 1 onward describe the full design; this section is what exists. The architecture is in [ARCHITECTURE.md §2.2](ARCHITECTURE.md#22-the-editor-today-d-012-d-015-d-016-d-017).

**Architecture (D-015).** `px-ui` is a toolkit with no window or files: an indexed UI canvas in a theme palette ([`assets/theme-dark.kdl`](../crates/px-editor/assets/theme-dark.kdl), a light one too), the engine's pixel font, icons rendered from KDL scenes of `pixels` ([`assets/icons-*.kdl`](../crates/px-editor/assets/)), and immediate-mode widgets. `px-editor` holds the domain (documents, layers, commands, history), tools (one `Tool` type each), views (panels that only emit intents) and the app (windows, dispatch). `pxe edit` is the only adapter: minifb windows and the file system.

**Source of truth (D-014).** The scene's KDL text. Every action is a node-level splice of the text followed by a reload, so `git diff` shows exactly what changed. One undo history (Ctrl+Z / Ctrl+Shift+Z, 200 steps) covers strokes, layer edits, clicks and typed code. A stroke previews in memory and becomes one entry when the button is released.

**Finding things.** Every action is in the menu bar (File, Edit, Sprite, Layer, Frame, Palette, View, Window, Help) with its shortcut, so nothing is reachable only through a key or a gesture. Tools sit down the right; the color bar is down the left with the two paints (main and second) below it; the bar under the menus names the current tool, says what the mouse does with it, shows its options (Filled, Every matching pixel, Use main color) and warns when the active layer can't be painted. F1 (Help menu) shows how to use everything. Menus, tooltips and dialogs draw on a top layer, above the canvas. The UI size follows the window (View > UI size to fix it), and the inspector folds into the Window menu when the window is narrow.

| Area | What it does |
|---|---|
| Home and tabs | Aseprite's arrangement: the Home tab (recent files, New, Open) and one tab per open file under the menu bar, `*` when unsaved, `x` or middle click closes (Ctrl+W asks first when dirty, Ctrl+Shift+W closes all), Ctrl+Tab and Ctrl+Shift+Tab cycle, Ctrl+Home shows the Home tab; the Window menu has the same entries. Each tab keeps its own tool, paints, selection, zoom and frame. |
| Files | New (Ctrl+N: name, width, height, fps, frames, palette preset, background) makes an untitled tab, saved on first Save; Open (Ctrl+O), Save As (Ctrl+Shift+S) and every export path use the system file dialog; recent files and the last folder persist in `~/.config/px/editor.kdl`. |
| Export | File > Export As (Ctrl+Alt+Shift+S) and Export Sprite Sheet (Ctrl+E, opens on the sheet kind): GIF (all frames or the current one), the current frame as PNG, a PNG sequence numbered with four digits, or a sprite sheet for games (horizontal or vertical strip, rows or columns, border and shape padding, 1–16× scale, transparent background by keyed render (D-018), Aseprite-compatible JSON that Godot, Unity and Phaser read). A GIF with Transparent background keys its frames and disposes them to the background; Trim sprite crops every frame to the box their opaque pixels share and the JSON records the offset (`trimmed`, `spriteSourceSize`, `sourceSize`); it only removes transparent pixels, so it needs Transparent background (or a scene with transparent pixels) and leaves a frame-filling box untrimmed; Split layers writes one sheet or sequence per visible layer as `<name>-<layer>.png`; the scene's timeline markers become `frameTags`, each running to the frame before the next marker. Ctrl+Shift+X repeats the last export for the active file. |
| Canvas | Integer zoom (wheel, `+`/`-`, `0` fits), pan with Space+drag or the middle button, pixel grid from 6× (toggle), onion skin of the previous and next cel, marching ants around the selection, a pixel cursor. Left button paints with the primary paint, right with the secondary. |
| Tools | Pencil (pixel-perfect: L-corners are removed), eraser, fill (Shift: every matching pixel), line, rectangle, ellipse (outline or filled; Shift: 45° and squares), eyedropper, box select, lasso, move, pick. |
| Selection | Ctrl+C / X / V, Delete, H / Shift+H flip, Ctrl+A, Ctrl+D / Esc. Tools only change selected pixels. |
| Layers | Pixel layers (`layer` + `pixels`, FORMAT A.9) and shape layers in file order, top first: show/hide per window, lock, double click to rename, drag to reorder, opacity, add, delete, move. |
| Timeline | A ruler in seconds (click or drag on it to scrub; it pages when the playhead leaves the view, and long scenes get narrower frames so about six seconds show at once), a row of the scene's `timeline` markers (click one to jump to it), one read-only row per animated object (the camera, ambient, groups, shapes, lights) with a diamond at every key of any of its properties (click a key to select the object and jump to that frame; click the name to select it), then frames × pixel layers: a cel starts at a dot and holds through the bar. New cel (a copy), delete cel, add or remove a frame, play (Enter), step with `,` and `.`. Keys of placed scenes are not listed, and keys can't be dragged yet. |
| Palette | Aseprite's color bar down the left. The scene's ramps as swatches in their real colors. Click picks the paint (right click: secondary, X swaps); with the pick tool (O) and a picked object, one click rewrites that object's `paint=` in the file. Double click opens a color picker (RGB sliders, hex) that rewrites the color in the `palette` node, recoloring everything that uses it. |
| Palette editing | F4 (Palette menu; the menu also adds, removes and generates ramps and tones for the selected ramp) turns the color bar into the palette editor: add or remove ramps and tones, rename, pick colors with a hue bar and saturation/value square or a hex field, generate a ramp from one base color (steps, hue shift, contrast, saturation; PALETTES.md §5), import Lospec `.hex` or GIMP `.gpl` (colors are clustered into ramps by hue family, grays apart, each sorted dark to light and split past eight tones; PALETTES.md §6.1), save the palette as a standalone `.kdl`, or apply a shipped preset. Removing a ramp, or a tone, that someone uses is refused and the first user is named. Adding, renaming, recoloring or removing a ramp is one `ramp` line in the file; import and presets rewrite the `palette` node. Each edit is one undo step. |
| Elements | The current layer's nodes as a tree; the funnel button in its title bar switches to every node of the file (palette, ramps, scene, layers, pixels, shapes, `use`). Click selects: the inspector shows that node and the code view narrows to it; double click opens the code. |
| Sprite menu | Canvas Size (C: width, height, 3×3 anchor; pixels outside are dropped; one undo step, D-022) and Properties (Ctrl+P: fps, frames, loop; the three write as one `properties` undo step). |
| Inspector | The picked object's properties (or the scene's): numbers, paints and expressions as text fields, booleans as toggles; Enter writes the property. |
| Code | F2 cycles canvas / split / code. With an element selected, only that node's text is shown and edited (Esc returns to the whole file); writing back replaces exactly that node (D-021). The KDL is edited live: after 300 ms without typing it loads; while it doesn't, the errors show below and the last good scene stays. Picking an object moves the cursor to its node. Shift+arrows or a mouse drag select; Cmd/Ctrl+A, C, X and V select all, copy, cut and paste through the system clipboard. The title bar closes the pane or maximizes it to code only, and code only shows a live preview of the frame under the text. |
| Windows | Ctrl+Shift+N opens another canvas window on the same document with its own zoom, frame, active layer and hidden layers; edits show in every window. Any panel tears off into its own window (its title-bar button, or drag the title out) and docks back when that window closes. Side panels fold like an accordion (click the title or its fold button), the side columns resize by dragging the grip beside them, and the palette moves to the right column (its title-bar button or View → Palette on the right). The layout, column widths and folded panels are kept in `~/.config/px/editor.kdl`. |

Closing with unsaved changes keeps them in `<file>.unsaved.kdl` next to the file; untitled files go to `~/.config/px/unsaved/`.

## 1. Layout

> **Today (2026-11-01):** menu bar, tab strip, context bar (tool options), the color bar left, the tool bar right with Layers, Elements and Inspector beside it, the timeline at the bottom and the status bar (`W×H | zoom | x,y | color | frame | layer`) under it, as in Aseprite. Panels still tear off into their own windows.

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

> **Today (2026-11-01):** the ruler, a marker row (the Cue track's markers, without events) and read-only key rows, one per animated object with the keys of all its properties merged (`px_format::Timeline::keys` reports each animated property with its key times). Retiming, interpolation menus, loop regions, on2/on3 preview and the clip, state, camera-shot, sim and audio tracks are still planned.

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
| Export dialog | Ctrl/Cmd+E, Ctrl/Cmd+Alt+Shift+S (today) |
| Repeat last export | Ctrl/Cmd+Shift+X |
| New / Open / Save As / Close / Close all | Ctrl/Cmd+N / O / Shift+S / W / Shift+W |
| Next / previous tab / Home tab | Ctrl/Cmd+Tab / Shift+Tab / Home |
| Canvas size / Properties | C / Ctrl/Cmd+P |
| Edit palette | F4 |
| Command palette | Ctrl/Cmd+P |

---

## 7. Projects, scenes and sequences

- One window per project; open scenes as tabs.
- **Sequence view:** a strip of shots with transitions; click a shot to edit its scene and camera.
- Exposed params (`param`) are shown in a **Params** strip above the timeline for quick tweaking.
- Packs are listed in `project.kdl`; the project panel shows missing or outdated packs.

---

## 8. Export dialog

> **Today (2026-11-01):** GIF, PNG (current frame), PNG sequence and sprite sheet (strip, rows, columns; border and shape padding; scale; detail 1–8, which redraws the scene at that many pixels per scene pixel before scaling; transparent background; Aseprite JSON hash format). Clips, directions, tags, slices, trim, tilesets and parallax layers are still planned.

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
