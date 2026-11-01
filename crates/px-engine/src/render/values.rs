use px_expr::{Env, EvalError};

use super::RenderError;
use super::report::FrameReport;
use super::transform::Transform;
use crate::model::Param;

pub(crate) struct Values<'a> {
    pub(crate) env: Env,
    pub(crate) report: &'a mut FrameReport,
    pub(crate) object: usize,
    pub(crate) transform: Transform,
    pub(crate) timed: bool,
}

impl Values<'_> {
    pub(crate) fn of(&mut self, param: &Param) -> Result<f64, RenderError> {
        self.timed |= !param.is_timeless();
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

    pub(crate) fn y(&mut self, param: &Param) -> Result<f64, RenderError> {
        let y = self.of(param)?;
        Ok(self.transform.y(y))
    }

    pub(crate) fn point(&mut self, x: &Param, y: &Param) -> Result<[f64; 2], RenderError> {
        let local = [self.of(x)?, self.of(y)?];
        Ok(self.transform.point(local))
    }

    pub(crate) fn extent(&mut self, param: &Param) -> Result<f64, RenderError> {
        let length = self.of(param)?;
        Ok(self.transform.extent(length))
    }
}
