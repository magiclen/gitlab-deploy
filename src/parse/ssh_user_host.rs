use std::fmt::{self, Display, Formatter};

use crate::regex::Regex;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub(crate) struct SshUserHost {
    user: String,
    host: String,
    port: u16,
}

impl SshUserHost {
    pub(crate) fn parse_str<S: AsRef<str>>(s: S) -> Result<Self, ()> {
        let s = s.as_ref();

        let regex = Regex::new(r"^([^/\s]+)@([^/\s:]+)(?::([0-9]{1,5}))?$").unwrap();

        let result = regex.captures(s).ok_or(())?;

        let user = result.get(1).unwrap().as_str();
        let host = result.get(2).unwrap().as_str();
        let port = match result.get(3) {
            Some(port) => Some(port.as_str().parse::<u16>().map_err(|_| ())?),
            None => None,
        };

        Ok(SshUserHost {
            user: String::from(user),
            host: String::from(host),
            port: port.unwrap_or(22),
        })
    }
}

impl SshUserHost {
    #[inline]
    pub(crate) fn get_user(&self) -> &str {
        self.user.as_str()
    }

    #[inline]
    pub(crate) fn get_host(&self) -> &str {
        self.host.as_str()
    }

    #[inline]
    pub(crate) fn get_port(&self) -> u16 {
        self.port
    }

    #[inline]
    pub(crate) fn user_host(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }
}

impl Display for SshUserHost {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        if self.port != 22 {
            f.write_fmt(format_args!("{}@{}:{}", self.user, self.host, self.port))
        } else {
            f.write_fmt(format_args!("{}@{}", self.user, self.host))
        }
    }
}
