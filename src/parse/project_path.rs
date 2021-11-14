use crate::validators::prelude::*;

#[derive(Debug, Validator)]
#[validator(line(char_length(trimmed_min = 1)))]
pub(crate) struct ProjectPath(String);

impl AsRef<str> for ProjectPath {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
