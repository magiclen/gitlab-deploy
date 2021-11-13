use std::error::Error;
use std::fs;
use std::io::ErrorKind;
use std::process::Command;

use crate::tempfile::TempDir;

use crate::execute::Execute;

use crate::trim_in_place::TrimInPlace;

use crate::validators::prelude::*;

use crate::parse::*;

pub(crate) fn check_zstd() -> Result<(), Box<dyn Error>> {
    let mut command = command!("zstd --version");

    if command.execute_check_exit_status_code(0).is_err() {
        return Err("Cannot find zstd.".into());
    }

    Ok(())
}

pub(crate) fn check_ssh() -> Result<(), Box<dyn Error>> {
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

pub(crate) fn check_front_deploy(temp_dir: &TempDir) -> Result<PublicName, Box<dyn Error>> {
    let deploy_dir = temp_dir.path().join("deploy");

    if !deploy_dir.join("build.sh").is_file() {
        return Err("deploy/build.sh cannot be found in the project.".into());
    }

    let public_name = match fs::read_to_string(deploy_dir.join("public-name.txt")) {
        Ok(mut public_name) => {
            public_name.trim_in_place();

            match PublicName::parse_string(public_name) {
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

pub(crate) fn run_front_build<T: AsRef<str>>(
    temp_dir: &TempDir,
    target: T,
) -> Result<(), Box<dyn Error>> {
    let mut command: Command = command_args!("bash",
        "deploy/build.sh",
        target.as_ref(),
    );

    command.current_dir(temp_dir.path());

    let mut child = command.spawn()?;

    let result = child.wait()?;

    if !result.success() {
        return Err("Build failed".into());
    }

    Ok(())
}

pub(crate) fn download_and_extract_archive<S: AsRef<str>, T: AsRef<str>, SHA: AsRef<str>>(
    temp_dir: &TempDir,
    api_url_prefix: S,
    api_token: T,
    project_id: u64,
    commit_sha: SHA,
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
