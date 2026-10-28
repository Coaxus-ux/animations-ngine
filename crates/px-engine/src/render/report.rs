#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FrameReport {
    pub non_finite_values: u32,
    pub first_non_finite_shape: Option<usize>,
}

impl FrameReport {
    pub(crate) fn record_non_finite(&mut self, shape: usize) {
        self.non_finite_values = self.non_finite_values.saturating_add(1);
        self.first_non_finite_shape.get_or_insert(shape);
    }
}
