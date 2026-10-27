use core::f64::consts::TAU;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Ease {
    Step,
    Linear,
    Smooth,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bounce,
    Elastic,
}

impl Ease {
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "step" => Self::Step,
            "linear" => Self::Linear,
            "smooth" => Self::Smooth,
            "ease-in" => Self::EaseIn,
            "ease-out" => Self::EaseOut,
            "ease-in-out" => Self::EaseInOut,
            "bounce" => Self::Bounce,
            "elastic" => Self::Elastic,
            _ => return None,
        })
    }

    #[must_use]
    pub fn apply(self, k: f64) -> f64 {
        let k = k.clamp(0.0, 1.0);
        match self {
            Self::Step => {
                if k < 1.0 {
                    0.0
                } else {
                    1.0
                }
            }
            Self::Linear => k,
            Self::Smooth => k * k * (3.0 - 2.0 * k),
            Self::EaseIn => k * k,
            Self::EaseOut => 1.0 - (1.0 - k) * (1.0 - k),
            Self::EaseInOut if k < 0.5 => 2.0 * k * k,
            Self::EaseInOut => 1.0 - 2.0 * (1.0 - k) * (1.0 - k),
            Self::Bounce => bounce(k),
            Self::Elastic if k <= 0.0 || k >= 1.0 => k,
            Self::Elastic => {
                libm::pow(2.0, -10.0 * k) * libm::sin((k * 10.0 - 0.75) * TAU / 3.0) + 1.0
            }
        }
    }
}

fn bounce(k: f64) -> f64 {
    const N: f64 = 7.5625;
    const D: f64 = 2.75;
    if k < 1.0 / D {
        N * k * k
    } else if k < 2.0 / D {
        let k = k - 1.5 / D;
        N * k * k + 0.75
    } else if k < 2.5 / D {
        let k = k - 2.25 / D;
        N * k * k + 0.9375
    } else {
        let k = k - 2.625 / D;
        N * k * k + 0.984_375
    }
}
