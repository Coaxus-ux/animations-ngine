use crate::units::{Axis, Quantity};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Prop {
    pub(crate) name: &'static str,
    pub(crate) quantity: Quantity,
    pub(crate) default: Option<f64>,
}

const fn required(name: &'static str, quantity: Quantity) -> Prop {
    Prop {
        name,
        quantity,
        default: None,
    }
}

const fn optional(name: &'static str, quantity: Quantity, default: f64) -> Prop {
    Prop {
        name,
        quantity,
        default: Some(default),
    }
}

const H: Quantity = Quantity::Length(Axis::Horizontal);
const V: Quantity = Quantity::Length(Axis::Vertical);
const N: Quantity = Quantity::Number;

const BOX: [Prop; 4] = [
    required("x", H),
    required("y", V),
    required("width", H),
    required("height", V),
];
const ENDS: [Prop; 4] = [
    required("x1", H),
    required("y1", V),
    required("x2", H),
    required("y2", V),
];
const ORIGIN: [Prop; 2] = [optional("x", H, 0.0), optional("y", V, 0.0)];
const STYLE: [Prop; 4] = [
    optional("opacity", N, 1.0),
    optional("tone-shift", N, 0.0),
    optional("visible", N, 1.0),
    optional("contact-shadow", N, 0.0),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    Rect,
    Ellipse,
    RoundedRect,
    Line,
    Capsule,
    Polygon,
    Blob,
    Rope,
    Particles,
    Text,
    Shade,
    LightPoint,
    LightCone,
    LightRect,
    Reflection,
    Camera,
    Group,
    Layer,
    Ambient,
    UseScene,
}

pub(crate) const SHAPE_NAMES: [(&str, Kind); 11] = [
    ("rect", Kind::Rect),
    ("ellipse", Kind::Ellipse),
    ("rounded-rect", Kind::RoundedRect),
    ("line", Kind::Line),
    ("capsule", Kind::Capsule),
    ("polygon", Kind::Polygon),
    ("blob", Kind::Blob),
    ("rope", Kind::Rope),
    ("particles", Kind::Particles),
    ("text", Kind::Text),
    ("shade", Kind::Shade),
];

const OTHER_NAMES: [(&str, Kind); 9] = [
    ("layer", Kind::Layer),
    ("ambient", Kind::Ambient),
    ("light-point", Kind::LightPoint),
    ("light-cone", Kind::LightCone),
    ("light-rect", Kind::LightRect),
    ("reflection", Kind::Reflection),
    ("camera", Kind::Camera),
    ("group", Kind::Group),
    ("use-scene", Kind::UseScene),
];

impl Kind {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        SHAPE_NAMES
            .iter()
            .chain(&OTHER_NAMES)
            .find(|(known, _)| *known == name)
            .map(|(_, kind)| *kind)
    }

    pub(crate) fn all_names() -> impl Iterator<Item = &'static str> {
        SHAPE_NAMES
            .iter()
            .chain(&OTHER_NAMES)
            .map(|(name, _)| *name)
    }

    pub(crate) const fn is_shape(self) -> bool {
        !matches!(
            self,
            Self::LightPoint
                | Self::LightCone
                | Self::LightRect
                | Self::Reflection
                | Self::Camera
                | Self::Group
                | Self::Layer
                | Self::Ambient
                | Self::UseScene
        )
    }

    fn geometry(self) -> &'static [Prop] {
        const GROUP: [Prop; 4] = [
            optional("x", H, 0.0),
            optional("y", V, 0.0),
            optional("width", H, 0.0),
            optional("height", V, 0.0),
        ];
        const ROUNDED: [Prop; 5] = [BOX[0], BOX[1], BOX[2], BOX[3], optional("radius", N, 2.0)];
        const LINE: [Prop; 5] = [
            ENDS[0],
            ENDS[1],
            ENDS[2],
            ENDS[3],
            optional("width", N, 1.0),
        ];
        const CAPSULE: [Prop; 5] = [
            ENDS[0],
            ENDS[1],
            ENDS[2],
            ENDS[3],
            optional("radius", N, 2.0),
        ];
        const BLOB: [Prop; 3] = [ORIGIN[0], ORIGIN[1], optional("smooth", N, 3.0)];
        const ROPE: [Prop; 7] = [
            ENDS[0],
            ENDS[1],
            ENDS[2],
            ENDS[3],
            optional("sag", N, 0.0),
            optional("sway", N, 0.0),
            optional("width", N, 1.0),
        ];
        const PARTICLES: [Prop; 11] = [
            BOX[0],
            BOX[1],
            BOX[2],
            BOX[3],
            optional("vx", N, 0.0),
            optional("vy", N, 0.0),
            optional("gravity", N, 0.0),
            optional("life", Quantity::Time, 1.0),
            optional("size", N, 1.0),
            optional("wind", N, 0.0),
            optional("streak", Quantity::Time, 0.0),
        ];
        const TEXT: [Prop; 5] = [
            required("x", H),
            required("y", V),
            optional("size", N, 1.0),
            optional("reveal", N, f64::INFINITY),
            optional("scatter", N, 0.0),
        ];
        const LAYER: [Prop; 1] = [optional("fog", N, 0.0)];
        const AMBIENT: [Prop; 1] = [optional("darkness", N, 0.0)];
        const CAMERA: [Prop; 3] = [ORIGIN[0], ORIGIN[1], optional("zoom", N, 1.0)];
        // Everything but x, y and scale is read once per copy when the file loads.
        const USE_SCENE: [Prop; 10] = [
            ORIGIN[0],
            ORIGIN[1],
            optional("scale", N, 1.0),
            optional("flip", N, 0.0),
            optional("seed", N, 0.0),
            optional("count", N, 1.0),
            optional("start", Quantity::Time, 0.0),
            optional("end", Quantity::Time, f64::INFINITY),
            optional("offset", Quantity::Time, 0.0),
            optional("speed", N, 1.0),
        ];
        const SHADE: [Prop; 5] = [BOX[0], BOX[1], BOX[2], BOX[3], optional("steps", N, -1.0)];
        match self {
            Self::Rect | Self::Ellipse => &BOX,
            Self::Group => &GROUP,
            Self::RoundedRect => &ROUNDED,
            Self::Line => &LINE,
            Self::Capsule => &CAPSULE,
            Self::Polygon => &ORIGIN,
            Self::Camera => &CAMERA,
            Self::Blob => &BLOB,
            Self::Rope => &ROPE,
            Self::Particles => &PARTICLES,
            Self::Text => &TEXT,
            Self::Shade => &SHADE,
            Self::Layer => &LAYER,
            Self::Ambient => &AMBIENT,
            Self::UseScene => &USE_SCENE,
            Self::LightPoint | Self::LightCone | Self::LightRect | Self::Reflection => {
                self.extras()
            }
        }
    }

    fn extras(self) -> &'static [Prop] {
        const POINT: [Prop; 4] = [
            required("x", H),
            required("y", V),
            optional("reach", N, 40.0),
            optional("intensity", N, 1.0),
        ];
        const CONE: [Prop; 6] = [
            POINT[0],
            POINT[1],
            POINT[2],
            POINT[3],
            optional("angle", Quantity::Angle, 90.0),
            optional("spread", Quantity::Angle, 60.0),
        ];
        const RECT: [Prop; 6] = [
            BOX[0],
            BOX[1],
            BOX[2],
            BOX[3],
            optional("reach", N, 40.0),
            optional("intensity", N, 1.0),
        ];
        const REFLECTION: [Prop; 9] = [
            BOX[0],
            BOX[1],
            BOX[2],
            BOX[3],
            required("axis", V),
            optional("ripple", N, 1.0),
            optional("speed", N, 2.0),
            optional("tone-shift", N, -1.0),
            optional("opacity", N, 1.0),
        ];
        match self {
            Self::LightCone => &CONE,
            Self::LightRect => &RECT,
            Self::Reflection => &REFLECTION,
            _ => &POINT,
        }
    }

    /// Shapes, lights and placed scenes can be attached to another object.
    pub(crate) const fn can_attach(self) -> bool {
        self.is_shape()
            || matches!(
                self,
                Self::LightPoint | Self::LightCone | Self::LightRect | Self::UseScene
            )
    }

    pub(crate) fn props(self) -> impl Iterator<Item = &'static Prop> {
        let style: &'static [Prop] = if self.is_shape() { &STYLE } else { &[] };
        self.geometry().iter().chain(style)
    }

    pub(crate) fn prop_index(self, name: &str) -> Option<usize> {
        self.props().position(|prop| prop.name == name)
    }

    pub(crate) fn prop_count(self) -> usize {
        self.props().count()
    }
}
