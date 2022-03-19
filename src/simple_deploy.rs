use std::error::Error;
use std::fmt::Write as FmtWrite;
use std::fs::File;

use execute::Execute;

use clap::ArgMatches;

use tempfile::tempdir;

use crate::constants::*;
use crate::functions::*;
use crate::parse::*;

pub(crate) fn simple_deploy(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    check_ssh()?;
    check_wget()?;
    check_docker()?;

    let project_id = parse_parse_id(matches);

    let commit_sha = parse_commit_sha(matches);

    let project_name = parse_project_name(matches);

    let reference_name = parse_reference_name(matches);

    let phase = parse_phase(matches);

    let api_url_prefix = parse_api_url_prefix(matches);

    let api_token = parse_api_token(matches);

    let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

    if ssh_user_hosts.is_empty() {
        warn!("No hosts to deploy!");
        return Ok(());
    }

    let temp_dir = tempdir()?;

    let archive_file_path =
        download_archive(&temp_dir, api_url_prefix, api_token, project_id, &commit_sha)?;

    for ssh_user_host in ssh_user_hosts.iter() {
        info!("Deploying to {}", ssh_user_host);

        let ssh_root = {
            let mut ssh_home = get_ssh_home(ssh_user_host)?;

            ssh_home.write_fmt(format_args!(
                "/{PROJECT_DIRECTORY}",
                PROJECT_DIRECTORY = PROJECT_DIRECTORY,
            ))?;

            ssh_home
        };

        let ssh_project = format!(
            "{SSH_ROOT}/{PROJECT_NAME}-{PROJECT_ID}/{REFERENCE_NAME}-{SHORT_SHA}",
            SSH_ROOT = ssh_root,
            PROJECT_NAME = project_name.as_ref(),
            PROJECT_ID = project_id,
            REFERENCE_NAME = reference_name.as_ref(),
            SHORT_SHA = commit_sha.get_short_sha(),
        );

        {
            let mut command = create_ssh_command(
                ssh_user_host,
                format!("mkdir -p {SSH_PROJECT:?}", SSH_PROJECT = ssh_project),
            );

            let status = command.execute()?;

            if let Some(0) = status {
                // do nothing
            } else {
                return Err(format!(
                    "Cannot create the directory {:?} for storing the project files.",
                    ssh_project
                )
                .into());
            }
        }

        info!("Unpacking the archive file");

        {
            let mut command = create_ssh_command(
                ssh_user_host,
                format!(
                    "tar --strip-components 1 -x -v -f - -C {SSH_PROJECT:?}",
                    SSH_PROJECT = ssh_project
                ),
            );

            let status =
                command.execute_input_reader(&mut File::open(archive_file_path.as_path())?)?;

            if let Some(0) = status {
                // do nothing
            } else {
                return Err("Cannot deploy the project".into());
            }
        }
    }

    info!("Successfully!");

    Ok(())
}
