use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    env,
    fs::{self, File},
    io::{BufRead, BufReader, ErrorKind},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::anyhow;
use chrono::{
    format::{DelayedFormat, StrftimeItems},
    Local,
};
use execute::{command, command_args, Execute};
use regex::Regex;
use scanner_rust::{ScannerError, ScannerStr};
use slash_formatter::delete_end_slash_in_place;
use tempfile::TempDir;
use trim_in_place::TrimInPlace;
use validators::prelude::*;

use crate::{constants::*, models::*};

#[inline]
pub(crate) fn check_zstd() -> anyhow::Result<()> {
    let mut command = command!("zstd --version");

    if command.execute_check_exit_status_code(0).is_err() {
        return Err(anyhow!("Cannot find zstd."));
    }

    Ok(())
}

#[inline]
pub(crate) fn check_ssh() -> anyhow::Result<()> {
    // scp should also be checked implicitly
    let mut command = command!("ssh -V");

    if command.execute_check_exit_status_code(0).is_err() {
        return Err(anyhow!("Cannot find ssh."));
    }

    Ok(())
}

#[inline]
pub(crate) fn check_wget() -> anyhow::Result<()> {
    let mut command = command!("wget --version");

    if command.execute_check_exit_status_code(0).is_err() {
        return Err(anyhow!("Cannot find wget."));
    }

    Ok(())
}

#[inline]
pub(crate) fn check_tar() -> anyhow::Result<()> {
    let mut command = command!("tar --version");

    if command.execute_check_exit_status_code(0).is_err() {
        return Err(anyhow!("Cannot find tar."));
    }

    Ok(())
}

#[inline]
pub(crate) fn check_bash() -> anyhow::Result<()> {
    let mut command = command!("bash --version");

    if command.execute_check_exit_status_code(0).is_err() {
        return Err(anyhow!("Cannot find bash."));
    }

    Ok(())
}

#[inline]
pub(crate) fn check_docker() -> anyhow::Result<()> {
    let mut command = command!("docker --version");

    if command.execute_check_exit_status_code(0).is_err() {
        return Err(anyhow!("Cannot find docker."));
    }

    Ok(())
}

pub(crate) fn check_front_deploy(temp_dir: &TempDir) -> anyhow::Result<Name> {
    let deploy_dir = temp_dir.path().join("deploy");

    if !deploy_dir.join("build.sh").is_file() {
        return Err(anyhow!("deploy/build.sh cannot be found in the project."));
    }

    let public_name = match fs::read_to_string(deploy_dir.join("public-name.txt")) {
        Ok(mut public_name) => {
            public_name.trim_in_place();

            match Name::parse_string(public_name) {
                Ok(public_name) => public_name,
                Err(_) => {
                    return Err(anyhow!("deploy/public-name.txt is not correct"));
                },
            }
        },
        Err(ref error) if error.kind() == ErrorKind::NotFound => {
            return Err(anyhow!("deploy/public-name.txt cannot be found in the project."));
        },
        Err(error) => return Err(error.into()),
    };

    Ok(public_name)
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

    let image_name = match fs::read_to_string(deploy_dir.join("image-name.txt")) {
        Ok(mut image_name) => {
            image_name.trim_in_place();

            match ImageName::parse_string(image_name) {
                Ok(image_name) => image_name,
                Err(_) => {
                    return Err(anyhow!("deploy/image-name.txt is not correct"));
                },
            }
        },
        Err(ref error) if error.kind() == ErrorKind::NotFound => {
            return Err(anyhow!("deploy/image-name.txt cannot be found in the project."));
        },
        Err(error) => return Err(error.into()),
    };

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

    let regex =
        Regex::new(&format!("(?m)^( *image: +{image_name}) *$", image_name = image_name.as_ref()))
            .unwrap();

    if !regex.is_match(docker_compose.as_str()) {
        return Err(anyhow!("deploy/{docker_compose_name} or deploy/image-name.txt cannot match"));
    }

    let docker_compose = regex
        .replace_all(
            docker_compose.as_str(),
            format!("$1:{commit_sha}", commit_sha = commit_sha.get_short_sha()),
        )
        .into_owned();

    Ok((image_name, docker_compose))
}

pub(crate) fn check_back_deploy_via_ssh<S: AsRef<str>>(
    ssh_user_host: &SshUserHost,
    ssh_root: S,
) -> anyhow::Result<()> {
    let deploy_path = format!("{ssh_root}/deploy", ssh_root = ssh_root.as_ref());

    if !check_file_exist(ssh_user_host, format!("{deploy_path}/develop-up.sh"))? {
        return Err(anyhow!("deploy/develop-up.sh cannot be found in the project."));
    }

    if !check_file_exist(ssh_user_host, format!("{deploy_path}/develop-down.sh"))? {
        return Err(anyhow!("deploy/develop-down.sh cannot be found in the project."));
    }

    Ok(())
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

#[inline]
pub(crate) fn create_ssh_command<S: AsRef<str>>(
    ssh_user_host: &SshUserHost,
    command: S,
) -> Command {
    command_args!(
        "ssh",
        "-o",
        "StrictHostKeyChecking=no",
        "-o",
        "BatchMode=yes",
        "-p",
        ssh_user_host.get_port().to_string(),
        ssh_user_host.user_host(),
        command.as_ref()
    )
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
        "StrictHostKeyChecking=no",
        "-o",
        "BatchMode=yes",
        "-P",
        ssh_user_host.get_port().to_string(),
        from.as_ref(),
        format!(
            "{ssh_user_host}:{to}",
            ssh_user_host = ssh_user_host.user_host(),
            to = to.as_ref()
        ),
    )
}

pub(crate) fn get_ssh_home(ssh_user_host: &SshUserHost) -> anyhow::Result<String> {
    let mut command = create_ssh_command(ssh_user_host, "echo $HOME");

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output()?;

    if !output.status.success() {
        String::from_utf8_lossy(output.stderr.as_slice()).split('\n').for_each(|line| {
            if !line.is_empty() {
                log::error!("{line}");
            }
        });

        return Err(anyhow!("Cannot get the home directory of {ssh_user_host}"));
    }

    let mut home = String::from_utf8(output.stdout)?;

    home.trim_in_place();

    delete_end_slash_in_place(&mut home);

    Ok(home)
}

pub(crate) fn list_ssh_files<S: AsRef<str>>(
    ssh_user_host: &SshUserHost,
    path: S,
) -> anyhow::Result<()> {
    let mut command =
        create_ssh_command(ssh_user_host, format!("ls {path:?}", path = path.as_ref(),));

    command.stderr(Stdio::piped());

    let output = command.execute_output()?;

    if !output.status.success() {
        String::from_utf8_lossy(output.stderr.as_slice()).split('\n').for_each(|line| {
            if !line.is_empty() {
                log::warn!("{line}");
            }
        });
    }

    Ok(())
}

pub(crate) fn check_file_exist<S: AsRef<str>>(
    ssh_user_host: &SshUserHost,
    path: S,
) -> anyhow::Result<bool> {
    let mut command =
        create_ssh_command(ssh_user_host, format!("test -f {PATH:?}", PATH = path.as_ref(),));

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

    String::from_utf8_lossy(output.stderr.as_slice()).split('\n').for_each(|line| {
        if !line.is_empty() {
            log::error!("{}", line);
        }
    });

    Err(anyhow!("Cannot check the existence of {:?} of {}", path.as_ref(), ssh_user_host))
}

pub(crate) fn check_directory_exist<S: AsRef<str>>(
    ssh_user_host: &SshUserHost,
    path: S,
) -> anyhow::Result<bool> {
    let mut command =
        create_ssh_command(ssh_user_host, format!("test -d {PATH:?}", PATH = path.as_ref(),));

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

    String::from_utf8_lossy(output.stderr.as_slice()).split('\n').for_each(|line| {
        if !line.is_empty() {
            log::error!("{}", line);
        }
    });

    Err(anyhow!("Cannot check the existence of {:?} of {}", path.as_ref(), ssh_user_host))
}

pub(crate) fn download_archive(
    temp_dir: &TempDir,
    api_url_prefix: ApiUrlPrefix,
    api_token: ApiToken,
    project_id: u64,
    commit_sha: &CommitSha,
) -> anyhow::Result<PathBuf> {
    let archive_url = format!(
        "{api_url_prefix}/projects/{project_id}/repository/archive.tar?sha={commit_sha}",
        api_url_prefix = api_url_prefix.as_ref(),
        commit_sha = commit_sha.as_ref()
    );

    let archive_save_path = temp_dir.path().join("archive.tar");

    log::info!("Fetching project from {archive_url:?}");

    {
        let mut command = command_args!(
            "wget",
            "--no-check-certificate",
            archive_url,
            "--header",
            format!("PRIVATE-TOKEN: {api_token}", api_token = api_token.as_ref()),
            "-O",
            archive_save_path,
        );

        let output = command.execute()?;

        if let Some(0) = output {
            log::info!("Fetched successfully.");
        } else {
            return Err(anyhow!("Fetched unsuccessfully!"));
        }
    }

    Ok(archive_save_path)
}

pub(crate) fn download_and_extract_archive(
    temp_dir: &TempDir,
    api_url_prefix: ApiUrlPrefix,
    api_token: ApiToken,
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
        let mut command1 = command_args!(
            "wget",
            "--no-check-certificate",
            archive_url,
            "--header",
            format!("PRIVATE-TOKEN: {api_token}", api_token = api_token.as_ref()),
            "-O",
            "-",
        );

        let mut command2: Command = command!("tar --strip-components 1 -z -x -v -f -");

        command2.current_dir(temp_dir.path());

        let output = command1.execute_multiple(&mut [&mut command2])?;

        if let Some(0) = output {
            log::info!("Fetched successfully.");
        } else {
            return Err(anyhow!("Fetched unsuccessfully!"));
        }
    }

    Ok(())
}

pub(crate) fn find_ssh_user_hosts(
    phase: Phase,
    project_id: u64,
) -> anyhow::Result<HashSet<SshUserHost>> {
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

    let mut reader = BufReader::new(file);

    let mut map: HashMap<u64, HashSet<SshUserHost>> = HashMap::new();

    let mut line_number = 0;

    let mut line = String::new();

    let mut last_project_id: Option<u64> = None;

    loop {
        line.clear();
        line_number += 1;

        let c = reader.read_line(&mut line)?;

        if c == 0 {
            break;
        }

        if let Some(index) = line.find('#') {
            unsafe {
                line.as_mut_vec().set_len(index);
            }
        }

        let mut sc = ScannerStr::new(&line);

        let project_id = match sc.next_u64() {
            Ok(r) => match r {
                Some(r) => r,
                None => continue,
            },
            Err(err) => match err {
                ScannerError::ParseIntError(_) => {
                    return Err(anyhow!(
                        "In {phase_path:?} at line {line_number}, cannot read the project id: \
                         {err:?}",
                    ))
                },
                ScannerError::IOError(err) => return Err(err.into()),
                ScannerError::ParseFloatError(_) => unreachable!(),
            },
        };

        let mut set: HashSet<SshUserHost> = HashSet::with_capacity(1);

        while let Some(user_host) = sc.next()? {
            if set.is_empty() && user_host == "." {
                if sc.next()?.is_some() {
                    return Err(anyhow!(
                        "In {phase_path:?} at line {line_number}, it is not correct",
                    ));
                }

                match last_project_id {
                    Some(last_project_id) => {
                        set.extend(map.get(&last_project_id).unwrap().iter().cloned());
                        break;
                    },
                    None => {
                        return Err(anyhow!(
                            "In {phase_path:?} at line {line_number}, should be written after the \
                             line that you want to reference",
                        ))
                    },
                }
            }

            let ssh_user_host = match SshUserHost::parse_str(user_host) {
                Ok(ssh_user_host) => ssh_user_host,
                Err(_) => {
                    return Err(anyhow!(
                        "In {phase_path:?} at line {line_number}, the format of {user_host:?} is \
                         not correct",
                    ))
                },
            };

            if !set.insert(ssh_user_host) {
                return Err(anyhow!(
                    "In {phase_path:?} at line {line_number}, {user_host:?} is duplicated",
                ));
            }
        }

        map.insert(project_id, set);
        last_project_id = Some(project_id);
    }

    if let Some(set) = map.remove(&project_id) {
        Ok(set)
    } else {
        Err(anyhow!("The project {project_id} is not set in {phase_path:?}",))
    }
}

#[inline]
pub(crate) fn current_timestamp() -> DelayedFormat<StrftimeItems<'static>> {
    Local::now().format("[%Y-%m-%d-%H-%M-%S]")
}
