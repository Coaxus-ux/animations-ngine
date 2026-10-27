#!/usr/bin/env python3
"""Rejects files that grow beyond the project's size limits.

One policy, three entry points (so the rules live in exactly one place):
  hook   Claude Code PreToolUse hook: reads the tool call JSON on stdin and
         blocks (exit 2) a Write/Edit/MultiEdit whose result is too large.
  check  Checks files on disk given as arguments (git pre-commit, tools/check.sh).
         Exits 1 when any file is over its limit.

A file that is already over its limit may still be edited as long as the edit
does not make it larger, so legacy files can be shrunk incrementally.
"""

from __future__ import annotations

import json
import os
import sys
from dataclasses import dataclass
from pathlib import Path

# --- Policy -----------------------------------------------------------------


@dataclass(frozen=True)
class Limit:
    kind: str
    max_lines: int | None
    max_bytes: int


CODE = Limit("code", max_lines=400, max_bytes=64 * 1024)
DOC = Limit("doc", max_lines=1000, max_bytes=128 * 1024)
DATA = Limit("data", max_lines=None, max_bytes=512 * 1024)
BINARY = Limit("binary", max_lines=None, max_bytes=1024 * 1024)

CODE_SUFFIXES = {".rs", ".ts", ".js", ".py", ".sh", ".kdl", ".toml", ".yml", ".yaml"}
DOC_SUFFIXES = {".md", ".txt"}
DATA_SUFFIXES = {".json", ".csv", ".gpl", ".hex"}
EXEMPT_NAMES = {"Cargo.lock"}

SPLIT_HINT = (
    "Split it into smaller, single-responsibility modules "
    "(see docs/engineering/RUST-STANDARDS.md §0 and §7 and CLAUDE.md)."
)


def limit_for(path: Path) -> Limit | None:
    if path.name in EXEMPT_NAMES:
        return None
    suffix = path.suffix.lower()
    if suffix in CODE_SUFFIXES or path.parent.name == ".githooks":
        return CODE
    if suffix in DOC_SUFFIXES:
        return DOC
    if suffix in DATA_SUFFIXES:
        return DATA
    return BINARY


# --- Measurement --------------------------------------------------------------


@dataclass(frozen=True)
class Size:
    lines: int
    bytes: int


def measure(data: bytes) -> Size:
    lines = data.count(b"\n") + (1 if data and not data.endswith(b"\n") else 0)
    return Size(lines=lines, bytes=len(data))


def violations(limit: Limit, size: Size) -> list[str]:
    found = []
    if limit.max_lines is not None and size.lines > limit.max_lines:
        found.append(f"{size.lines} lines > {limit.max_lines} allowed for {limit.kind} files")
    if size.bytes > limit.max_bytes:
        found.append(f"{size.bytes} bytes > {limit.max_bytes} allowed for {limit.kind} files")
    return found


def grew(before: Size | None, after: Size) -> bool:
    return before is None or after.lines > before.lines or after.bytes > before.bytes


# --- Hook mode ----------------------------------------------------------------


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except (FileNotFoundError, UnicodeDecodeError):
        return ""


def apply_edit(text: str, old: str, new: str, replace_all: bool) -> str:
    return text.replace(old, new) if replace_all else text.replace(old, new, 1)


def resulting_text(tool: str, tool_input: dict, current: str) -> str | None:
    if tool == "Write":
        return tool_input.get("content", "")
    if tool == "Edit":
        return apply_edit(
            current,
            tool_input.get("old_string", ""),
            tool_input.get("new_string", ""),
            bool(tool_input.get("replace_all", False)),
        )
    if tool == "MultiEdit":
        text = current
        for edit in tool_input.get("edits", []):
            text = apply_edit(
                text,
                edit.get("old_string", ""),
                edit.get("new_string", ""),
                bool(edit.get("replace_all", False)),
            )
        return text
    return None


def inside_project(path: Path) -> bool:
    root = os.environ.get("CLAUDE_PROJECT_DIR")
    if not root:
        return True
    try:
        path.resolve().relative_to(Path(root).resolve())
    except ValueError:
        return False
    return True


def run_hook() -> int:
    payload = json.load(sys.stdin)
    tool_input = payload.get("tool_input", {})
    raw_path = tool_input.get("file_path")
    if not raw_path:
        return 0
    path = Path(raw_path)
    limit = limit_for(path)
    if limit is None or not inside_project(path):
        return 0

    current = read_text(path)
    result = resulting_text(payload.get("tool_name", ""), tool_input, current)
    if result is None:
        return 0

    after = measure(result.encode("utf-8"))
    before = measure(current.encode("utf-8")) if path.exists() else None
    problems = violations(limit, after)
    if problems and grew(before, after):
        print(f"Blocked: {path} would be too large: {'; '.join(problems)}. {SPLIT_HINT}", file=sys.stderr)
        return 2
    return 0


# --- Check mode ---------------------------------------------------------------


def run_check(paths: list[str]) -> int:
    failed = False
    for raw in paths:
        path = Path(raw)
        limit = limit_for(path)
        if limit is None or not path.is_file():
            continue
        problems = violations(limit, measure(path.read_bytes()))
        for problem in problems:
            print(f"{path}: {problem}", file=sys.stderr)
            failed = True
    if failed:
        print(SPLIT_HINT, file=sys.stderr)
    return 1 if failed else 0


def main(argv: list[str]) -> int:
    if len(argv) >= 2 and argv[1] == "hook":
        return run_hook()
    if len(argv) >= 2 and argv[1] == "check":
        return run_check(argv[2:])
    print("usage: file_size_guard.py hook | check <paths...>", file=sys.stderr)
    return 64


if __name__ == "__main__":
    sys.exit(main(sys.argv))
