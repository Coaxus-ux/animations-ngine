# Command Line (draft v1)

> Companion to [SPEC.md](SPEC.md). The editor's export dialog can copy any export as a CLI command ([EDITOR.md](EDITOR.md) §8).

The binary's working name is **`pxe`** (to be renamed with the project). All commands are deterministic: the same project, seed and engine version always produce the same output.

---

## 1. Commands

| Command | Purpose |
|---|---|
| `pxe new <name>` | Create a project |
| `pxe preview [scene]` | Open the editor (or a minimal preview window with `--lite`) |
| `pxe render <scene>` | Render frames to PNG (debugging, stills) |
| `pxe edit <file>` | The pixel-art editor: paint layers, animate cels, edit the scene visually or as code ([EDITOR.md](EDITOR.md) §0) |
| `pxe export <target>` | Produce final outputs: animations, sprite sheets, tilesets, layers |
| `pxe watch <file>` | Live preview window that reloads whenever the scene file changes |
| `pxe check` | Validate the whole project without rendering |
| `pxe types` | Regenerate TypeScript declarations in `.engine/types/` |
| `pxe bake <scene> <node>` | Bake a node to explicit KDL ([EDITOR.md](EDITOR.md) §3.1) |
| `pxe pack <sub>` | Manage packs |
| `pxe palette import <file>` | Import a palette from Lospec `.hex`/`.gpl` |
| `pxe bench` | Time benchmark scenes and measure how they scale (today); golden-image tests (planned) |
| `pxe timeline <file>` | Print the markers and placed scenes of a scene on a time ruler |
| `pxe migrate` | Upgrade project files to the current format version, preserving formatting ([ARCHITECTURE.md](ARCHITECTURE.md)) |

---

## 2. Global flags

| Flag | Meaning |
|---|---|
| `--project <dir>` | Project root (default: search upward for `project.kdl`) |
| `--json` | Machine-readable output (diagnostics, results) |
| `-q`, `--quiet` / `-v`, `--verbose` | Output level |
| `--threads <n>` | Worker threads (default: all cores) |
| `--no-color` | Plain terminal output |

---

## 3. Command reference

### 3.1 `new`
```bash
pxe new rain-street --template urban
```
| Flag | Default | Meaning |
|---|---|---|
| `--template` | `empty` | `empty`, `interior`, `nature`, `urban`, `water`, `character`, `isometric` |
| `--resolution` | `256x256` | Canvas size |
| `--fps` | `12` | |
| `--palette` | template's | Default palette |

It creates `project.kdl`, the standard folders, a sample scene, `tsconfig.json` and the generated types.

### 3.2 `render`
```bash
pxe render forest-dusk --frame 48 --scale 4 --out out/frame48.png
pxe render night-room --range 0..23 --debug light
```
| Flag | Meaning |
|---|---|
| `--frame <n>` / `--time <t>` | One frame (`--time 2.5s` works too) |
| `--range <a..b>` | A range of frames as numbered PNGs |
| `--scale <n>` | Integer upscale |
| `--seed <n>` | Override the scene seed |
| `--palette <name>` | Override the palette |
| `--param <name=value>` | Override an exposed param (repeatable) |
| `--debug <view>` | `material`, `tone`, `light`, `depth`, `emission`, `id`, `rig`, `sockets`, `cleanup` |
| `--out <path>` | File or folder |

### 3.3 `export`
`<target>` is a scene, a sequence, a character id (`scene:id`), a tileset generator, or a preset name from `project.kdl`.
```bash
pxe export rain-short --format mp4 --scale 6
pxe export night-room --format gif --loop auto --variations 4
pxe export hero-sheet:hero --format sheet --clips idle,walk,run,jump,slash --directions 8
pxe export village-tiles --format tileset --autotile 47
pxe export forest-dusk --format layers
pxe export --preset release-gifs
```
| Flag | Meaning |
|---|---|
| `--format` | `gif`, `mp4`, `webm`, `apng`, `webp`, `png`, `sheet`, `tileset`, `layers` |
| `--scale <n>` | Integer, nearest-neighbor |
| `--loop auto\|off\|<duration>` | Auto uses the loop solver ([DIRECTOR.md](DIRECTOR.md)) |
| `--range <a..b>` | Subrange in frames or time |
| `--variations <n>` | N outputs with successive seeds |
| `--palettes <a,b,…>` | One output per palette |
| `--clips <list>` | Sprite sheet: clips to include |
| `--directions 1\|4\|8` | Sprite sheet: facings |
| `--layout rows\|cols\|packed` | Sprite sheet layout |
| `--padding <px>` | Padding between cells |
| `--meta json\|none` | Metadata file (frames, durations, pivots, anchors, parallax factors) |
| `--autotile 16\|47` | Tileset rule set |
| `--preset <name>` | Use an `export` node from `project.kdl` |
| `--out <path>` | Output file or folder (default: `output/`) |

MP4/WebM need `ffmpeg` (see §5).

> **Today (2026-10-30):** `pxe export <file> [--scene NAME] [--scale N] [--param NAME=VALUE] [--out PATH] [--jobs N]` writes a GIF of a scene or a sequence; a sequence's GIF gives each frame its own color table, so shots in different palettes keep their exact colors. Frames are rendered and compressed on `--jobs` threads (default: every core) and stored as deltas, only the box that changed since the previous frame; the file is byte-identical for any `--jobs` (D-010). `cargo pxe …` runs the optimized CLI from the repository. `pxe render <file> --frame N` writes one PNG of either.

### 3.4 `watch`
```bash
pxe watch examples/sunset-walk.kdl --scale 5 --param stride=6
```
Opens a window that plays the scene at its fps and reloads within a quarter second of the file, or any file it `use`s, being saved. If the file has errors, they are printed and the last valid version keeps playing. The title shows the frame, fps and average render time per frame.

| Key | Action |
|---|---|
| Space | Pause / resume |
| ← / → | Step one frame (while paused) |
| R / Home | Restart from frame 0 |
| Esc / Q | Quit |

Flags: `--scene`, `--scale`, `--param` (as in `export`). Planned: also reloading on changes to packs and scripts the scene uses, and `--export` to re-export on every change.

### 3.5 `check`
```bash
pxe check
pxe check scenes/forest.kdl --json
```
Validates KDL syntax, prefab schemas, references (`@id`, `$param`, sockets), palettes (tier 1 roles, twins), pieces, body plans and scripts (load, sandbox rules, declared events). With `--tsc`, it also runs a TypeScript type check if `tsc` is installed.

### 3.6 `types`
Regenerates `.engine/types/`. Runs automatically in `preview`, `watch` and `check`.

### 3.7 `bake`
```bash
pxe bake night-room "layer room/shelf[0]" --what repeat
```
`--what repeat|ranges|sim|pose|all`. Writes into the scene file, keeping the original node disabled with `/-`.

### 3.8 `pack`
| Subcommand | Meaning |
|---|---|
| `pack list` | Loaded packs, versions and sources |
| `pack new <name>` | Create a pack folder (prefabs, body plans, pieces, palettes, scripts) |
| `pack add <path\|git-url>` | Add a pack dependency to `project.kdl` |
| `pack check <name>` | Validate a pack and render its prefab thumbnails in every supported view |
| `pack fork <pack>/<prefab>` | Copy a pack prefab into the project's `prefabs/` |

### 3.9 `palette import`
```bash
pxe palette import ~/Downloads/resurrect-64.hex --name resurrect
```
Clusters colors into ramps by hue and lightness and writes `palettes/<name>.kdl` with a TODO list of roles to alias. `--ramps <n>` hints how many ramps to build.

### 3.10 `bench`
Golden-image tests for the benchmark scenes ([BENCHMARKS.md](BENCHMARKS.md)) and any scene with a `golden` node.
```bash
pxe bench                 # render and compare all
pxe bench B3 --update     # accept new output as golden
```

> **Today (2026-10-28):** `pxe bench [FILES…] [--budget-ms 33.3] [--rounds 3] [--sweep NAME=V1,V2,…]` times every frame of `examples/*.kdl` and `benchmarks/*/*.kdl` (or the given files; a sequence is timed frame by frame with its transitions and reports its largest shot's counts), prints an aligned table of each scene's size, frame count, shapes, placed scenes (`inst`) and lights with the mean, 95th-percentile and worst frame time, the sustained fps and the worst frame as a share of the budget, then a summary line (scenes, frames, total frame time, slowest frame, how many are over budget), and exits with code 4 when a worst frame is over budget. `--sweep` renders each scene once per value of a scene param and reports the first value over budget instead of failing, e.g. `pxe bench examples/town-festival.kdl --sweep crowd=10,100,300,1000`. `tools/check.sh` runs it in release mode. Golden comparison runs as indexed-buffer hashes in `cargo test` (`crates/px-cli/tests/golden.rs`); the flags below are not implemented yet.
| Flag | Meaning |
|---|---|
| `--update` | Overwrite golden images |
| `--frames <list>` | Frames to compare (default: those listed in the scene's `golden` node) |
| `--threshold <n>` | Allowed differing pixels (default 0, since rendering is deterministic) |
| `--report <dir>` | Write diff images and an HTML summary |

### 3.11 `timeline`
```bash
pxe timeline examples/town-festival.kdl
```
Prints the scene's duration, a ruler, its `timeline` markers and one row per `use-scene`, nested ones indented. A bar shows when the placed scene is visible (`=`) or hidden (`.`), then whether it loops, its own duration, `speed` and `offset`. `count` copies share one row (`×n`). Flags: `--scene`, `--param`.

---

## 4. Exit codes

| Code | Meaning |
|---|---|
| 0 | Success (warnings allowed) |
| 1 | Content errors (invalid KDL, schema, palette, piece, missing reference) |
| 2 | Usage error (bad command or flag) |
| 3 | I/O or export failure (cannot write, `ffmpeg` missing or failed) |
| 4 | Bench mismatch |
| 5 | Script runtime error or timeout |

`--json` always prints diagnostics as an array of `{ level, code, message, file, line, column, node }`.

---

## 5. Configuration resolution

From highest to lowest priority:
1. Command-line flags
2. Environment variables (`PXE_THREADS`, `PXE_FFMPEG`, `PXE_PACKS`)
3. `project.kdl`
4. User config: `~/.config/pxe/config.kdl` (macOS/Linux) or `%APPDATA%\pxe\config.kdl` (Windows)
5. Built-in defaults

**Project discovery:** `--project`, otherwise the nearest `project.kdl` walking up from the current directory.

**Pack resolution** (first match wins):
1. The project's `prefabs/` (local prefabs and forks)
2. The project's `packs/`
3. User packs folder (`PXE_PACKS` or `packs-dir` in the user config)
4. Standard packs bundled with the engine

**ffmpeg:** `PXE_FFMPEG`, then `ffmpeg-path` in configs, then `PATH`.
