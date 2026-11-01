use px_core::{Paint, RampId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Origin {
    pub file: usize,
    pub offset: usize,
}

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

    #[must_use]
    pub fn find(&self, name: &str) -> Option<RampId> {
        self.ramps
            .iter()
            .find(|(ramp, _)| ramp == name)
            .map(|(_, id)| *id)
    }

    pub fn ramps(&self) -> impl Iterator<Item = (&str, RampId)> {
        self.ramps.iter().map(|(name, id)| (name.as_str(), *id))
    }

    #[must_use]
    pub fn paint(&self, paint: Paint) -> Option<String> {
        self.ramp(paint.ramp)
            .map(|ramp| format!("{ramp}.{}", paint.tone.index()))
    }
}
