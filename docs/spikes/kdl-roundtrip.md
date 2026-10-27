# Spike: KDL round-trip editing (M0)

> Time-boxed, throwaway code (not in the repo). Question: can the editor change a value in a hand-written KDL file without destroying comments and formatting? ([ROADMAP.md](../ROADMAP.md) M0, [EDITOR.md](../EDITOR.md))

## Setup
- Crate: `kdl` 6.7.1 (KDL v2), `default-features = false, features = ["span"]`.
- Input: a scene with a header comment, trailing comments, a slashdash-disabled node (`/-ellipse`), odd spacing (`light-cone   color="warm"  reach=80`) and a nested `anim` block.
- Edits: change a property value (`reach=80 → 95`), change a positional value (`param rain 0.8 → 0.35`), insert a keyframe node.

## Results

| Edit | Result |
|---|---|
| Parse + print without edits | Byte-identical output |
| Change a property / argument value | Only the edited token changes; comments, spacing, the slashdash node and all other lines are untouched |
| Insert a new node (keyframe) | Works; copying the sibling's format gives the right indentation, but produced one extra blank line |

## Gotchas
1. **`KdlEntry::set_value` alone changes nothing in the output.** Entries keep their original text (`format().value_repr`). The edit layer must also set `value_repr` to the new value's text.
2. **New nodes need their whitespace normalized.** Copying a sibling's `KdlNodeFormat` duplicates its leading newline. The edit layer should set `leading` to the sibling's indentation only and `terminator` to `\n`.

## Verdict
**Go.** `kdl` 6.7 is format-preserving and good enough for the editor's round-trip ([EDITOR.md](../EDITOR.md) §3). We need a thin edit layer in `px-format` (M7) that:
- updates values through one function that also rewrites `value_repr`;
- inserts nodes with normalized indentation and terminators;
- is covered by round-trip fuzz tests on every benchmark file ([ROADMAP.md](../ROADMAP.md) M7 risks).

No fallback (our own format-preserving parser) is needed.
