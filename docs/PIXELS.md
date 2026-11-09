# Pixel layers

A `pixels` node draws pixels one by one, usually one per `layer`, alongside shapes and lit like them (D-013). It takes its art from text grids (`cel`) or from a PNG (`src`), never both.

## Text grids
Hand-painted pixels, usually one per `layer`, drawn alongside shapes and lit like them (D-013). The editor (`pxe edit`, [EDITOR.md](EDITOR.md)) writes these nodes; they are also plain text to edit by hand.

```kdl
layer "cat" {
    pixels "cat-art" x=12 y=17 {
        legend { a "fur.1"; b "fur.2"; e "eye.1" }
        cel frame=0 """
        a....a
        abbbba
        bebbeb
        """
        cel frame=9 x=0 y=2 "bbbbbb"
    }
}
```

| Node | Props |
|---|---|
| `pixels "id"` | `x`, `y` (the layer's origin, animatable; 0), `paint` (outline and fallback color; the palette's darkest), every style prop (A.3) |
| `legend { code "paint" … }` | each code names a paint; codes are 1 or 2 characters, all the same length, never `.` |
| `cel` | the rows as a string, one code per pixel, `.` (or `..`) for transparent; `frame` (0), `x`, `y` (offset from the layer's origin; 0) |

- A cel shows from its `frame` until the next cel's; in a looping scene the frames wrap with the loop.
- The editor generates the legend (letters, then two-letter codes past 52 paints) and crops each cel to its painted pixels, so diffs stay small.
- A layer with a single cel at frame 0 never changes, so the renderer draws it once and copies it (D-011).

## From an image

```kdl
layer roots {
    pixels src="roots.png"
}
```

Traced or painted art that never moves can live in a PNG next to the scene file (D-029). The path is relative to the file that holds the node.

- **Colors become paints.** Every opaque pixel must be exactly one color of the scene's palette, and is drawn with that paint. A fully transparent pixel is empty; a partly transparent one is an error.
- **A color in two ramps** is an error naming both paints. Choose one with a legend keyed by color: `pixels src="roots.png" { legend { "#4d332a" root.1 } }`.
- **Bake it from shapes** with the exporter: put the static shapes in a scene of their own and run `pxe export that.kdl --format png --transparent --scale 1`. Anything that animates stays as text on top.
- **Limits.** Each side is at most 2048 pixels, and the image can hold at most 2,704 colors.
- **Editor.** Picking selects the `pixels` node, but painting on it is refused: edit the image. `pxe watch` and `pxe preview` reload when the PNG changes.
- **Zoom.** A camera zoom or `detail` scales the image up pixel by pixel, while shapes are drawn again at the finer grid, so their edges can differ by a pixel.
