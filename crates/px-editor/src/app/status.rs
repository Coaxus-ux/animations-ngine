#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Status {
    pub text: String,
    pub problem: bool,
}

impl Status {
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            problem: false,
        }
    }

    pub fn problem(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            problem: true,
        }
    }

    #[must_use]
    pub fn from_result(result: Result<String, String>) -> Self {
        result.map_or_else(Self::problem, Self::info)
    }
}
