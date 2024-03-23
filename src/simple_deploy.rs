use std::{fmt::Write, fs::File};

use anyhow::anyhow;
use execute::Execute;
use tempfile::tempdir;

use crate::{
    cli::{CLIArgs, CLICommands},
    constants::*,
    functions::*,
};

pub(crate) fn simple_deploy(cli_args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(cli_args.command, CLICommands::SimpleDeploy { .. }));

    if let CLICommands::SimpleDeploy {
        gitlab_project_id: project_id,
        commit_sha,
        project_name,
        reference_name,
        phase,
        gitlab_api_url_prefix: api_url_prefix,
        gitlab_api_token: api_token,
    } = cli_args.command
    {
        check_ssh()?;
        check_wget()?;
        check_docker()?;

        let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

        if ssh_user_hosts.is_empty() {
            log::warn!("No hosts to deploy!");
            return Ok(());
        }

        let temp_dir = tempdir()?;

        let archive_file_path =
            download_archive(&temp_dir, api_url_prefix, api_token, project_id, &commit_sha)?;

        for ssh_user_host in ssh_user_hosts.iter() {
            log::info!("Deploying to {ssh_user_host}");

            let ssh_root = {
                let mut ssh_home = get_ssh_home(ssh_user_host)?;

                ssh_home.write_fmt(format_args!("/{PROJECT_DIRECTORY}",))?;

                ssh_home
            };

            let ssh_project = format!(
                "{ssh_root}/{project_name}-{project_id}/{reference_name}-{commit_sha}",
                project_name = project_name.as_ref(),
                reference_name = reference_name.as_ref(),
                commit_sha = commit_sha.get_short_sha(),
            );

            {
                let mut command =
                    create_ssh_command(ssh_user_host, format!("mkdir -p {ssh_project:?}"));

                let status = command.execute()?;

                if let Some(0) = status {
                    // do nothing
                } else {
                    return Err(anyhow!(
                        "Cannot create the directory {ssh_project:?} for storing the project \
                         files.",
                    ));
                }
            }

            log::info!("Unpacking the archive file");

            {
                let mut command = create_ssh_command(
                    ssh_user_host,
                    format!("tar --strip-components 1 -x -v -f - -C {ssh_project:?}"),
                );

                let status =
                    command.execute_input_reader(&mut File::open(archive_file_path.as_path())?)?;

                if let Some(0) = status {
                    // do nothing
                } else {
                    return Err(anyhow!("Cannot deploy the project"));
                }
            }
        }

        log::info!("Successfully!");
    }

    Ok(())
}
