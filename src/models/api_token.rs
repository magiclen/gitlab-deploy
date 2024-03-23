use validators::prelude::*;

#[derive(Debug, Clone, Validator)]
#[validator(regex(regex(r"^\S+$")))]
pub(crate) struct ApiToken(String);

impl AsRef<str> for ApiToken {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
