use px_core::Size;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Rect {
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) width: u16,
    pub(crate) height: u16,
}

pub(crate) fn changed<T: PartialEq>(size: Size, current: &[T], previous: Option<&[T]>) -> Rect {
    let (width, height) = (size.width(), size.height());
    let full = Rect {
        x: 0,
        y: 0,
        width,
        height,
    };
    let Some(previous) = previous else {
        return full;
    };
    let columns = usize::from(width);
    let rows = current
        .chunks_exact(columns)
        .zip(previous.chunks_exact(columns));
    let mut bounds: Option<[usize; 4]> = None;
    for (y, (now, before)) in rows.enumerate() {
        if now == before {
            continue;
        }
        let differs = |(a, b): (&T, &T)| a != b;
        let first = now.iter().zip(before).position(differs).unwrap_or(0);
        let last = columns
            - 1
            - now
                .iter()
                .rev()
                .zip(before.iter().rev())
                .position(differs)
                .unwrap_or(0);
        let [x0, y0, x1, _] = bounds.unwrap_or([first, y, last, y]);
        bounds = Some([x0.min(first), y0, x1.max(last), y]);
    }
    let Some([x0, y0, x1, y1]) = bounds else {
        return Rect {
            width: 1,
            height: 1,
            ..full
        };
    };
    let narrow = |value: usize| u16::try_from(value).unwrap_or(u16::MAX);
    Rect {
        x: narrow(x0),
        y: narrow(y0),
        width: narrow(x1 - x0 + 1),
        height: narrow(y1 - y0 + 1),
    }
}

pub(crate) fn pixels<'a, T: Copy>(
    size: Size,
    rect: Rect,
    current: &'a [T],
    previous: Option<&'a [T]>,
) -> impl Iterator<Item = (T, Option<T>)> + 'a {
    let columns = usize::from(size.width());
    let (x, width) = (usize::from(rect.x), usize::from(rect.width));
    (usize::from(rect.y)..usize::from(rect.y) + usize::from(rect.height)).flat_map(move |y| {
        let start = y * columns + x;
        let now = &current[start..start + width];
        let before = previous.map(|previous| &previous[start..start + width]);
        (0..width).map(move |i| (now[i], before.map(|before| before[i])))
    })
}
