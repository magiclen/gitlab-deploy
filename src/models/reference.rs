use validators::prelude::*;

#[derive(Debug, Clone, Validator)]
#[validator(line(char_length(trimmed_min = 1)))]
pub(crate) struct Reference(String);

impl AsRef<str> for Reference {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
