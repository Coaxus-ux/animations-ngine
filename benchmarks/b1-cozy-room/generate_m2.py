"""Writes the B1 re-attempt with M2/M4 features: lights, twins, shading, outlines, texture,
clips, ropes, particles. Still generated because the engine has no `repeat` yet.

Usage: python3 benchmarks/b1-cozy-room/generate_m2.py benchmarks/b1-cozy-room/m2-attempt.kdl
"""
import random
import sys

random.seed(4)
out = []

RAMPS = {
    "ink": ["#0a141c"],
    "wall": ["#16242e", "#1b2d39", "#22394a", "#2a4a5c", "#31566b", "#3b6378"],
    "floor": ["#0f1a21", "#132029", "#1a2c38", "#223a47", "#2b4855"],
    "wood": ["#0a1218", "#0e1820", "#15232e", "#1e3140", "#284152"],
    "book-navy": ["#121f36", "#1a2e4e", "#25406a", "#355886"],
    "book-teal": ["#132f31", "#1b4446", "#255c5e", "#357675"],
    "book-green": ["#18271f", "#22382c", "#2f4f3c", "#40664d"],
    "book-plum": ["#241c2d", "#33283f", "#463656", "#5d4770"],
    "glass": ["#2c6585", "#3f8db5", "#5fb0d4", "#86d0ea", "#b4e6f4"],
    "skin": ["#7a5040", "#a8735c", "#d49a7c", "#ecc0a0"],
    "hair": ["#0b1016", "#121820", "#202b36", "#2e3c4a"],
    "curls": ["#7a5232", "#a8754a", "#d4a36a", "#ecc68a"],
    "sweater": ["#1a2b1f", "#253c2c", "#33513b", "#456a4d"],
    "pink": ["#4a283f", "#6e3d63", "#9a5a8a", "#c088b0"],
    "jeans": ["#111a28", "#1a2638", "#263750", "#344a68"],
    "tv": ["#0d1720", "#14222e", "#1f3446", "#2d4c66", "#3f6a8c"],
}
GLOWS = {
    "screen": ["#123a24", "#2f8a4a", "#8fe39a"],
    "blue-screen": ["#2a6fb0", "#7cc4f2"],
    "bulb": ["#b86f55", "#e6a07a", "#ffd2a8", "#fff1dc"],
    "heater": ["#5a1e22", "#c0443e", "#ff9a7a"],
}
BOOKS = ["book-navy", "book-teal", "book-green", "book-plum"]


def warm(color, amount=0.42):
    """The color under a warm lamp: pulled toward amber and slightly brighter."""
    channels = [int(color[i:i + 2], 16) for i in (1, 3, 5)]
    target = (255, 176, 112)
    mixed = [round(c + (t - c) * amount * (0.4 + c / 255)) for c, t in zip(channels, target)]
    return "#" + "".join(f"{min(255, c):02x}" for c in mixed)


def emit(text):
    out.append(text)


def node(kind, ident=None, **props):
    name = f' "{ident}"' if ident else ""
    body = " ".join(f"{key.replace('_', '-')}={value}" for key, value in props.items())
    emit(f"    {kind}{name} {body}")


def text(value):
    return f'"{value}"'


def rect(x, y, w, h, paint, ident=None, **style):
    node("rect", ident, x=x, y=y, width=w, height=h, paint=text(paint), **style)


def ellipse(x, y, w, h, paint, ident=None, **style):
    node("ellipse", ident, x=x, y=y, width=w, height=h, paint=text(paint), **style)


def book_row(x0, x1, base, low, high, gap_chance=0.12):
    x = x0
    while x < x1 - 1:
        w = random.choice([2, 2, 3, 3, 4])
        if x + w > x1:
            break
        if random.random() < gap_chance:
            x += random.choice([2, 3])
            continue
        paint = f'{random.choice(BOOKS)}.{random.choice(["shade", "base", "lit"])}'
        node("rect", x=x, y=text(f"=({base}) - @self.height"), width=w,
             height=text(f"{low}..{high}"), paint=text(paint), casts_shadow="#true")
        x += w


def stack(x, base, count, wmin, wmax):
    y = base
    for _ in range(count):
        w = random.randint(wmin, wmax)
        paint = f'{random.choice(BOOKS)}.{random.choice(["shade", "base", "lit"])}'
        rect(x + random.randint(-1, 1), y - 2, w, 2, paint, casts_shadow="#true")
        y -= 2


def palette():
    emit('palette "night-room" {')
    for name, colors in RAMPS.items():
        emit(f'    ramp "{name}" ' + " ".join(f'"{c}"' for c in colors))
        emit(f'    ramp "{name}-warm" ' + " ".join(f'"{warm(c)}"' for c in colors))
    for name, colors in GLOWS.items():
        emit(f'    ramp "{name}" ' + " ".join(f'"{c}"' for c in colors))
    for name in RAMPS:
        emit(f'    twin "{name}" warm="{name}-warm"')
    emit("}\n")


def room():
    emit('''    ambient darkness=1 occlusion=1 key-light=200

    // Ceiling, walls and floor, with a little plaster texture
    rect x=0 y=0 width="100%" height=26 paint="wall.shade" gradient="down" gradient-steps=-1
    rect x=0 y=26 width=63 height=140 paint="wall.base" texture=0.12 texture-scale=5
    rect x=63 y=26 width=129 height=140 paint="wall.lit" texture=0.12 texture-scale=5
    rect x=0 y=166 width="100%" height=26 paint="floor.base" gradient="down" gradient-steps=-1
    line x1=0 y1=166 x2=191 y2=166 paint="floor.lit"
    line x1=63 y1=26 x2=63 y2=165 paint="wall.shade"''')


def window():
    emit("\n    // Window: a rainy city clipped to the glass")
    rect(95, 59, 54, 67, "wood.base", outline=text("full"))
    emit('    group "glass" clip=#true x=99 y=63 width=46 height=59 {')
    rect(99, 63, 46, 59, "glass.lit", gradient=text("down"), gradient_steps=-2)
    for x, w, h in [(100, 6, 30), (107, 5, 42), (113, 8, 24), (122, 6, 46), (129, 9, 33), (139, 6, 38)]:
        rect(x, 122 - h, w, h, "glass.base")
        for _ in range(3):
            rect(x + random.randint(1, w - 2), 122 - h + random.randint(3, h - 3), 1, 1, "bulb.hi",
                 emissive="#true")
    node("particles", x=95, y=50, width=56, height=10, count=40, vx=-8, vy=70, life=text("1s"),
         size=1, paint=text("glass.hi"), opacity=0.8)
    emit("    }")
    rect(120, 61, 3, 63, "wood.lit")
    rect(93, 122, 58, 3, "wood.lit", contact_shadow=1)
    node("light-rect", x=99, y=63, width=46, height=59, reach=14, intensity=0.8)


def lamps():
    emit("\n    // Pendant lamp and desk lamp: the two warm lights")
    node("rope", x1=96, y1=0, x2=96, y2=44, sag=0, width=1, paint=text("ink"))
    node("polygon", x=78, y=40, points=text("8,0 28,0 36,10 0,10"), paint=text("wood.base"),
         shading=text("dome"), outline=text("selective"))
    ellipse(90, 48, 12, 5, "bulb.hi", emissive="#true")
    node("light-cone", x=96, y=50, angle=text("90deg"), spread=text("140deg"), reach=120,
         intensity=text("=2.6 + wiggle(0.15, 3)"), color=text("warm"), shadows="#true")
    node("rope", x1=140, y1=125, x2=138, y2=98, sag=0, width=1, paint=text("ink"))
    node("rope", x1=138, y1=98, x2=146, y2=88, sag=0, width=1, paint=text("ink"))
    node("polygon", x=142, y=84, points=text("0,4 10,0 14,6 4,9"), paint=text("ink"))
    ellipse(147, 90, 6, 4, "bulb.hi", emissive="#true")
    node("light-cone", x=151, y=92, angle=text("60deg"), spread=text("100deg"), reach=70,
         intensity=2.6, color=text("warm"), shadows="#true")


def wall_things():
    emit("\n    // Calendar and a hanging sweater")
    rect(73, 71, 22, 19, "wall.hi", outline=text("selective"))
    rect(75, 73, 18, 8, "glass.base")
    ellipse(80, 74, 7, 6, "pink.base")
    for r in range(2):
        for c in range(6):
            rect(75 + c * 3, 83 + r * 3, 2, 2, "wall.deep")
    node("rounded-rect", x=81, y=96, width=13, height=19, radius=2, paint=text("sweater.shade"),
         shading=text("dome"), casts_shadow="#true")


def left_side():
    emit("\n    // Left bookshelf with stacks and a bust on top")
    rect(0, 58, 49, 106, "wood.base", casts_shadow="#true", outline=text("selective"))
    for y in (58, 81, 104, 127):
        rect(0, y, 49, 2, "wood.lit")
    for base in (80, 103, 126):
        book_row(1, 47, base, 12, 19)
    stack(2, 58, 6, 10, 15)
    stack(20, 58, 3, 8, 12)
    node("blob", "bust", x=42, y=46, circles=text("0,-3,6 0,4,5 -2,8,6"), smooth=3,
         paint=text("wood.lit"), shading=text("dome"), outline=text("selective"), casts_shadow="#true")

    emit("\n    // Desk, CRT and the person typing")
    rect(10, 133, 44, 3, "wood.lit", "desk", casts_shadow="#true", contact_shadow=1)
    rect(12, 136, 3, 30, "wood.base")
    node("rounded-rect", "crt", x=19, y=108, width=16, height=18, radius=2, paint=text("tv.hi"),
         shading=text("dome"), outline=text("selective"), casts_shadow="#true")
    rect(22, 111, 10, 11, "screen.deep", emissive="#true")
    for k in range(4):
        node("rect", x=23, y=112 + k * 2,
             width=text(f"=2 + floor(rand(mod(floor(t * 4), 8) + {k}) * 6)"), height=1,
             paint=text("screen.lit"), emissive="#true")
    node("light-rect", x=22, y=111, width=10, height=11, reach=16, intensity=1)
    rect(24, 125, 6, 8, "tv.base")
    rect(63, 106, 11, 8, "wood.base")
    rect(64, 107, 9, 6, "blue-screen.lit", emissive="#true")

    node("capsule", "chair", x1=48, y1=121, x2=48, y2=164, radius=2, paint=text("tv.lit"),
         shading=text("dome"))
    rect(45, 140, 30, 3, "tv.lit")
    rect(70, 140, 3, 26, "tv.lit")
    ellipse(49, 98, 13, 14, "hair.lit", "head", shading=text("dome"), outline=text("selective"))
    node("rounded-rect", x=50, y=104, width=8, height=8, radius=2, paint=text("skin.base"),
         shading=text("dome"))
    node("rounded-rect", "body", x=41, y=112, width=24, height=25, radius=5,
         paint=text("sweater.base"), shading=text("dome"), outline=text("selective"),
         casts_shadow="#true")
    for x in range(43, 63, 4):
        rect(x, 126, 2, 2, "pink.base")
    node("capsule", x1=33, y1=text("=130 + on2(square(t * 3))"), x2=42, y2=130, radius=1.5,
         paint=text("skin.base"))
    node("capsule", x1=36, y1=139, x2=66, y2=139, radius=3, paint=text("jeans.lit"),
         shading=text("dome"))
    node("capsule", x1=37, y1=142, x2=37, y2=164, radius=3, paint=text("jeans.base"),
         shading=text("dome"), contact_shadow=1)

    emit("\n    // Heater and stool")
    node("rounded-rect", x=14, y=156, width=15, height=10, radius=1, paint=text("tv.base"),
         outline=text("selective"), contact_shadow=1)
    for x in (16, 19, 22, 25):
        node("rect", x=x, y=158, width=2, height=6, paint=text("heater.lit"), emissive="#true",
             tone_shift=text("=round(wiggle(0.6, 2))"))
    node("light-rect", x=16, y=158, width=11, height=6, reach=12, intensity=1.2, color=text("warm"))
    rect(0, 150, 17, 3, "wood.lit", casts_shadow="#true")
    rect(2, 153, 3, 13, "wood.base")
    rect(12, 153, 3, 13, "wood.base")


def middle():
    emit("\n    // TV with the kid on top, books, cat and bag")
    node("rounded-rect", "tv", x=83, y=129, width=43, height=34, radius=3, paint=text("tv.base"),
         shading=text("dome"), outline=text("selective"), casts_shadow="#true", contact_shadow=1)
    rect(88, 135, 26, 22, "tv.shade", emissive="#true",
         tone_shift=text("=floor(wiggle(1.2, 1.5))"))
    node("light-rect", x=88, y=135, width=26, height=22, reach=12, intensity=0.8)
    ellipse(118, 136, 4, 4, "tv.hi")
    ellipse(118, 143, 4, 4, "tv.hi")
    stack(84, 129, 3, 12, 16)
    node("blob", "kid-head", x=106, y=106, circles=text("0,0,7 -4,-2,4 4,-3,4 0,-6,4"), smooth=2,
         paint=text("curls.base"), shading=text("dome"), outline=text("selective"))
    rect(110, 104, 4, 6, "skin.lit")
    node("rounded-rect", x=102, y=112, width=14, height=13, radius=3, paint=text("pink.base"),
         shading=text("dome"), outline=text("selective"), casts_shadow="#true")
    rect(100, 125, 18, 4, "jeans.base")
    node("blob", "cat", x=135, y=152, circles=text("0,0,10 -3,-14,6"), smooth=4, paint=text("ink"),
         casts_shadow="#true", contact_shadow=1)
    node("polygon", x=131, y=126, points=text("0,6 2,0 4,5 7,5 9,0 11,6"), paint=text("ink"))
    node("rope", "tail", x1=143, y1=158, x2=text("=151 + wiggle(2, 0.8)"),
         y2=text("=148 + wiggle(3, 0.6)"), sag=-2, sway=text("=sin(t * tau)"), width=1,
         paint=text("ink"))
    node("rounded-rect", x=124, y=151, width=13, height=14, radius=2, paint=text("pink.lit"),
         shading=text("dome"), outline=text("selective"), contact_shadow=1)
    node("rope", x1=127, y1=151, x2=134, y2=151, sag=-4, width=1, paint=text("pink.shade"))


def right_side():
    emit("\n    // Right wall shelves, desk and a mug with steam")
    for y in (57, 80, 103):
        rect(149, y, 43, 2, "wood.lit", casts_shadow="#true")
        node("line", x1=151, y1=y + 2, x2=155, y2=y + 6, paint=text("wood.lit"))
    book_row(150, 191, 79, 9, 15)
    book_row(156, 191, 102, 10, 16)
    rect(151, 40, 15, 16, "book-green.shade", outline=text("selective"))
    rect(153, 42, 11, 8, "book-green.lit")
    stack(168, 56, 6, 14, 20)
    rect(137, 127, 55, 3, "wood.lit", "right-desk", casts_shadow="#true", contact_shadow=1)
    node("line", x1=146, y1=130, x2=146, y2=136, paint=text("wood.lit"))
    node("rounded-rect", x=176, y=109, width=9, height=17, radius=1, paint=text("tv.base"),
         shading=text("dome"), outline=text("selective"), casts_shadow="#true")
    node("rope", x1=180, y1=106, x2=text("=180 + wiggle(2, 0.7)"), y2=text("=99 + wiggle(1, 0.5)"),
         sag=0, sway=text("=sin(t * tau) * 1.5"), width=1, paint=text("wall.hi"), opacity=0.7)
    rect(164, 114, 8, 8, "wall.hi", outline=text("selective"))
    stack(163, 166, 9, 16, 21)


emit("""// B1 re-attempt with M2/M4 features: lights with warm twins and cast shadows, dome shading,
// outlines, texture, gradients, a clipped window, ropes and particles.
// Generated by benchmarks/b1-cozy-room/generate_m2.py: the engine has no `repeat` yet.
""")
palette()
emit('scene "cozy-room" palette="night-room" width=192 height=192 fps=12 duration="2s" loop=#true '
     'seed=11 background="wall.lit" {')
room()
window()
lamps()
wall_things()
left_side()
middle()
right_side()
emit("}")
open(sys.argv[1], "w").write("\n".join(out) + "\n")
