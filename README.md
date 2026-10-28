# animations-ngine

A general-purpose engine that creates pixel art scenes, characters, animations and game assets **entirely from code and data**: no drawing skills required and no AI inside the engine. Scenes are written in [KDL](https://kdl.dev), behaviors in TypeScript (planned), and everything renders through a palette-indexed Rust core.

> Status: **M1 done, with parts of M2–M5 pulled forward**. KDL scenes with expressions, keyframes, SDF shapes, form shading, outlines, textures, layers with parallax, clips, lights with cast shadows, fog, reflections, particles and ropes render to PNG stills and seamless looping GIFs, with a live preview. Scenes can span files and place other scenes (with their own position, scale, copies and local time) on a timeline, and the camera zooms smoothly by redrawing shapes at every size. What the loader accepts today: [FORMAT.md appendix A](docs/FORMAT.md#appendix-a-what-the-loader-accepts-today). See the [roadmap](docs/ROADMAP.md).

## Quick start

```bash
# Rust toolchain (pinned in rust-toolchain.toml)
brew install rustup && rustup default stable

cargo run -p px-cli -- export examples/hello.kdl            # → out/hello.gif
cargo run -p px-cli -- render examples/hello.kdl --frame 6  # → out/hello-6.png
cargo run -p px-cli -- watch examples/lantern-night.kdl     # live preview, reloads on save
cargo run -p px-cli -- bench                                # frame times for every example
```

## Development

```bash
tools/check.sh   # every check: size and commit guards, fmt, clippy, tests, golden hashes
```

- Engineering rules: [CLAUDE.md](CLAUDE.md), [docs/engineering/RUST-STANDARDS.md](docs/engineering/RUST-STANDARDS.md), [decisions](docs/engineering/DECISIONS.md).
- Design documentation: [docs/README.md](docs/README.md).
- No hosted CI: all checks run locally (D-006). Commits never carry AI agent signatures (D-005).
