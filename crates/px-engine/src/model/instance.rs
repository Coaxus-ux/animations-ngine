use super::param::Param;

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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Anchor {
    pub instance: Option<usize>,
    pub x: Param,
    pub y: Param,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Instance {
    pub parent: Option<usize>,
    pub x: Param,
    pub y: Param,
    pub scale: Param,
    pub flip: bool,
    pub window: Window,
    pub duration: f64,
    pub looping: bool,
    pub index: u32,
    pub count: u32,
    pub seed: u64,
    pub anchor: Option<Anchor>,
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
            anchor: None,
        }
    }
}
