use crate::validators::prelude::*;

#[derive(Debug, Validator)]
#[validator(regex("^[a-zA-Z0-9-_]{1,80}$"))]
pub(crate) struct PublicName(String);

impl AsRef<str> for PublicName {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
