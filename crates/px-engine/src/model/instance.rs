use super::param::Param;

/// When an instance plays, in its parent's time: hidden before `start` and from `end` on;
/// its local time is `offset + (t − start) × speed`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    pub start: f64,
    pub end: f64,
    pub offset: f64,
    pub speed: f64,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            start: 0.0,
            end: f64::INFINITY,
            offset: 0.0,
            speed: 1.0,
        }
    }
}

/// A scene placed inside another scene. Shapes, lights, clips and reflections that belong to
/// it name it by index and are drawn through its transform, at its local time.
#[derive(Clone, Debug, PartialEq)]
pub struct Instance {
    /// An earlier instance this one is nested in.
    pub parent: Option<usize>,
    /// Placement in the parent's coordinates and time.
    pub x: Param,
    pub y: Param,
    pub scale: Param,
    pub flip: bool,
    pub window: Window,
    /// Length of the placed scene in seconds; local time wraps at it when `looping`,
    /// otherwise holds its last frame.
    pub duration: f64,
    pub looping: bool,
    /// Position among the copies made by one `count`, read as `i` and `n`.
    pub index: u32,
    pub count: u32,
    /// Root of every seed inside the instance.
    pub seed: u64,
}

impl Default for Instance {
    fn default() -> Self {
        Self {
            parent: None,
            x: Param::Fixed(0.0),
            y: Param::Fixed(0.0),
            scale: Param::Fixed(1.0),
            flip: false,
            window: Window::default(),
            duration: 0.0,
            looping: true,
            index: 0,
            count: 1,
            seed: 0,
        }
    }
}
