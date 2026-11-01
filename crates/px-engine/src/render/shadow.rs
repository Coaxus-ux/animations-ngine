use super::casters::Casters;
use super::surface::Surface;
use crate::model::MAX_LIGHTS;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Memo {
    row: i32,
    depth: u16,
    blocked: bool,
}

impl Memo {
    pub(crate) const EMPTY: Self = Self {
        row: i32::MIN,
        depth: 0,
        blocked: false,
    };
}

#[must_use]
pub(crate) const fn memo_cells(width: usize) -> usize {
    MAX_LIGHTS * (width + 1)
}

pub(crate) struct Shadows<'a> {
    casters: Option<Casters>,
    memo: &'a mut [Memo],
    grain: i32,
    columns: usize,
}

impl<'a> Shadows<'a> {
    pub(crate) fn new(
        surface: &Surface<'_>,
        casting: bool,
        memo: &'a mut [Memo],
        grain: i32,
    ) -> Self {
        let grain = grain.max(1);
        if casting && grain > 1 {
            memo.fill(Memo::EMPTY);
        }
        Self {
            casters: casting.then(|| Casters::find(surface)).flatten(),
            memo,
            grain,
            columns: usize::try_from(surface.width.div_euclid(grain) + 1).unwrap_or(0),
        }
    }

    #[inline(never)]
    pub(crate) fn blocked(
        &mut self,
        surface: &Surface<'_>,
        light: usize,
        (x, y): (i32, i32),
        source: (f64, f64),
    ) -> bool {
        let Some(casters) = self.casters else {
            return false;
        };
        if self.grain == 1 {
            return casters.block(surface, x, y, source);
        }
        let own = surface.index(x, y).map_or(0, |index| surface.depth[index]);
        let row = y.div_euclid(self.grain);
        let slot = usize::try_from(x.div_euclid(self.grain))
            .ok()
            .map(|column| light * self.columns + column)
            .and_then(|index| self.memo.get_mut(index));
        match slot {
            Some(memo) if memo.row == row && memo.depth == own => memo.blocked,
            Some(memo) => {
                let blocked = casters.block(surface, x, y, source);
                *memo = Memo {
                    row,
                    depth: own,
                    blocked,
                };
                blocked
            }
            None => casters.block(surface, x, y, source),
        }
    }
}
