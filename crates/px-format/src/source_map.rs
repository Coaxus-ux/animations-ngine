use px_core::{Paint, RampId};

/// The node a shape was built from: a file of the load (`Loaded::files`) and the byte
/// offset of the node's name in it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Origin {
    pub file: usize,
    pub offset: usize,
}

/// The names of a scene's palette ramps, to write paints back as text (`"skin.3"`) and to
/// find ramps by name.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PaintNames {
    ramps: Vec<(String, RampId)>,
}

impl PaintNames {
    pub(crate) fn new(named: impl Iterator<Item = (String, RampId)>) -> Self {
        let mut ramps: Vec<(String, RampId)> = named.collect();
        ramps.sort_by_key(|(_, id)| *id);
        Self { ramps }
    }

    #[must_use]
    pub fn ramp(&self, id: RampId) -> Option<&str> {
        self.ramps
            .iter()
            .find(|(_, ramp)| *ramp == id)
            .map(|(name, _)| name.as_str())
    }

    /// The ramp called `name`.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<RampId> {
        self.ramps
            .iter()
            .find(|(ramp, _)| ramp == name)
            .map(|(_, id)| *id)
    }

    /// Ramps in palette order.
    pub fn ramps(&self) -> impl Iterator<Item = (&str, RampId)> {
        self.ramps.iter().map(|(name, id)| (name.as_str(), *id))
    }

    /// `ramp.tone`, with the tone as its index, which every ramp accepts.
    #[must_use]
    pub fn paint(&self, paint: Paint) -> Option<String> {
        self.ramp(paint.ramp)
            .map(|ramp| format!("{ramp}.{}", paint.tone.index()))
    }
}
