use validators::prelude::*;

#[derive(Debug, Clone, Validator)]
#[validator(regex(regex(r"^[a-z0-9\-_]{1,80}$")))]
pub(crate) struct BuildTarget(String);

impl AsRef<str> for BuildTarget {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
