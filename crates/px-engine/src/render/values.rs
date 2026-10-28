use px_expr::{Env, EvalError};

use super::RenderError;
use super::report::FrameReport;
use crate::model::Param;

/// Evaluates params for one object; non-finite results become 0 and are reported (FORMAT §9.8).
pub(crate) struct Values<'a> {
    pub(crate) env: Env,
    pub(crate) report: &'a mut FrameReport,
    pub(crate) object: usize,
    pub(crate) offset: [f64; 2],
}

impl Values<'_> {
    pub(crate) fn of(&mut self, param: &Param) -> Result<f64, RenderError> {
        match param.value(&self.env) {
            Ok(value) => Ok(value),
            Err(EvalError::NonFinite) => {
                self.report.record_non_finite(self.object);
                Ok(0.0)
            }
            Err(error) => Err(RenderError::Eval {
                shape: self.object,
                error,
            }),
        }
    }

    pub(crate) fn x(&mut self, param: &Param) -> Result<f64, RenderError> {
        Ok(self.of(param)? + self.offset[0])
    }

    pub(crate) fn y(&mut self, param: &Param) -> Result<f64, RenderError> {
        Ok(self.of(param)? + self.offset[1])
    }
}
