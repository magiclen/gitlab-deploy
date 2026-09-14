use std::process::Stdio;

use anyhow::anyhow;
use execute::Execute;

use crate::{functions::*, models::SshUserHost};

pub(crate) struct Publication<'a> {
    host:    &'a SshUserHost,
    root:    &'a str,
    staging: String,
}

impl<'a> Publication<'a> {
    pub(crate) fn prepare(host: &'a SshUserHost, root: &'a str) -> anyhow::Result<Self> {
        let mut command = create_ssh_command(host, prepare_script(root));

        command.stdout(Stdio::piped());

        let output = command.execute_output()?;

        ensure_exit_success(output.status.code(), || {
            anyhow!("Cannot prepare the publication directory on {host}")
        })?;

        let staging = String::from_utf8(output.stdout)?.trim().to_owned();

        if !staging.starts_with(format!("{root}/.publish.").as_str()) {
            return Err(anyhow!("Cannot read the publication directory on {host}"));
        }

        Ok(Self {
            host,
            root,
            staging,
        })
    }

    pub(crate) fn html_path(&self) -> String {
        format!("{}/html", self.staging)
    }

    pub(crate) fn discard(self) {
        let mut command =
            create_ssh_command(self.host, format!("rm -rf -- {}", shell_quote(&self.staging)));

        if let Err(error) = ensure_command_success(&mut command, || {
            anyhow!("Cannot remove the publication directory {:?}", self.staging)
        }) {
            log::warn!("{error}");
        }
    }

    pub(crate) fn publish(self) -> anyhow::Result<()> {
        let mut command = create_ssh_command(self.host, publish_script(self.root, &self.staging));

        // The remote script owns cleanup now, since a failed restore must keep the backup.
        ensure_command_success(&mut command, || {
            anyhow!("Cannot publish the static files on {}", self.host)
        })
    }
}

fn prepare_script(root: &str) -> String {
    format!(
        r#"mkdir -p -- {root} &&
staging=$(mktemp -d -- {template}) &&
if mkdir -- "$staging/html"; then
    printf '%s\n' "$staging"
else
    rmdir -- "$staging"
    exit 1
fi"#,
        root = shell_quote(root),
        template = shell_quote(&format!("{root}/.publish.XXXXXXXXXX")),
    )
}

fn publish_script(root: &str, staging: &str) -> String {
    format!(
        r#"set -e
staging={staging}
html={html}
restore() {{
    status=$?
    trap - EXIT HUP INT TERM
    if test -e "$staging/previous" || test -L "$staging/previous"; then
        if ! mv -T -- "$staging/previous" "$html"; then
            printf 'Cannot restore the website; backup kept at %s\n' "$staging/previous" >&2
            exit 1
        fi
    fi
    rm -rf -- "$staging" || printf 'Cannot remove %s\n' "$staging" >&2
    exit "$status"
}}
trap restore EXIT
trap 'exit 1' HUP INT TERM
if test -e "$html" || test -L "$html"; then
    mv -T -- "$html" "$staging/previous"
fi
mv -T -- "$staging/html" "$html"
trap - EXIT HUP INT TERM
rm -rf -- "$staging" || printf 'Cannot remove %s\n' "$staging" >&2"#,
        staging = shell_quote(staging),
        html = shell_quote(&format!("{root}/html")),
    )
}

#[cfg(test)]
mod tests {
    use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

    use tempfile::tempdir;

    use super::*;

    fn prepare(root: &Path) -> String {
        let output = Command::new("sh")
            .args(["-c", &prepare_script(root.to_str().unwrap())])
            .output()
            .unwrap();

        assert!(output.status.success());

        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    #[test]
    fn publish_the_first_and_next_versions() {
        let temp = tempdir().unwrap();
        let root = temp.path().join("a site's files");

        for content in ["first", "next"] {
            let staging = prepare(&root);
            let html = Path::new(&staging).join("html");

            fs::write(html.join("index.html"), content).unwrap();
            fs::set_permissions(&html, fs::Permissions::from_mode(0o755)).unwrap();

            assert!(
                Command::new("sh")
                    .args(["-c", &publish_script(root.to_str().unwrap(), &staging)])
                    .status()
                    .unwrap()
                    .success()
            );
            assert_eq!(content, fs::read_to_string(root.join("html/index.html")).unwrap());
            assert_eq!(
                0o755,
                fs::metadata(root.join("html")).unwrap().permissions().mode() & 0o777
            );
            assert!(!Path::new(&staging).exists());
        }
    }

    #[test]
    fn restore_the_previous_version_when_the_switch_fails() {
        let temp = tempdir().unwrap();
        let root = temp.path();

        fs::create_dir(root.join("html")).unwrap();
        fs::write(root.join("html/index.html"), "previous").unwrap();

        let staging = prepare(root);

        fs::remove_dir(Path::new(&staging).join("html")).unwrap();

        assert!(
            !Command::new("sh")
                .args(["-c", &publish_script(root.to_str().unwrap(), &staging)])
                .output()
                .unwrap()
                .status
                .success()
        );
        assert_eq!("previous", fs::read_to_string(root.join("html/index.html")).unwrap());
        assert!(!Path::new(&staging).exists());
    }
}
