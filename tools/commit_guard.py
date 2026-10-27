#!/usr/bin/env python3
"""Rejects commits that carry an AI agent's signature or attribution.

Project rule (docs/engineering/DECISIONS.md D-005): commits are authored by people.
No "Co-Authored-By: Claude", no "Generated with …" footers, no agent author or
committer identities, for any agent.

Entry points (the policy lives only here):
  msg <file>       git commit-msg hook: checks the message file and the pending
                   author/committer identities (`git var`).
  history [range]  CI: checks every commit in a revision range (default: all of HEAD).
"""

from __future__ import annotations

import re
import subprocess
import sys

# Coding agents and assistants, matched as whole words, case-insensitive.
# Unambiguous names are never people's names or ordinary words.
UNAMBIGUOUS = (
    "claude", "anthropic", "openai", "chatgpt", r"gpt-\d", "copilot", "codex", "gemini",
    "aider", "codeium", "windsurf", "tabnine", "codewhisperer", "amazon q", "ai assistant",
    "ai agent",
)
# Ambiguous names are also people's names or common words ("cursor position", "Devin").
AMBIGUOUS = ("cursor", "devin", "jules", "cody")
# Signs that an ambiguous name refers to an agent.
AI_CONTEXT = r"(\bai\b|\bagent\b|\bbot\b|\[bot\]|\.ai\b|cognition)"

# Trailers that can name people: ambiguous names count only with AI context.
PERSON_KEYS = (
    "co-authored-by", "signed-off-by", "authored-by", "created-by", "written-by", "helped-by",
)
# Trailers that name tools: any agent name counts.
TOOL_KEYS = ("assisted-by", "generated-by", "made-with", "ai-assisted", "ai-model", "model", "tool")


def words(names: tuple[str, ...]) -> str:
    return r"\b(" + "|".join(names) + r")\b"


CLEAR_AGENT = re.compile(words(UNAMBIGUOUS), re.IGNORECASE)
ANY_AGENT = re.compile(words(UNAMBIGUOUS + AMBIGUOUS), re.IGNORECASE)
CONTEXT = re.compile(AI_CONTEXT, re.IGNORECASE)
IDENTITY = re.compile(words(UNAMBIGUOUS) + r"|\[bot\]", re.IGNORECASE)
TRAILER = re.compile(r"^\s*([A-Za-z][\w-]*)\s*:\s*(.*)$")
MARKERS = (
    re.compile(r"generated (with|by|using)\b.*" + words(UNAMBIGUOUS + AMBIGUOUS), re.IGNORECASE),
    re.compile(r"noreply@anthropic\.com|claude\.(ai|com)/|\[bot\]@", re.IGNORECASE),
    re.compile("\N{ROBOT FACE}"),
)

MAX_COMMITS = 10_000


def is_attribution(line: str) -> bool:
    if any(marker.search(line) for marker in MARKERS):
        return True
    trailer = TRAILER.match(line)
    if trailer is None:
        return False
    key, value = trailer.group(1).lower(), trailer.group(2)
    if CLEAR_AGENT.search(key.replace("-", " ")):  # e.g. "Claude-Session:"
        return True
    if key in TOOL_KEYS:
        return ANY_AGENT.search(value) is not None
    if key in PERSON_KEYS:
        clear = CLEAR_AGENT.search(value) is not None
        return clear or (ANY_AGENT.search(value) is not None and CONTEXT.search(value) is not None)
    return False


def message_problems(message: str) -> list[str]:
    problems = []
    for number, line in enumerate(message.splitlines(), start=1):
        if line.lstrip().startswith("#"):  # git comment lines are stripped from the message
            continue
        if is_attribution(line):
            problems.append(f"line {number}: agent attribution: {line.strip()!r}")
    return problems


def identity_problems(label: str, identity: str) -> list[str]:
    if IDENTITY.search(identity):
        return [f"{label} looks like an agent: {identity.strip()!r}"]
    return []


def git(*args: str) -> str:
    return subprocess.run(["git", *args], check=True, capture_output=True, text=True).stdout


def report(problems: list[str], where: str) -> int:
    if not problems:
        return 0
    print(f"Commit rejected ({where}): commits must not carry AI agent signatures.", file=sys.stderr)
    for problem in problems:
        print(f"  - {problem}", file=sys.stderr)
    print("Remove the attribution and commit again (docs/engineering/DECISIONS.md D-005).", file=sys.stderr)
    return 1


def run_msg(path: str) -> int:
    with open(path, encoding="utf-8") as file:
        problems = message_problems(file.read())
    for label, var in (("author", "GIT_AUTHOR_IDENT"), ("committer", "GIT_COMMITTER_IDENT")):
        problems += identity_problems(label, git("var", var).rsplit(">", 1)[0] + ">")
    return report(problems, "commit-msg")


def run_history(revision_range: str) -> int:
    log = git("log", f"--max-count={MAX_COMMITS}", "--format=%H%x1f%an <%ae>%x1f%cn <%ce>%x1f%B%x1e", revision_range)
    failed = 0
    for record in filter(str.strip, log.split("\x1e")):
        sha, author, committer, message = record.strip().split("\x1f", 3)
        problems = message_problems(message)
        problems += identity_problems("author", author) + identity_problems("committer", committer)
        failed |= report(problems, sha[:12])
    return failed


def main(argv: list[str]) -> int:
    if len(argv) == 3 and argv[1] == "msg":
        return run_msg(argv[2])
    if len(argv) in (2, 3) and argv[1] == "history":
        return run_history(argv[2] if len(argv) == 3 else "HEAD")
    print("usage: commit_guard.py msg <file> | history [range]", file=sys.stderr)
    return 64


if __name__ == "__main__":
    sys.exit(main(sys.argv))
