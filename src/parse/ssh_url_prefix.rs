use crate::validators::prelude::*;

#[derive(Debug, Validator)]
#[validator(regex(r"^(ssh://)?[^/\s]+@[^/\s:]+(?::[0-9]{1,5})?$"))]
pub(crate) struct SshUrlPrefix(String);

impl AsRef<str> for SshUrlPrefix {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
