use std::{
    fmt::{self, Display, Formatter},
    sync::LazyLock,
};

use regex::Regex;

static USER_HOST_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([^/\s]+)@([^/\s:]+)(?::([0-9]{1,5}))?$").unwrap());

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub(crate) struct SshUserHost {
    user_host: String,
    port:      u16,
}

impl SshUserHost {
    pub(crate) fn parse_str<S: AsRef<str>>(s: S) -> Result<Self, ()> {
        let s = s.as_ref();

        let result = USER_HOST_REGEX.captures(s).ok_or(())?;

        let user = result.get(1).unwrap().as_str();
        let host = result.get(2).unwrap().as_str();
        let port = match result.get(3) {
            Some(port) => Some(port.as_str().parse::<u16>().map_err(|_| ())?),
            None => None,
        };

        let user_host = format!("{user}@{host}");

        Ok(SshUserHost {
            user_host,
            port: port.unwrap_or(22),
        })
    }

    #[inline]
    pub(crate) fn get_port(&self) -> u16 {
        self.port
    }

    #[inline]
    pub(crate) fn user_host(&self) -> &str {
        self.user_host.as_str()
    }
}

impl Display for SshUserHost {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        if self.port != 22 {
            f.write_fmt(format_args!(
                "{user_host}:{port}",
                user_host = self.user_host,
                port = self.port
            ))
        } else {
            f.write_str(self.user_host.as_str())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_without_a_port_uses_22() {
        let ssh_user_host = SshUserHost::parse_str("alice@a.example.com").unwrap();

        assert_eq!(22, ssh_user_host.get_port());
        assert_eq!("alice@a.example.com", ssh_user_host.user_host());
        assert_eq!("alice@a.example.com", ssh_user_host.to_string());
    }

    #[test]
    fn parse_with_a_port() {
        let ssh_user_host = SshUserHost::parse_str("bob@b.example.com:2222").unwrap();

        assert_eq!(2222, ssh_user_host.get_port());
        assert_eq!("bob@b.example.com", ssh_user_host.user_host());
        assert_eq!("bob@b.example.com:2222", ssh_user_host.to_string());
    }
}
