use validators::prelude::*;

#[derive(Debug, Clone, Validator)]
#[validator(regex(regex(r"^(ssh://)?[^/\s]+@[^/\s:]+(?::[^/\s]+)?$")))]
pub(crate) struct SshUrlPrefix(String);

impl AsRef<str> for SshUrlPrefix {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
