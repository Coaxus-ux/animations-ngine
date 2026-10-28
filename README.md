# animations-ngine

A general-purpose engine that creates pixel art scenes, characters, animations and game assets **entirely from code and data**: no drawing skills required and no AI inside the engine. Scenes are written in [KDL](https://kdl.dev), behaviors in TypeScript (planned), and everything renders through a palette-indexed Rust core.

> Status: **M0 walking skeleton**. A KDL scene with shapes and expressions renders to PNG stills and seamless looping GIFs. See the [roadmap](docs/ROADMAP.md).

## Quick start

```bash
# Rust toolchain (pinned in rust-toolchain.toml)
brew install rustup && rustup default stable

cargo run -p px-cli -- export examples/hello.kdl            # → out/hello.gif
cargo run -p px-cli -- render examples/hello.kdl --frame 6  # → out/hello-6.png
cargo run -p px-cli -- watch examples/sunset-walk.kdl       # live preview, reloads on save
```

## Development

```bash
tools/check.sh   # every check: size and commit guards, fmt, clippy, tests, golden hashes
```

- Engineering rules: [CLAUDE.md](CLAUDE.md), [docs/engineering/RUST-STANDARDS.md](docs/engineering/RUST-STANDARDS.md), [decisions](docs/engineering/DECISIONS.md).
- Design documentation: [docs/README.md](docs/README.md).
- No hosted CI: all checks run locally (D-006). Commits never carry AI agent signatures (D-005).
