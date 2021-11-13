use crate::regex::Regex;

#[derive(Debug)]
pub(crate) struct SshUserHost {
    user: String,
    host: String,
    port: Option<u16>,
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
            port,
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
        self.port.unwrap_or(22)
    }

    #[inline]
    pub(crate) fn user_host(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }
}
