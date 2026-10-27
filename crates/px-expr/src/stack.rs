use crate::error::EvalError;

pub(crate) struct Stack<T: Copy, const N: usize> {
    values: [T; N],
    len: usize,
}

impl<T: Copy, const N: usize> Stack<T, N> {
    pub(crate) const fn new(fill: T) -> Self {
        Self {
            values: [fill; N],
            len: 0,
        }
    }

    pub(crate) fn push(&mut self, value: T) -> Result<(), EvalError> {
        *self.values.get_mut(self.len).ok_or(EvalError::Malformed)? = value;
        self.len += 1;
        Ok(())
    }

    pub(crate) fn pop(&mut self) -> Result<T, EvalError> {
        self.len = self.len.checked_sub(1).ok_or(EvalError::Malformed)?;
        Ok(self.values[self.len])
    }

    pub(crate) fn pop_into<'a>(
        &mut self,
        count: usize,
        dest: &'a mut [T],
    ) -> Result<&'a [T], EvalError> {
        let start = self.len.checked_sub(count).ok_or(EvalError::Malformed)?;
        let dest = dest.get_mut(..count).ok_or(EvalError::Malformed)?;
        dest.copy_from_slice(&self.values[start..self.len]);
        self.len = start;
        Ok(dest)
    }

    pub(crate) fn finish(&self) -> Result<T, EvalError> {
        match self.values.get(..self.len) {
            Some([value]) => Ok(*value),
            _ => Err(EvalError::Malformed),
        }
    }
}
