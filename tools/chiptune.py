#!/usr/bin/env python3
import importlib.util
import math
import struct
import sys
import wave
from array import array
from pathlib import Path

import sequence_cues

RATE = 44100
PEAK = 0.89
SATURATION = 1.1
FADE_OUT = 0.6


def hz(midi):
    return 440.0 * 2 ** ((midi - 69) / 12)


def envelope(i, n, attack, release):
    t, length = i / RATE, n / RATE
    level = min(1.0, t / attack) if attack > 0 else 1.0
    if release > 0 and t > length - release:
        level = min(level, max(0.0, (length - t) / release))
    return level


class Mix:
    def __init__(self, seconds):
        self.length = seconds
        self.size = int(RATE * seconds)
        self.left = array("f", bytes(4 * self.size))
        self.right = array("f", bytes(4 * self.size))
        self.state = 12345

    def noise(self):
        self.state = (self.state * 1103515245 + 12345) & 0x7FFFFFFF
        return self.state / 0x3FFFFFFF - 1.0

    def put(self, start, samples, pan=0.0):
        first = int(start * RATE)
        left = math.cos((pan + 1) * math.pi / 4) * math.sqrt(2)
        right = math.sin((pan + 1) * math.pi / 4) * math.sqrt(2)
        for offset, value in enumerate(samples):
            index = first + offset
            if 0 <= index < self.size:
                self.left[index] += value * left
                self.right[index] += value * right

    def write(self, path):
        loudest = max(max(map(abs, self.left)), max(map(abs, self.right)), 1e-9)
        gain, fade = PEAK / loudest, int(FADE_OUT * RATE)
        squash = math.tanh(SATURATION)
        frames = bytearray()
        for index in range(self.size):
            level = gain * min(1.0, (self.size - index) / fade)
            for channel in (self.left, self.right):
                value = math.tanh(channel[index] * level * SATURATION) / squash
                frames += struct.pack("<h", int(value * 32000))
        with wave.open(str(path), "wb") as out:
            out.setnchannels(2)
            out.setsampwidth(2)
            out.setframerate(RATE)
            out.writeframes(bytes(frames))


def pulse(mix, start, midi, dur, vol=0.12, duty=0.25, attack=0.005, release=0.05, pan=0.0, vibrato=0.0):
    n, f, phase, out = int(dur * RATE), hz(midi), 0.0, []
    for i in range(n):
        wobble = 1 + vibrato * math.sin(2 * math.pi * 5.5 * i / RATE) * min(1, i / (0.25 * RATE))
        phase = (phase + f * wobble / RATE) % 1.0
        out.append((1.0 if phase < duty else -1.0) * vol * envelope(i, n, attack, release))
    mix.put(start, out, pan)


def triangle(mix, start, midi, dur, vol=0.22, attack=0.004, release=0.04, pan=0.0):
    n, f, phase, out = int(dur * RATE), hz(midi), 0.0, []
    for i in range(n):
        phase = (phase + f / RATE) % 1.0
        out.append((4 * abs(phase - 0.5) - 1) * vol * envelope(i, n, attack, release))
    mix.put(start, out, pan)


def pad(mix, start, notes, dur, vol=0.05, attack=0.6, release=0.8, cutoff=1800.0):
    n = int(dur * RATE)
    out = [0.0] * n
    smoothing = 1 - math.exp(-2 * math.pi * cutoff / RATE)
    for midi in notes:
        for detune in (-0.06, 0.0, 0.07):
            f, phase, level = hz(midi) * 2 ** (detune / 12), mix.noise() * 0.5 + 0.5, 0.0
            for i in range(n):
                phase = (phase + f / RATE) % 1.0
                level += smoothing * ((2 * phase - 1) - level)
                out[i] += level * vol * envelope(i, n, attack, release)
    mix.put(start, out)


def bell(mix, start, midi, dur=1.2, vol=0.1, pan=0.0):
    n, f, out = int(dur * RATE), hz(midi), []
    for i in range(n):
        t = i / RATE
        tone = math.sin(2 * math.pi * f * t) + 0.4 * math.sin(2 * math.pi * f * 2.76 * t) * math.exp(-t * 6)
        out.append(tone * vol * math.exp(-t * 3.2) * min(1, t / 0.003))
    mix.put(start, out, pan)


def kick(mix, start, vol=0.55):
    n, phase, out = int(0.28 * RATE), 0.0, []
    for i in range(n):
        t = i / RATE
        phase += (45 + 120 * math.exp(-t * 30)) / RATE
        out.append(math.sin(2 * math.pi * phase) * vol * math.exp(-t * 11))
    mix.put(start, out)


def snare(mix, start, vol=0.22):
    n = int(0.18 * RATE)
    out = [(mix.noise() * 0.8 + 0.4 * math.sin(2 * math.pi * 190 * i / RATE)) * vol * math.exp(-i / RATE * 22) for i in range(n)]
    mix.put(start, out, 0.05)


def hat(mix, start, vol=0.06):
    n, out, last = int(0.045 * RATE), [], 0.0
    for i in range(n):
        value = mix.noise()
        out.append((value - last) * vol * math.exp(-i / RATE * 70))
        last = value
    mix.put(start, out, 0.3)


def riser(mix, start, dur, vol=0.12):
    n, out, level = int(dur * RATE), [], 0.0
    for i in range(n):
        progress = i / n
        level += (1 - math.exp(-2 * math.pi * (300 + 7000 * progress**2) / RATE)) * (mix.noise() - level)
        out.append(level * vol * progress**2)
    mix.put(start, out)


def crash(mix, start, vol=0.16):
    n, out, last = int(1.6 * RATE), [], 0.0
    for i in range(n):
        value = mix.noise()
        out.append((value - 0.6 * last) * vol * math.exp(-i / RATE * 2.6))
        last = value
    mix.put(start, out)


def impact(mix, start, chord):
    kick(mix, start, 0.7)
    crash(mix, start)
    pad(mix, start, chord, 1.4, vol=0.04, attack=0.01, release=1.0, cutoff=3000)


def click(mix, start):
    pulse(mix, start, 96, 0.025, vol=0.07, duty=0.5, attack=0.001, release=0.01, pan=0.2)
    pulse(mix, start + 0.03, 103, 0.02, vol=0.05, duty=0.5, attack=0.001, release=0.01, pan=0.2)


def arpeggio(mix, start, dur, chord, step=0.125, vol=0.06):
    for k in range(int(round(dur / step))):
        pulse(mix, start + k * step, chord[k % len(chord)] + 12, step * 0.9, vol=vol, duty=0.125, attack=0.002, release=0.03, pan=-0.3)


def beat(mix, start, bars, kicks=(0, 4), snares=(2, 6), level=0.5):
    for bar in range(bars):
        for eighth in range(8):
            t = start + bar * 2 + eighth * 0.25
            if eighth in kicks:
                kick(mix, t, level)
            if eighth in snares:
                snare(mix, t)
            hat(mix, t, 0.05 if eighth % 2 else 0.07)


def load_score(path):
    spec = importlib.util.spec_from_file_location("score", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


if __name__ == "__main__":
    if len(sys.argv) != 4:
        sys.exit("usage: tools/chiptune.py <sequence.kdl> <score.py> <out.wav>")
    cues = sequence_cues.read(sys.argv[1])
    mix = Mix(cues.duration)
    load_score(Path(sys.argv[2])).compose(mix, cues)
    mix.write(sys.argv[3])
    print(f"wrote {sys.argv[3]} ({cues.duration:.2f} s)")
