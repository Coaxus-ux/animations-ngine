from chiptune import arpeggio, beat, bell, click, crash, impact, kick, pad, pulse, riser, snare, triangle

BAR = 2.0
EIGHTH = BAR / 8
FUJI_CHORDS = [[50, 54, 57], [47, 50, 54], [43, 47, 50], [45, 49, 52]]
FUJI_THEME = [(0, 74, 0.5), (0.5, 76, 0.5), (1.0, 78, 1.0), (2.0, 81, 0.75), (2.75, 78, 0.25), (3.0, 76, 1.0),
              (4.0, 74, 0.5), (4.5, 71, 0.5), (5.0, 69, 1.0), (6.0, 81, 0.5), (6.5, 78, 0.5), (7.0, 76, 1.0),
              (8.0, 78, 0.5), (8.5, 81, 0.75)]
FIELD_CHORDS = [[43, 47, 50], [40, 43, 47], [36, 40, 43], [38, 42, 45]]
FIELD_THEME = [(0.0, 71, 0.75), (0.75, 74, 0.25), (1.0, 76, 1.0), (2.0, 79, 0.5), (2.5, 78, 0.5), (3.0, 74, 0.5)]
DUSK_THEME = [(0.3, 76, 0.7), (1.0, 74, 0.5), (1.5, 71, 0.5), (2.0, 67, 1.2)]
PICKS = ["roots", "sword", "body", "hair"]
PICK_BELLS = [69, 72, 76, 74]


def bars_between(start, end):
    return max(0, int((end - start) // BAR))


def opening(mix, cues):
    gathered = cues.shot("logo-wide").start + float(cues.param("logo", "gather"))
    riser(mix, 0.0, gathered, 0.10)
    pad(mix, 0.0, [57, 60, 64], cues.shot("card-wide").start + 0.1, vol=0.035, attack=1.2, release=0.6)
    impact(mix, gathered, [57, 60, 64, 69])
    arpeggio(mix, gathered, cues.shot("in-editor-fuji").start - gathered, [69, 72, 76, 81], vol=0.045)
    bell(mix, gathered, 81, 1.6, 0.08)
    bell(mix, gathered + 0.8, 84, 1.4, 0.06, 0.3)
    card = cues.shot("card-wide")
    riser(mix, card.start + 0.4, card.end - card.start - 0.5, 0.10)
    triangle(mix, card.start, 45, card.duration - 0.5, 0.18)


def fuji(mix, cues):
    start = cues.shot("in-editor-fuji").start
    dive, full = cues.at("in-editor-fuji", "dive"), cues.at("in-editor-fuji", "full")
    impact(mix, start, [62, 66, 69, 74])
    bars = bars_between(start, dive)
    for bar in range(bars):
        chord = FUJI_CHORDS[bar % len(FUJI_CHORDS)]
        pad(mix, start + bar * BAR, [note + 12 for note in chord], BAR + 0.05, vol=0.03, attack=0.3, release=0.4)
        for eighth in range(8):
            triangle(mix, start + bar * BAR + eighth * EIGHTH, chord[0] - 12 + (12 if eighth % 4 == 2 else 0), 0.22, 0.2)
    beat(mix, start, bars, kicks=(0, 3, 4), snares=(4,), level=0.45)
    for at, note, length in FUJI_THEME:
        if start + at + length <= full + 1.2:
            pulse(mix, start + at, note, length * 0.95, vol=0.075, vibrato=0.004, pan=0.15)
    click(mix, cues.at("in-editor-fuji", "pick"))
    pad(mix, dive, [59, 62, 66], full - dive + 2.0, vol=0.04, attack=0.8, release=1.2, cutoff=1200)
    arpeggio(mix, dive, full - dive, [71, 74, 78, 83], vol=0.035)
    riser(mix, dive, full - dive, 0.08)
    crash(mix, full, 0.12)
    triangle(mix, full, 35, 2.0, 0.2, attack=0.05, release=0.6)
    for k, note in enumerate([83, 78, 74, 71, 74, 78, 86]):
        bell(mix, full + k * 0.28, note, 1.4, 0.06, (-0.4, 0.4)[k % 2])
    pad(mix, full + 1.0, [55, 59, 62, 66], 1.6, vol=0.035, attack=0.3, release=0.9, cutoff=1500)


def card_build(mix, cues, index, chord):
    card = cues.shot("card-wide", index)
    impact(mix, card.start, chord)
    riser(mix, card.start + 0.8, card.duration - 0.8, 0.1)
    for k in range(8):
        snare(mix, card.end - 1.0 + k * 0.125, 0.06 + k * 0.02)


def field(mix, cues):
    scene = "in-editor-sunflowers"
    start, load = cues.shot(scene).start, cues.at(scene, "load")
    dive, full = cues.at(scene, "dive"), cues.at(scene, "full")
    impact(mix, start, [55, 59, 62, 67])
    for bar in range(bars_between(start, dive) + 1):
        t = start + bar * BAR
        if t >= dive:
            break
        chord, dark = FIELD_CHORDS[bar % len(FIELD_CHORDS)], t >= load
        for eighth in range(8):
            triangle(mix, t + eighth * EIGHTH, chord[0] - 12 + (7 if eighth % 2 else 0) - (3 if dark and eighth % 2 else 0), 0.2, 0.2)
        if dark:
            pad(mix, t, [note + 12 for note in chord], BAR + 0.05, vol=0.035, attack=0.2, release=0.3, cutoff=900)
        else:
            arpeggio(mix, t, BAR, [note + 24 for note in chord], vol=0.03)
    beat(mix, start, bars_between(start, load), kicks=(0, 3, 4, 6), snares=(2, 6))
    for at, note, length in FIELD_THEME:
        pulse(mix, start + at, note, length * 0.9, vol=0.07, duty=0.125, pan=0.15)
    click(mix, cues.at(scene, "click"))
    typing, typed = cues.at(scene, "typing"), cues.at(scene, "typed")
    keys = round((typed - typing) * 18)
    for k in range(keys):
        pulse(mix, typing + k * (typed - typing) / keys, 84 + (k * 5) % 7, 0.022, vol=0.045, duty=0.5, attack=0.001, release=0.01, pan=0.35)
    kick(mix, load, 0.75)
    crash(mix, load, 0.1)
    pad(mix, load, [40, 47, 52, 55], dive - load + 0.1, vol=0.04, attack=0.05, release=1.0, cutoff=700)
    beat(mix, load, bars_between(load, dive), kicks=(0, 5), snares=(4,))
    for at, note, length in DUSK_THEME:
        pulse(mix, load + at, note, length * 0.9, vol=0.06, vibrato=0.006, pan=0.15)
    riser(mix, dive, full - dive, 0.09)
    crash(mix, full, 0.12)
    pad(mix, full, [43, 50, 55, 59, 62], 1.5, vol=0.035, attack=0.1, release=1.0, cutoff=2200)
    arpeggio(mix, full, cues.shot(scene).end - full - 0.75, [67, 71, 74, 79], vol=0.035)


def fallen(mix, cues):
    scene = "in-editor-fallen"
    start, play = cues.shot(scene).start, cues.at(scene, "play")
    dive, full = cues.at(scene, "dive"), cues.at(scene, "full")
    kick(mix, start, 0.7)
    crash(mix, start, 0.1)
    triangle(mix, start, 33, dive - start, 0.16, attack=0.4, release=1.0)
    pad(mix, start, [45, 52, 57, 60], dive - start, vol=0.035, attack=1.0, release=1.5, cutoff=800)
    beats = int((dive - play) // 1.6)
    for k in range(beats):
        kick(mix, play + k * 1.6, 0.45)
        kick(mix, play + k * 1.6 + 0.22, 0.32)
    click(mix, play)
    for k, (name, note) in enumerate(zip(PICKS, PICK_BELLS)):
        at = cues.at(scene, name)
        click(mix, at)
        bell(mix, at + 0.05, note, 1.4, 0.07, (-0.3, 0.3)[k % 2])
    pulse(mix, cues.at(scene, "roots") + 0.5, 64, 0.5, vol=0.04, duty=0.5, pan=-0.2)
    pulse(mix, cues.at(scene, "sword") + 0.5, 67, 0.5, vol=0.04, duty=0.5, pan=-0.2)
    riser(mix, dive, full - dive, 0.08)
    crash(mix, full, 0.1)
    pad(mix, full, [41, 48, 53, 57, 60], 2.1, vol=0.04, attack=0.2, release=1.2, cutoff=1400)
    triangle(mix, full, 29, 2.0, 0.18, attack=0.02, release=0.8)
    for k, note in enumerate([72, 69, 65, 64]):
        bell(mix, full + k * 0.4, note, 1.6, 0.05, (-0.4, 0.4)[k % 2])


def closing(mix, cues):
    start = cues.shot("logo-wide", 1).start
    gathered = start + float(cues.param("logo", "gather"))
    impact(mix, start, [48, 55, 60, 64])
    riser(mix, start + 0.2, gathered - start - 0.2, 0.09)
    kick(mix, gathered, 0.8)
    crash(mix, gathered, 0.18)
    rest = mix.length - gathered
    pad(mix, gathered, [48, 55, 60, 64, 67, 72], rest, vol=0.04, attack=0.05, release=1.6, cutoff=3200)
    triangle(mix, gathered, 36, rest, 0.22, attack=0.01, release=1.5)
    arpeggio(mix, gathered, 1.6, [72, 76, 79, 84], vol=0.04)
    for k, note in enumerate([84, 88, 91, 96]):
        bell(mix, gathered + k * 0.18, note, 1.8, 0.06, (-0.3, 0.3)[k % 2])
    pulse(mix, gathered + 0.8, 79, 0.4, vol=0.06, pan=0.15)
    pulse(mix, gathered + 1.2, 84, 1.3, vol=0.06, vibrato=0.008, release=0.9, pan=0.15)


def compose(mix, cues):
    opening(mix, cues)
    fuji(mix, cues)
    card_build(mix, cues, 1, [55, 59, 62, 67])
    field(mix, cues)
    card_build(mix, cues, 2, [57, 60, 64])
    fallen(mix, cues)
    closing(mix, cues)
