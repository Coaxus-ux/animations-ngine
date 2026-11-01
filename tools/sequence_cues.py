#!/usr/bin/env python3
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

USE = re.compile(r'^\s*use\s+"([^"]+)"', re.MULTILINE)
BLOCK = re.compile(r'^(scene|sequence)\s+"([^"]+)"([^{\n]*)\{', re.MULTILINE)
MARKER = re.compile(r'marker\s+"([^"]+)"\s+at="([^"]+)"')
PARAM = re.compile(r'^\s*param\s+([\w-]+)\s+("[^"]*"|\S+)', re.MULTILINE)
NODE = re.compile(r'^\s*(shot|transition)\s+"([^"]+)"(.*)$', re.MULTILINE)
PROP = re.compile(r'([\w-]+)=("[^"]*"|\S+)')


def seconds(text, fps):
    value = text.strip('"')
    for suffix, scale in (("ms", 0.001), ("s", 1.0), ("f", 1.0 / fps)):
        if value.endswith(suffix):
            return float(value[: -len(suffix)]) * scale
    raise ValueError(f"not a time: {text}")


def props(text):
    return {name: value.strip('"') for name, value in PROP.findall(text)}


def body(text, open_at):
    depth, inside = 0, False
    for index in range(open_at, len(text)):
        char = text[index]
        if char == '"' and text[index - 1] != "\\":
            inside = not inside
        elif not inside and char == "{":
            depth += 1
        elif not inside and char == "}":
            depth -= 1
            if depth == 0:
                return text[open_at + 1 : index]
    raise ValueError("unbalanced braces")


@dataclass
class Scene:
    fps: int
    markers: dict = field(default_factory=dict)
    params: dict = field(default_factory=dict)

    def marker(self, name):
        return self.markers[name]


@dataclass
class Shot:
    scene: str
    start: float
    duration: float
    props: dict

    @property
    def end(self):
        return self.start + self.duration


@dataclass
class Cues:
    shots: list
    scenes: dict

    @property
    def duration(self):
        return max(shot.end for shot in self.shots)

    @property
    def fps(self):
        return self.scenes[self.shots[0].scene].fps

    @property
    def frames(self):
        return round(self.duration * self.fps)

    def shot(self, scene, index=0):
        return [shot for shot in self.shots if shot.scene == scene][index]

    def at(self, scene, marker, index=0):
        return self.shot(scene, index).start + self.scenes[scene].marker(marker)

    def param(self, scene, name):
        return self.scenes[scene].params[name]


def files(root):
    seen, queue = [], [root.resolve()]
    while queue:
        path = queue.pop(0)
        if path in seen:
            continue
        seen.append(path)
        queue.extend((path.parent / used).resolve() for used in USE.findall(path.read_text()))
    return seen


def read(root):
    scenes, sequence = {}, None
    for path in files(Path(root)):
        text = path.read_text()
        for match in BLOCK.finditer(text):
            kind, name, header = match.groups()
            inner = body(text, match.end() - 1)
            if kind == "sequence":
                sequence = sequence or inner
                continue
            fps = int(props(header).get("fps", "12"))
            scenes[name] = Scene(
                fps=fps,
                markers={marker: seconds(at, fps) for marker, at in MARKER.findall(inner)},
                params={param: value.strip('"') for param, value in PARAM.findall(inner)},
            )
    if sequence is None:
        raise ValueError(f"{root} has no sequence")
    return Cues(shots=shots(sequence, scenes), scenes=scenes)


def shots(sequence, scenes):
    found, start, overlap = [], 0.0, 0.0
    for kind, name, rest in NODE.findall(sequence):
        given = props(rest)
        if kind == "transition":
            overlap = seconds(given["duration"], scenes[found[-1].scene].fps) if "duration" in given else 0.0
            continue
        start = found[-1].end - overlap if found else 0.0
        duration = seconds(given["duration"], scenes[name].fps)
        found.append(Shot(scene=name, start=start, duration=duration, props=given))
        overlap = 0.0
    return found


if __name__ == "__main__":
    cues = read(sys.argv[1])
    if sys.argv[2:] == ["--frames"]:
        print(cues.frames)
    else:
        for shot in cues.shots:
            print(f"{shot.start:7.2f}s  {shot.scene}")
