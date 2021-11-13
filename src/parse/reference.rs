use crate::validators::prelude::*;

#[derive(Debug, Validator)]
#[validator(line)]
pub(crate) struct Reference(String);

impl AsRef<str> for Reference {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
