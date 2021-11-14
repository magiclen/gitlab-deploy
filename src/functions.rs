use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::tempfile::TempDir;

use crate::execute::Execute;

use crate::regex::Regex;
use crate::scanner_rust::{ScannerError, ScannerStr};
use crate::slash_formatter::delete_end_slash_in_place;
use crate::trim_in_place::TrimInPlace;

use crate::validators::prelude::*;

use crate::constants::*;
use crate::parse::*;

pub(crate) fn check_zstd() -> Result<(), Box<dyn Error>> {
    let mut command = command!("zstd --version");

    if command.execute_check_exit_status_code(0).is_err() {
        return Err("Cannot find zstd.".into());
    }

    Ok(())
}

pub(crate) fn check_ssh() -> Result<(), Box<dyn Error>> {
    // scp should also be checked implicitly
    let mut command = command!("ssh -V");

    if command.execute_check_exit_status_code(0).is_err() {
        return Err("Cannot find ssh.".into());
    }

    Ok(())
}

pub(crate) fn check_wget() -> Result<(), Box<dyn Error>> {
    let mut command = command!("wget --version");

    if command.execute_check_exit_status_code(0).is_err() {
        return Err("Cannot find wget.".into());
    }

    Ok(())
}

pub(crate) fn check_tar() -> Result<(), Box<dyn Error>> {
    let mut command = command!("tar --version");

    if command.execute_check_exit_status_code(0).is_err() {
        return Err("Cannot find tar.".into());
    }

    Ok(())
}

pub(crate) fn check_bash() -> Result<(), Box<dyn Error>> {
    let mut command = command!("bash --version");

    if command.execute_check_exit_status_code(0).is_err() {
        return Err("Cannot find bash.".into());
    }

    Ok(())
}

pub(crate) fn check_front_deploy(temp_dir: &TempDir) -> Result<Name, Box<dyn Error>> {
    let deploy_dir = temp_dir.path().join("deploy");

    if !deploy_dir.join("build.sh").is_file() {
        return Err("deploy/build.sh cannot be found in the project.".into());
    }

    let public_name = match fs::read_to_string(deploy_dir.join("public-name.txt")) {
        Ok(mut public_name) => {
            public_name.trim_in_place();

            match Name::parse_string(public_name) {
                Ok(public_name) => public_name,
                Err(_) => {
                    return Err("deploy/public-name.txt is not correct".into());
                }
            }
        }
        Err(ref error) if error.kind() == ErrorKind::NotFound => {
            return Err("deploy/public-name.txt cannot be found in the project.".into());
        }
        Err(error) => return Err(error.into()),
    };

    Ok(public_name)
}

pub(crate) fn check_back_deploy_via_ssh<S: AsRef<str>>(
    ssh_user_host: &SshUserHost,
    ssh_root: S,
) -> Result<ImageName, Box<dyn Error>> {
    let deploy_path = format!("{}/deploy", ssh_root.as_ref());

    if !check_file_exist(ssh_user_host, format!("{}/build.sh", deploy_path.as_str()))? {
        return Err("deploy/build.sh cannot be found in the project.".into());
    }

    if !check_file_exist(ssh_user_host, format!("{}/develop-up.sh", deploy_path.as_str()))? {
        return Err("deploy/develop-up.sh cannot be found in the project.".into());
    }

    if !check_file_exist(ssh_user_host, format!("{}/develop-down.sh", deploy_path.as_str()))? {
        return Err("deploy/develop-down.sh cannot be found in the project.".into());
    }

    let image_name = {
        let mut command = create_ssh_command(
            ssh_user_host,
            format!("cat {DEPLOY_PATH:?}/image-name.txt", DEPLOY_PATH = deploy_path),
        );

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let output = command.execute_output()?;

        match output.status.code().unwrap_or(-1) {
            0 => {
                let mut image_name = String::from_utf8(output.stdout)?;

                image_name.trim_in_place();

                match ImageName::parse_string(image_name) {
                    Ok(image_name) => image_name,
                    Err(_) => {
                        return Err("deploy/image-name.txt is not correct".into());
                    }
                }
            }
            1 => return Err("deploy/image-name.txt cannot be found in the project.".into()),
            _ => {
                String::from_utf8_lossy(output.stderr.as_slice()).split('\n').for_each(|line| {
                    if !line.is_empty() {
                        error!("{}", line);
                    }
                });

                return Err("deploy/image-name.txt cannot be found in the project.".into());
            }
        }
    };

    let docker_compose = {
        let mut command = create_ssh_command(
            ssh_user_host,
            format!("cat {DEPLOY_PATH:?}/docker-compose.yml", DEPLOY_PATH = deploy_path),
        );

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let output = command.execute_output()?;

        match output.status.code().unwrap_or(-1) {
            0 => {
                let mut docker_compose = String::from_utf8(output.stdout)?;

                docker_compose.trim_in_place();

                docker_compose
            }
            1 => return Err("deploy/docker-compose.yml cannot be found in the project.".into()),
            _ => {
                String::from_utf8_lossy(output.stderr.as_slice()).split('\n').for_each(|line| {
                    if !line.is_empty() {
                        error!("{}", line);
                    }
                });

                return Err("deploy/docker-compose.yml cannot be found in the project.".into());
            }
        }
    };

    let regex =
        Regex::new(&format!("(?m)^ *image: +{IMAGE_NAME} *$", IMAGE_NAME = image_name.as_ref()))
            .unwrap();

    if !regex.is_match(docker_compose.as_str()) {
        return Err("deploy/docker-compose.yml or deploy/image-name.txt cannot match".into());
    }

    Ok(image_name)
}

pub(crate) fn run_front_build(
    temp_dir: &TempDir,
    target: BuildTarget,
) -> Result<(), Box<dyn Error>> {
    info!("Running deploy/build.sh");

    let mut command: Command = command_args!("bash", "deploy/build.sh", target.as_ref());

    command.current_dir(temp_dir.path());

    let output = command.execute_output()?;

    if !output.status.success() {
        return Err("Build failed".into());
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
        format!("{}:{}", ssh_user_host.user_host(), to.as_ref()),
    )
}

pub(crate) fn get_ssh_home(ssh_user_host: &SshUserHost) -> Result<String, Box<dyn Error>> {
    let mut command = create_ssh_command(ssh_user_host, "echo $HOME");

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output()?;

    if !output.status.success() {
        String::from_utf8_lossy(output.stderr.as_slice()).split('\n').for_each(|line| {
            if !line.is_empty() {
                error!("{}", line);
            }
        });

        return Err(format!("Cannot get the home directory of {}", ssh_user_host).into());
    }

    let mut home = String::from_utf8(output.stdout)?;

    home.trim_in_place();

    delete_end_slash_in_place(&mut home);

    Ok(home)
}

pub(crate) fn list_ssh_files<S: AsRef<str>>(
    ssh_user_host: &SshUserHost,
    path: S,
) -> Result<(), Box<dyn Error>> {
    let mut command =
        create_ssh_command(ssh_user_host, format!("ls {PATH:?}", PATH = path.as_ref(),));

    command.stderr(Stdio::piped());

    let output = command.execute_output()?;

    if !output.status.success() {
        String::from_utf8_lossy(output.stderr.as_slice()).split('\n').for_each(|line| {
            if !line.is_empty() {
                warn!("{}", line);
            }
        });
    }

    Ok(())
}

pub(crate) fn check_file_exist<S: AsRef<str>>(
    ssh_user_host: &SshUserHost,
    path: S,
) -> Result<bool, Box<dyn Error>> {
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
            error!("{}", line);
        }
    });

    Err(format!("Cannot check the existence of {:?} of {}", path.as_ref(), ssh_user_host).into())
}

pub(crate) fn check_directory_exist<S: AsRef<str>>(
    ssh_user_host: &SshUserHost,
    path: S,
) -> Result<bool, Box<dyn Error>> {
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
            error!("{}", line);
        }
    });

    Err(format!("Cannot check the existence of {:?} of {}", path.as_ref(), ssh_user_host).into())
}

pub(crate) fn download_and_extract_archive(
    temp_dir: &TempDir,
    api_url_prefix: ApiUrlPrefix,
    api_token: ApiToken,
    project_id: u64,
    commit_sha: &CommitSha,
) -> Result<(), Box<dyn Error>> {
    let archive_url = format!(
        "{GITLAB_API_URL_PREFIX}/projects/{PROJECT_ID}/repository/archive?sha={COMMIT_SHA}",
        GITLAB_API_URL_PREFIX = api_url_prefix.as_ref(),
        PROJECT_ID = project_id,
        COMMIT_SHA = commit_sha.as_ref()
    );

    info!("Fetching project from {:?}", archive_url);

    {
        let mut command1 = command_args!(
            "wget",
            "--no-check-certificate",
            archive_url,
            "--header",
            format!("PRIVATE-TOKEN: {GITLAB_API_TOKEN}", GITLAB_API_TOKEN = api_token.as_ref()),
            "-O",
            "-",
        );

        let mut command2: Command = command!("tar --strip-components 1 -z -x -v -f -");

        command2.current_dir(temp_dir.path());

        let output = command1.execute_multiple(&mut [&mut command2])?;

        if let Some(0) = output {
            info!("Fetched successfully.");
        } else {
            return Err("Fetched unsuccessfully!".into());
        }
    }

    Ok(())
}

pub(crate) fn find_ssh_user_hosts(
    phase: Phase,
    project_id: u64,
) -> Result<HashSet<SshUserHost>, Box<dyn Error>> {
    let mut home = env::var("HOME")?;

    delete_end_slash_in_place(&mut home);

    let phase_path = Path::new(home.as_str()).join(PHASE_DIRECTORY).join(phase.as_ref());

    let file = match File::open(phase_path.as_path()) {
        Ok(f) => f,
        Err(ref err) if err.kind() == ErrorKind::NotFound => {
            return Err(format!("{:?} is not a supported phase!", phase.as_ref()).into());
        }
        Err(err) => return Err(err.into()),
    };

    let mut reader = BufReader::new(file);

    let mut map: HashMap<u64, HashSet<SshUserHost>> = HashMap::new();

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
            unsafe {
                line.as_mut_vec().set_len(index);
            }
        }

        let mut sc = ScannerStr::new(&line);

        let project_id = match sc.next_u64() {
            Ok(r) => {
                match r {
                    Some(r) => r,
                    None => continue,
                }
            }
            Err(err) => {
                match err {
                    ScannerError::ParseIntError(_) => {
                        return Err(format!(
                            "In {PHASE_PATH:?} at line {LINE}, cannot read the project id: {}",
                            err,
                            PHASE_PATH = phase_path,
                            LINE = line_number
                        )
                        .into())
                    }
                    ScannerError::IOError(err) => return Err(err.into()),
                    ScannerError::ParseFloatError(_) => unreachable!(),
                }
            }
        };

        let mut set: HashSet<SshUserHost> = HashSet::with_capacity(1);

        while let Some(user_host) = sc.next()? {
            let ssh_user_host = match SshUserHost::parse_str(user_host) {
                Ok(ssh_user_host) => ssh_user_host,
                Err(_) => {
                    return Err(format!(
                    "In {PHASE_PATH:?} at line {LINE}, the format of {USER_HOST:?} is not correct",
                    PHASE_PATH = phase_path,
                    LINE = line_number,
                    USER_HOST = user_host
                )
                    .into())
                }
            };

            if !set.insert(ssh_user_host) {
                return Err(format!(
                    "In {PHASE_PATH:?} at line {LINE}, {USER_HOST:?} is duplicated",
                    PHASE_PATH = phase_path,
                    LINE = line_number,
                    USER_HOST = user_host
                )
                .into());
            }
        }

        map.insert(project_id, set);
    }

    if let Some(set) = map.remove(&project_id) {
        Ok(set)
    } else {
        Err(format!(
            "The project {PROJECT_ID} is not set in {PHASE_PATH:?}",
            PROJECT_ID = project_id,
            PHASE_PATH = phase_path
        )
        .into())
    }
}
