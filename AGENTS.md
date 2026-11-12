# Instructions for coding agents

This file is the entry point for any AI coding agent working in this repository. It points to the sources of truth instead of repeating them.

## Read first

| Read | For |
|---|---|
| [CLAUDE.md](CLAUDE.md) | The always-on rules, the crate layout and the commands. They apply to every agent, whatever its name |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How a change is made, checked and delivered, with step-by-step recipes |
| [docs/engineering/RUST-STANDARDS.md](docs/engineering/RUST-STANDARDS.md) | The Rust standard |
| [docs/FORMAT.md appendix A](docs/FORMAT.md#appendix-a-what-the-loader-accepts-today) | What the KDL loader accepts today |

## Skills

A skill is a task guide in `.claude/skills/<name>/SKILL.md`: plain Markdown with a short header. Agents that support skills load them automatically; any other agent should open the file and follow it when the task matches.

| Task | Skill |
|---|---|
| Create or change an animation, scene, title card or sequence; export a GIF, PNG or MP4; add an example | [pxe-animation](.claude/skills/pxe-animation/SKILL.md) |

## Non-negotiable

- **No agent attribution.** No `Co-Authored-By` trailers for an agent, no "Generated with" footers, no agent identities in commits or pull requests (D-005). `tools/commit_guard.py` rejects them.
- **No comments in code** (D-024), small files (D-001), English everywhere.
- **No hosted CI or paid services** (D-006). `tools/check.sh` is the only gate; run it before reporting a change as done.
- **Look at what you render.** Open the frames you produce before saying a scene works.
- **Licence.** The project is source-available under the PolyForm Noncommercial License ([LICENSE.md](LICENSE.md), D-032). Do not add code, assets or dependencies whose licence forbids commercial distribution.
- **Commit and push only when asked.**
