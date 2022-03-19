use validators::prelude::*;

#[derive(Debug, Validator)]
#[validator(regex("^[a-zA-Z0-9]{40}$"))]
pub(crate) struct CommitSha(String);

impl CommitSha {
    #[inline]
    pub(crate) fn get_sha(&self) -> &str {
        self.0.as_str()
    }

    #[inline]
    pub(crate) fn get_short_sha(&self) -> &str {
        &self.get_sha()[..8]
    }
}

impl AsRef<str> for CommitSha {
    #[inline]
    fn as_ref(&self) -> &str {
        self.get_sha()
    }
}
