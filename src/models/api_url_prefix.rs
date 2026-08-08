use slash_formatter::delete_end_slash;
use validators::prelude::*;
use validators_prelude::url;

#[derive(Debug, Clone, Validator)]
#[validator(http_url(local(Allow)))]
pub(crate) struct ApiUrlPrefix {
    url:      url::Url,
    is_https: bool,
}

impl ApiUrlPrefix {
    /// Returns whether the URL uses HTTPS.
    #[inline]
    pub(crate) fn is_https(&self) -> bool {
        self.is_https
    }
}

impl AsRef<str> for ApiUrlPrefix {
    #[inline]
    fn as_ref(&self) -> &str {
        delete_end_slash(self.url.as_str())
    }
}
