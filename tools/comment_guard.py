#!/usr/bin/env python3
import re
import sys
from pathlib import Path

RUST_LINE = re.compile(r"^\s*//")
RUST_TRAILING = re.compile(r'^((?:[^"]|"[^"]*")*?)\s//[^/!]')
SCRIPT_LINE = re.compile(r"^\s*#")


def rust_offences(lines):
    inside_string = False
    for number, line in enumerate(lines, 1):
        code = line.replace('\\"', "").replace("'\"'", "")
        before = code.split("//", 1)[0] if "//" in code else code
        if not inside_string and RUST_LINE.match(line):
            yield number
        elif not inside_string and RUST_TRAILING.match(code) and before.count('"') % 2 == 0:
            yield number
        inside_string = (inside_string + code.count('"')) % 2 == 1


def script_offences(lines):
    for number, line in enumerate(lines, 1):
        if number == 1 and line.startswith("#!"):
            continue
        if SCRIPT_LINE.match(line):
            yield number


def main(paths):
    failed = False
    for path in map(Path, paths):
        if path.suffix == ".rs":
            offences = rust_offences
        elif path.suffix in {".py", ".sh"}:
            offences = script_offences
        else:
            continue
        try:
            lines = path.read_text().split("\n")
        except (OSError, UnicodeDecodeError):
            continue
        for number in offences(lines):
            print(f"{path}:{number}: comment; code explains itself (D-024)")
            failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
