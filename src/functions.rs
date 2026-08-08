use std::{
    borrow::Cow,
    collections::BTreeSet,
    env,
    fmt::{self, Display, Formatter},
    fs::{self, File},
    io::{BufRead, BufReader, ErrorKind, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::anyhow;
use chrono::{
    Local,
    format::{DelayedFormat, StrftimeItems},
};
use execute::{Execute, command, command_args};
use regex::Regex;
use scanner_rust::{ScannerError, ScannerStr};
use slash_formatter::delete_end_slash_in_place;
use tempfile::{NamedTempFile, TempDir, tempdir};
use trim_in_place::TrimInPlace;
use validators::prelude::*;

use crate::{constants::*, models::*};

/// A string wrapped in POSIX single quotes so that a shell reads it as one literal word.
pub(crate) struct ShellQuoted<'a>(&'a str);

impl Display for ShellQuoted<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str("'")?;

        // A single quote cannot be escaped inside single quotes, so close the quoting, emit an escaped quote, and open the quoting again.
        let mut rest = self.0;

        while let Some(index) = rest.find('\'') {
            f.write_str(&rest[..index])?;
            f.write_str("'\\''")?;

            rest = &rest[(index + 1)..];
        }

        f.write_str(rest)?;

        f.write_str("'")
    }
}

#[inline]
pub(crate) fn shell_quote(s: &str) -> ShellQuoted<'_> {
    ShellQuoted(s)
}

#[inline]
pub(crate) fn check_command(program: &str, version_arg: &str) -> anyhow::Result<()> {
    let mut command = command_args!(program, version_arg);

    if command.execute_check_exit_status_code(0).is_err() {
        return Err(anyhow!("Cannot find {program}."));
    }

    Ok(())
}

/// Treats an exit status other than `0` as an error built by `error`.
#[inline]
pub(crate) fn ensure_exit_success<E: FnOnce() -> anyhow::Error>(
    status: Option<i32>,
    error: E,
) -> anyhow::Result<()> {
    match status {
        Some(0) => Ok(()),
        _ => Err(error()),
    }
}

/// Runs the command with the standard streams inherited and treats an exit status other than `0` as an error built by `error`.
#[inline]
pub(crate) fn ensure_command_success<E: FnOnce() -> anyhow::Error>(
    command: &mut Command,
    error: E,
) -> anyhow::Result<()> {
    ensure_exit_success(command.execute_output()?.status.code(), error)
}

/// Runs `first | second` with the standard error streams inherited and treats an exit status other than `0` of either command as an error built by `error`.
pub(crate) fn ensure_pipeline_success<E: FnOnce() -> anyhow::Error>(
    first: &mut Command,
    second: &mut Command,
    error: E,
) -> anyhow::Result<()> {
    first.stdout(Stdio::piped());

    let mut first_child = first.spawn()?;

    // The read end is moved into `second`, which keeps owning it until it is replaced below.
    let first_stdout = first_child.stdout.take().unwrap();

    second.stdin(first_stdout);

    // The second command has to be waited for first, otherwise a full pipe would deadlock both of them.
    let second_status = match second.status() {
        Ok(second_status) => second_status,
        Err(err) => {
            // Nothing is reading the pipe now, so the first command has to be stopped instead of being waited for.
            let _ = first_child.kill();
            let _ = first_child.wait();

            return Err(err.into());
        },
    };

    // `second` still owns a copy of the read end of the pipe, which has to be closed before waiting, otherwise `first` would block forever on a full pipe when `second` exited early.
    second.stdin(Stdio::null());

    let first_status = first_child.wait()?;

    if !first_status.success() || !second_status.success() {
        return Err(error());
    }

    Ok(())
}

pub(crate) fn log_stderr(stderr: &[u8], level: log::Level) {
    String::from_utf8_lossy(stderr).split('\n').for_each(|line| {
        if !line.is_empty() {
            log::log!(level, "{line}");
        }
    });
}

/// Reads a file under `deploy` that holds a single name and validates it.
fn read_deploy_name<T: ValidateString>(deploy_dir: &Path, file_name: &str) -> anyhow::Result<T> {
    match fs::read_to_string(deploy_dir.join(file_name)) {
        Ok(mut name) => {
            name.trim_in_place();

            T::parse_string(name).map_err(|_| anyhow!("deploy/{file_name} is not correct"))
        },
        Err(ref error) if error.kind() == ErrorKind::NotFound => {
            Err(anyhow!("deploy/{file_name} cannot be found in the project."))
        },
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn check_front_deploy(temp_dir: &TempDir) -> anyhow::Result<Name> {
    let deploy_dir = temp_dir.path().join("deploy");

    if !deploy_dir.join("build.sh").is_file() {
        return Err(anyhow!("deploy/build.sh cannot be found in the project."));
    }

    read_deploy_name(deploy_dir.as_path(), "public-name.txt")
}

pub(crate) fn check_back_deploy(
    temp_dir: &TempDir,
    commit_sha: &CommitSha,
    build_target: Option<&BuildTarget>,
) -> anyhow::Result<(ImageName, String)> {
    let deploy_dir = temp_dir.path().join("deploy");

    if !deploy_dir.join("build.sh").is_file() {
        return Err(anyhow!("deploy/build.sh cannot be found in the project."));
    }

    if !deploy_dir.join("develop-up.sh").is_file() {
        return Err(anyhow!("deploy/develop-up.sh cannot be found in the project."));
    }

    if !deploy_dir.join("develop-down.sh").is_file() {
        return Err(anyhow!("deploy/develop-down.sh cannot be found in the project."));
    }

    let image_name: ImageName = read_deploy_name(deploy_dir.as_path(), "image-name.txt")?;

    let docker_compose_name = if let Some(build_target) = build_target {
        Cow::Owned(format!(
            "docker-compose.{build_target}.yml",
            build_target = build_target.as_ref()
        ))
    } else {
        Cow::Borrowed("docker-compose.yml")
    };

    let docker_compose = match fs::read_to_string(deploy_dir.join(docker_compose_name.as_ref())) {
        Ok(mut docker_compose) => {
            docker_compose.trim_in_place();

            docker_compose
        },
        Err(ref error) if error.kind() == ErrorKind::NotFound => {
            return Err(anyhow!("deploy/{docker_compose_name} cannot be found in the project."));
        },
        Err(error) => return Err(error.into()),
    };

    let docker_compose = match tag_docker_compose_image(
        docker_compose.as_str(),
        image_name.as_ref(),
        commit_sha.get_short_sha(),
    )? {
        Some(docker_compose) => docker_compose,
        None => {
            return Err(anyhow!(
                "deploy/{docker_compose_name} or deploy/image-name.txt cannot match"
            ));
        },
    };

    Ok((image_name, docker_compose))
}

/// Tags every untagged `image` entry of `image_name` with `short_sha`, or returns `None` when there is no such entry.
fn tag_docker_compose_image(
    docker_compose: &str,
    image_name: &str,
    short_sha: &str,
) -> anyhow::Result<Option<String>> {
    // The image name goes into a pattern instead of being matched literally, so it has to be escaped.
    let regex = Regex::new(&format!(
        "(?m)^( *image: +{image_name}) *$",
        image_name = regex::escape(image_name)
    ))?;

    if !regex.is_match(docker_compose) {
        return Ok(None);
    }

    Ok(Some(regex.replace_all(docker_compose, format!("$1:{short_sha}")).into_owned()))
}

pub(crate) fn check_back_deploy_via_ssh<S: AsRef<str>>(
    ssh_user_host: &SshUserHost,
    ssh_root: S,
) -> anyhow::Result<()> {
    let deploy_path = format!("{ssh_root}/deploy", ssh_root = ssh_root.as_ref());

    // Both scripts are checked in one connection, which prints the name of every script that is missing.
    let mut command = create_ssh_command(
        ssh_user_host,
        format!(
            "test -f {up} || echo 'develop-up.sh'; test -f {down} || echo 'develop-down.sh'",
            up = shell_quote(format!("{deploy_path}/develop-up.sh").as_str()),
            down = shell_quote(format!("{deploy_path}/develop-down.sh").as_str()),
        ),
    );

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output()?;

    if !output.status.success() {
        log_stderr(output.stderr.as_slice(), log::Level::Error);

        return Err(anyhow!("Cannot check the deployment scripts of {ssh_user_host}"));
    }

    let missing = String::from_utf8(output.stdout)?;

    if let Some(script) = missing.split_whitespace().next() {
        return Err(anyhow!("deploy/{script} cannot be found in the project."));
    }

    Ok(())
}

/// Returns the relative path of the archive that `deploy/build.sh` should have produced and fails when it is missing.
pub(crate) fn check_build_archive(temp_dir: &TempDir, name: &str) -> anyhow::Result<String> {
    let archive_path = format!("deploy/{name}.tar.zst");

    if !temp_dir.path().join(archive_path.as_str()).is_file() {
        return Err(anyhow!("{archive_path} cannot be found after the build."));
    }

    Ok(archive_path)
}

pub(crate) fn run_front_build(temp_dir: &TempDir, target: BuildTarget) -> anyhow::Result<()> {
    log::info!("Running deploy/build.sh");

    let mut command: Command = command_args!("bash", "deploy/build.sh", target.as_ref());

    command.current_dir(temp_dir.path());

    let output = command.execute_output()?;

    if !output.status.success() {
        return Err(anyhow!("Build failed"));
    }

    Ok(())
}

pub(crate) fn run_back_build(
    temp_dir: &TempDir,
    commit_sha: &CommitSha,
    build_target: Option<&BuildTarget>,
) -> anyhow::Result<()> {
    log::info!("Running deploy/build.sh");

    let mut command: Command = command_args!("bash", "deploy/build.sh", commit_sha.get_short_sha());

    if let Some(build_target) = build_target {
        command.arg(build_target.as_ref());
    }

    command.current_dir(temp_dir.path());

    let output = command.execute_output()?;

    if !output.status.success() {
        return Err(anyhow!("Build failed"));
    }

    Ok(())
}

fn create_ssh_command_inner(
    ssh_user_host: &SshUserHost,
    command: &str,
    read_stdin: bool,
) -> Command {
    // `accept-new` trusts a host that is seen for the first time, but still refuses a host whose key has changed.
    let mut ssh: Command =
        command_args!("ssh", "-o", "StrictHostKeyChecking=accept-new", "-o", "BatchMode=yes");

    if !read_stdin {
        ssh.arg("-n");
    }

    ssh.args([
        "-p",
        ssh_user_host.get_port().to_string().as_str(),
        ssh_user_host.user_host(),
        command,
    ]);

    ssh
}

/// Creates an `ssh` command that does not read the standard input.
///
/// The standard input is inherited, so without `-n` the first host of a loop would consume all of it and leave nothing for the remaining hosts.
#[inline]
pub(crate) fn create_ssh_command<S: AsRef<str>>(
    ssh_user_host: &SshUserHost,
    command: S,
) -> Command {
    create_ssh_command_inner(ssh_user_host, command.as_ref(), false)
}

/// Creates an `ssh` command that reads the standard input, for the callers that feed data to the remote command.
#[inline]
pub(crate) fn create_ssh_command_with_stdin<S: AsRef<str>>(
    ssh_user_host: &SshUserHost,
    command: S,
) -> Command {
    create_ssh_command_inner(ssh_user_host, command.as_ref(), true)
}

/// Creates a directory on the remote host and fails when it cannot be created.
pub(crate) fn create_ssh_directory(
    ssh_user_host: &SshUserHost,
    path: &str,
    purpose: &str,
) -> anyhow::Result<()> {
    let mut command =
        create_ssh_command(ssh_user_host, format!("mkdir -p {path}", path = shell_quote(path)));

    ensure_command_success(&mut command, || {
        anyhow!("Cannot create the directory {path:?} for storing {purpose}.")
    })
}

#[inline]
pub(crate) fn create_scp_command<F: AsRef<str>, T: AsRef<str>>(
    ssh_user_host: &SshUserHost,
    from: F,
    to: T,
) -> Command {
    command_args!(
        "scp",
        "-o",
        "StrictHostKeyChecking=accept-new",
        "-o",
        "BatchMode=yes",
        "-P",
        ssh_user_host.get_port().to_string(),
        from.as_ref(),
        // Since OpenSSH 9.0 scp speaks SFTP, which takes the remote path verbatim instead of passing it through a shell, so it must not be quoted here.
        format!(
            "{ssh_user_host}:{to}",
            ssh_user_host = ssh_user_host.user_host(),
            to = to.as_ref()
        ),
    )
}

/// Copies an archive to the remote host with scp and fails when the copy does not succeed.
pub(crate) fn scp_archive(
    current_dir: &Path,
    ssh_user_host: &SshUserHost,
    from: &str,
    to: &str,
) -> anyhow::Result<()> {
    let mut command = create_scp_command(ssh_user_host, from, to);

    command.current_dir(current_dir);

    ensure_command_success(&mut command, || {
        anyhow!(
            "Cannot copy {from:?} to {ssh_user_host}:{to:?} ({ssh_user_host_port}).",
            ssh_user_host = ssh_user_host.user_host(),
            ssh_user_host_port = ssh_user_host.get_port(),
        )
    })
}

pub(crate) fn get_ssh_home(ssh_user_host: &SshUserHost) -> anyhow::Result<String> {
    let mut command = create_ssh_command(ssh_user_host, "echo $HOME");

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output()?;

    if !output.status.success() {
        log_stderr(output.stderr.as_slice(), log::Level::Error);

        return Err(anyhow!("Cannot get the home directory of {ssh_user_host}"));
    }

    let mut home = String::from_utf8(output.stdout)?;

    home.trim_in_place();

    delete_end_slash_in_place(&mut home);

    Ok(home)
}

/// Returns the directory that keeps all deployed projects under a home directory on the remote host.
#[inline]
pub(crate) fn get_project_root(ssh_home: &str) -> String {
    format!("{ssh_home}/{PROJECT_DIRECTORY}")
}

/// Returns the directory that keeps all deployed projects on the remote host.
#[inline]
pub(crate) fn get_ssh_project_root(ssh_user_host: &SshUserHost) -> anyhow::Result<String> {
    let mut ssh_root = get_ssh_home(ssh_user_host)?;

    ssh_root.push('/');
    ssh_root.push_str(PROJECT_DIRECTORY);

    Ok(ssh_root)
}

/// Returns the directory that keeps every deployment of a project on the remote host.
#[inline]
pub(crate) fn get_ssh_project_dir(ssh_root: &str, project_name: &Name, project_id: u64) -> String {
    format!("{ssh_root}/{project_name}-{project_id}", project_name = project_name.as_ref())
}

/// Returns the directory of one deployment of a project on the remote host.
#[inline]
pub(crate) fn get_ssh_project(
    ssh_project_dir: &str,
    reference_name: &Name,
    commit_sha: &CommitSha,
) -> String {
    format!(
        "{ssh_project_dir}/{reference_name}-{commit_sha}",
        reference_name = reference_name.as_ref(),
        commit_sha = commit_sha.get_short_sha(),
    )
}

/// Returns the directory that keeps every deployment of a project and the directory of this deployment on the remote host.
pub(crate) fn get_ssh_project_dirs(
    ssh_user_host: &SshUserHost,
    project_name: &Name,
    project_id: u64,
    reference_name: &Name,
    commit_sha: &CommitSha,
) -> anyhow::Result<(String, String)> {
    let ssh_project_dir = get_ssh_project_dir(
        get_ssh_project_root(ssh_user_host)?.as_str(),
        project_name,
        project_id,
    );

    let ssh_project = get_ssh_project(ssh_project_dir.as_str(), reference_name, commit_sha);

    Ok((ssh_project_dir, ssh_project))
}

pub(crate) fn list_ssh_files<S: AsRef<str>>(
    ssh_user_host: &SshUserHost,
    path: S,
) -> anyhow::Result<()> {
    let mut command =
        create_ssh_command(ssh_user_host, format!("ls {path}", path = shell_quote(path.as_ref())));

    command.stderr(Stdio::piped());

    let output = command.execute_output()?;

    if !output.status.success() {
        log_stderr(output.stderr.as_slice(), log::Level::Warn);
    }

    Ok(())
}

pub(crate) fn check_directory_exist<S: AsRef<str>>(
    ssh_user_host: &SshUserHost,
    path: S,
) -> anyhow::Result<bool> {
    let path = path.as_ref();

    let mut command =
        create_ssh_command(ssh_user_host, format!("test -d {path}", path = shell_quote(path)));

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output()?;

    if let Some(code) = output.status.code() {
        match code {
            0 => return Ok(true),
            1 => return Ok(false),
            _ => (),
        }
    }

    log_stderr(output.stderr.as_slice(), log::Level::Error);

    Err(anyhow!("Cannot check the existence of {path:?} of {ssh_user_host}"))
}

/// A `wget` startup file that carries the API token.
pub(crate) struct WgetConfig {
    // The file has to be dropped before the directory that holds it, which is the order the fields are declared in.
    file: NamedTempFile,
    _dir: TempDir,
}

impl WgetConfig {
    #[inline]
    fn path(&self) -> &Path {
        self.file.path()
    }
}

/// Creates a `wget` startup file that carries the API token, so that the token never appears in the command line where any user could read it with `ps`.
///
/// The file gets a directory of its own, so that it never sits next to the files extracted from the project. Note that `--config` replaces the default startup files, which means `~/.wgetrc` is not read at all.
fn create_wget_config(
    api_url_prefix: &ApiUrlPrefix,
    api_token: &ApiToken,
    no_check_certificate: bool,
) -> anyhow::Result<WgetConfig> {
    if !api_url_prefix.is_https() {
        log::warn!("The GitLab API URL is not HTTPS, so the API token is sent in plain text.");
    } else if no_check_certificate {
        log::warn!(
            "The TLS certificate of the GitLab server is not verified, so the API token can be \
             read by a man in the middle."
        );
    }

    let dir = tempdir()?;
    let mut file = NamedTempFile::new_in(dir.path())?;

    writeln!(file, "header = PRIVATE-TOKEN: {api_token}", api_token = api_token.as_ref())?;

    Ok(WgetConfig {
        file,
        _dir: dir,
    })
}

pub(crate) fn download_archive(
    temp_dir: &TempDir,
    api_url_prefix: &ApiUrlPrefix,
    api_token: &ApiToken,
    no_check_certificate: bool,
    project_id: u64,
    commit_sha: &CommitSha,
) -> anyhow::Result<PathBuf> {
    let archive_url = format!(
        "{api_url_prefix}/projects/{project_id}/repository/archive.tar.gz?sha={commit_sha}",
        api_url_prefix = api_url_prefix.as_ref(),
        commit_sha = commit_sha.as_ref()
    );

    let archive_save_path = temp_dir.path().join("archive.tar.gz");

    log::info!("Fetching project from {archive_url:?}");

    {
        let config = create_wget_config(api_url_prefix, api_token, no_check_certificate)?;

        let mut command =
            command_args!("wget", "--config", config.path(), archive_url, "-O", archive_save_path);

        if no_check_certificate {
            command.arg("--no-check-certificate");
        }

        ensure_command_success(&mut command, || anyhow!("Fetched unsuccessfully!"))?;

        log::info!("Fetched successfully.");
    }

    Ok(archive_save_path)
}

pub(crate) fn download_and_extract_archive(
    temp_dir: &TempDir,
    api_url_prefix: &ApiUrlPrefix,
    api_token: &ApiToken,
    no_check_certificate: bool,
    project_id: u64,
    commit_sha: &CommitSha,
) -> anyhow::Result<()> {
    let archive_url = format!(
        "{api_url_prefix}/projects/{project_id}/repository/archive?sha={commit_sha}",
        api_url_prefix = api_url_prefix.as_ref(),
        commit_sha = commit_sha.as_ref()
    );

    log::info!("Fetching project from {archive_url:?}");

    {
        let config = create_wget_config(api_url_prefix, api_token, no_check_certificate)?;

        let mut command1 = command_args!("wget", "--config", config.path(), archive_url, "-O", "-");

        if no_check_certificate {
            command1.arg("--no-check-certificate");
        }

        let mut command2: Command = command!("tar --strip-components 1 -z -x -f -");

        command2.current_dir(temp_dir.path());

        ensure_pipeline_success(&mut command1, &mut command2, || {
            anyhow!("Fetched unsuccessfully!")
        })?;

        log::info!("Fetched successfully.");
    }

    Ok(())
}

pub(crate) fn find_ssh_user_hosts(
    phase: Phase,
    project_id: u64,
) -> anyhow::Result<BTreeSet<SshUserHost>> {
    let phase_str = phase.as_ref();

    // A phase is used as a path segment, so it must not be a traversal.
    if phase_str == "." || phase_str == ".." {
        return Err(anyhow!("{phase_str:?} is not a supported phase!"));
    }

    let mut home = env::var("HOME")?;

    delete_end_slash_in_place(&mut home);

    let phase_path = Path::new(home.as_str()).join(PHASE_DIRECTORY).join(phase.as_ref());

    let file = match File::open(phase_path.as_path()) {
        Ok(f) => f,
        Err(ref err) if err.kind() == ErrorKind::NotFound => {
            return Err(anyhow!("{:?} is not a supported phase!", phase.as_ref()));
        },
        Err(err) => return Err(err.into()),
    };

    match parse_phase_file(BufReader::new(file), project_id, phase_path.as_path())? {
        Some(set) => Ok(set),
        None => Err(anyhow!("The project {project_id} is not set in {phase_path:?}")),
    }
}

/// Reads a phase file and returns the hosts of `project_id`, or `None` when the project is absent.
fn parse_phase_file<R: BufRead>(
    mut reader: R,
    project_id: u64,
    phase_path: &Path,
) -> anyhow::Result<Option<BTreeSet<SshUserHost>>> {
    let mut target: Option<BTreeSet<SshUserHost>> = None;
    // The `.` reference only looks at the previous line, so there is no need to keep every line.
    let mut last_set: Option<BTreeSet<SshUserHost>> = None;
    let mut seen_project_ids: BTreeSet<u64> = BTreeSet::new();

    let mut line_number = 0;

    let mut line = String::new();

    loop {
        line.clear();
        line_number += 1;

        let c = reader.read_line(&mut line)?;

        if c == 0 {
            break;
        }

        if let Some(index) = line.find('#') {
            line.truncate(index);
        }

        let mut sc = ScannerStr::new(&line);

        let line_project_id = match sc.next_u64() {
            Ok(r) => match r {
                Some(r) => r,
                None => continue,
            },
            Err(err) => match err {
                ScannerError::ParseIntError(_) => {
                    return Err(anyhow!(
                        "In {phase_path:?} at line {line_number}, cannot read the project id: \
                         {err:?}",
                    ));
                },
                ScannerError::IOError(err) => return Err(err.into()),
                ScannerError::ParseFloatError(_) => unreachable!(),
            },
        };

        if !seen_project_ids.insert(line_project_id) {
            return Err(anyhow!(
                "In {phase_path:?} at line {line_number}, the project {line_project_id} is \
                 duplicated",
            ));
        }

        let mut set: BTreeSet<SshUserHost> = BTreeSet::new();

        while let Some(user_host) = sc.next()? {
            if set.is_empty() && user_host == "." {
                if sc.next()?.is_some() {
                    return Err(anyhow!(
                        "In {phase_path:?} at line {line_number}, it is not correct",
                    ));
                }

                match last_set.as_ref() {
                    Some(last_set) => {
                        set.extend(last_set.iter().cloned());
                        break;
                    },
                    None => {
                        return Err(anyhow!(
                            "In {phase_path:?} at line {line_number}, should be written after the \
                             line that you want to reference",
                        ));
                    },
                }
            }

            let ssh_user_host = match SshUserHost::parse_str(user_host) {
                Ok(ssh_user_host) => ssh_user_host,
                Err(_) => {
                    return Err(anyhow!(
                        "In {phase_path:?} at line {line_number}, the format of {user_host:?} is \
                         not correct",
                    ));
                },
            };

            if !set.insert(ssh_user_host) {
                return Err(anyhow!(
                    "In {phase_path:?} at line {line_number}, {user_host:?} is duplicated",
                ));
            }
        }

        if line_project_id == project_id {
            target = Some(set.clone());
        }

        last_set = Some(set);
    }

    Ok(target)
}

#[inline]
pub(crate) fn current_timestamp() -> DelayedFormat<StrftimeItems<'static>> {
    Local::now().format("[%Y-%m-%d-%H-%M-%S]")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str, project_id: u64) -> Option<BTreeSet<SshUserHost>> {
        parse_phase_file(input.as_bytes(), project_id, Path::new("phases/test")).unwrap()
    }

    fn host(s: &str) -> SshUserHost {
        SshUserHost::parse_str(s).unwrap()
    }

    #[test]
    fn ensure_pipeline_success_fails_when_the_second_command_exits_early() {
        // The first command writes much more than a pipe buffer, so it would block forever if the read end of the pipe were still open here.
        let mut first = command_args!("head", "-c", "10000000", "/dev/zero");
        let mut second: Command = command_args!("false");

        assert!(
            ensure_pipeline_success(&mut first, &mut second, || anyhow!("Pipeline failed"))
                .is_err()
        );
    }

    #[test]
    fn shell_quote_wraps_a_plain_word() {
        assert_eq!("'/home/user/projects'", shell_quote("/home/user/projects").to_string());
    }

    #[test]
    fn shell_quote_escapes_single_quotes() {
        assert_eq!(r"'it'\''s'", shell_quote("it's").to_string());
    }

    #[test]
    fn shell_quote_keeps_shell_metacharacters_literal() {
        assert_eq!("'$(id) `id`'", shell_quote("$(id) `id`").to_string());
    }

    #[test]
    fn tag_docker_compose_image_tags_every_entry_of_the_image() {
        assert_eq!(
            Some(String::from(
                "services:\n  api:\n    image: website-api:0b14cd4f\n  worker:\n    image: \
                 website-api:0b14cd4f\n"
            )),
            tag_docker_compose_image(
                "services:\n  api:\n    image: website-api\n  worker:\n    image: website-api  \n",
                "website-api",
                "0b14cd4f"
            )
            .unwrap()
        );
    }

    #[test]
    fn tag_docker_compose_image_finds_no_entry_of_another_image() {
        assert_eq!(
            None,
            tag_docker_compose_image("    image: another-api\n", "website-api", "0b14cd4f")
                .unwrap()
        );
    }

    #[test]
    fn tag_docker_compose_image_finds_no_entry_that_only_starts_with_the_image() {
        assert_eq!(
            None,
            tag_docker_compose_image("    image: website-api-2\n", "website-api", "0b14cd4f")
                .unwrap()
        );
    }

    #[test]
    fn parse_phase_file_finds_the_hosts_of_the_project() {
        let hosts = parse("123 alice@a.example.com bob@b.example.com:2222\n", 123).unwrap();

        assert_eq!(
            BTreeSet::from([host("alice@a.example.com"), host("bob@b.example.com:2222")]),
            hosts
        );
    }

    #[test]
    fn parse_phase_file_ignores_comments_and_blank_lines() {
        let hosts =
            parse("# a comment\n\n123 alice@a.example.com # another comment\n", 123).unwrap();

        assert_eq!(BTreeSet::from([host("alice@a.example.com")]), hosts);
    }

    #[test]
    fn parse_phase_file_expands_the_dot_reference() {
        let hosts = parse("123 alice@a.example.com bob@b.example.com\n456 .\n", 456).unwrap();

        assert_eq!(BTreeSet::from([host("alice@a.example.com"), host("bob@b.example.com")]), hosts);
    }

    #[test]
    fn parse_phase_file_returns_none_for_an_absent_project() {
        assert_eq!(None, parse("123 alice@a.example.com\n", 456));
    }

    #[test]
    fn parse_phase_file_rejects_a_duplicated_project() {
        assert!(
            parse_phase_file(
                "123 alice@a.example.com\n123 bob@b.example.com\n".as_bytes(),
                123,
                Path::new("phases/test")
            )
            .is_err()
        );
    }

    #[test]
    fn find_ssh_user_hosts_rejects_a_traversal_phase() {
        assert!(find_ssh_user_hosts(Phase::parse_str("..").unwrap(), 123).is_err());
        assert!(find_ssh_user_hosts(Phase::parse_str(".").unwrap(), 123).is_err());
    }
}
