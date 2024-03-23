use validators::prelude::*;

#[derive(Debug, Clone, Validator)]
#[validator(regex(regex(r"^[a-zA-Z0-9\-_.]{1,80}$")))]
pub(crate) struct Name(String);

impl AsRef<str> for Name {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
