use crate::validators::prelude::*;

#[derive(Debug, Validator)]
#[validator(regex(r"^\S+$"))]
pub(crate) struct ApiToken(String);

impl AsRef<str> for ApiToken {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
