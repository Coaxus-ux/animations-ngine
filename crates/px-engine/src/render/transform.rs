/// A uniform scale, an optional mirror on x and a translation: how local coordinates
/// (inside an instance, a layer, the camera) map to canvas pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Transform {
    pub(crate) scale: f64,
    pub(crate) flip: bool,
    pub(crate) dx: f64,
    pub(crate) dy: f64,
}

impl Transform {
    pub(crate) const IDENTITY: Self = Self {
        scale: 1.0,
        flip: false,
        dx: 0.0,
        dy: 0.0,
    };

    pub(crate) fn x(&self, x: f64) -> f64 {
        self.dx + self.scale * if self.flip { -x } else { x }
    }

    pub(crate) fn y(&self, y: f64) -> f64 {
        self.dy + self.scale * y
    }

    pub(crate) fn point(&self, [x, y]: [f64; 2]) -> [f64; 2] {
        [self.x(x), self.y(y)]
    }

    pub(crate) fn extent(&self, length: f64) -> f64 {
        self.scale * length
    }

    /// The left edge and width on the canvas of the local span `x..x + width`.
    pub(crate) fn span_x(&self, x: f64, width: f64) -> (f64, f64) {
        let width = self.extent(width);
        let left = self.x(x);
        (if self.flip { left - width } else { left }, width)
    }

    /// An angle in radians as seen after the mirror (0 points right, y grows down).
    pub(crate) fn angle(&self, radians: f64) -> f64 {
        if self.flip {
            core::f64::consts::PI - radians
        } else {
            radians
        }
    }

    /// The local coordinates of a canvas point.
    pub(crate) fn local(&self, [x, y]: [f64; 2]) -> [f64; 2] {
        let scale = if self.scale == 0.0 { 1.0 } else { self.scale };
        let lx = (x - self.dx) / scale;
        [if self.flip { -lx } else { lx }, (y - self.dy) / scale]
    }

    /// `self` applied first, then `outer`.
    pub(crate) fn then(&self, outer: &Self) -> Self {
        Self {
            scale: self.scale * outer.scale,
            flip: self.flip != outer.flip,
            dx: outer.x(self.dx),
            dy: outer.y(self.dy),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Transform;

    #[test]
    fn composition_matches_applying_in_order() {
        let inner = Transform {
            scale: 2.0,
            flip: true,
            dx: 10.0,
            dy: -3.0,
        };
        let outer = Transform {
            scale: 1.5,
            flip: false,
            dx: -4.0,
            dy: 7.0,
        };
        let both = inner.then(&outer);
        let point = [3.0, 5.0];
        assert_eq!(both.point(point), outer.point(inner.point(point)));
        let back = both.local(both.point(point));
        assert!((back[0] - point[0]).abs() < 1e-9 && (back[1] - point[1]).abs() < 1e-9);
    }

    #[test]
    fn identity_leaves_values_untouched() {
        let shifted = Transform {
            dx: -24.3,
            ..Transform::IDENTITY
        };
        assert_eq!(shifted.x(10.1), 10.1 + -24.3);
        assert_eq!(
            Transform::IDENTITY.then(&shifted).dx,
            shifted.dx,
            "composing with the identity is exact"
        );
        assert_eq!(shifted.span_x(2.0, 5.5), (2.0 + -24.3, 5.5));
    }
}
