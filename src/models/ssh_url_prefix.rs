use validators::prelude::*;

use super::ProjectPath;

#[derive(Debug, Clone, Validator)]
#[validator(regex(regex(r"^(ssh://)?[^/\s]+@[^/\s:]+(?::[^/\s]+)?$")))]
pub(crate) struct SshUrlPrefix(String);

impl SshUrlPrefix {
    pub(crate) fn repository_url(&self, project_path: &ProjectPath) -> String {
        let separator =
            if self.0.starts_with("ssh://") || self.0.contains(':') { '/' } else { ':' };

        format!("{}{separator}{}.git", self.0, project_path.as_ref())
    }
}

impl AsRef<str> for SshUrlPrefix {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_url_supports_ssh_and_scp_prefixes() {
        let path = ProjectPath::parse_str("group/project").unwrap();

        for (prefix, expected) in [
            ("git@example.com", "git@example.com:group/project.git"),
            ("ssh://git@example.com", "ssh://git@example.com/group/project.git"),
            ("ssh://git@example.com:2222", "ssh://git@example.com:2222/group/project.git"),
            ("git@example.com:base", "git@example.com:base/group/project.git"),
        ] {
            assert_eq!(expected, SshUrlPrefix::parse_str(prefix).unwrap().repository_url(&path));
        }
    }
}
