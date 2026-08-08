use std::fs::File;

use anyhow::anyhow;
use execute::Execute;
use tempfile::tempdir;

use crate::{
    cli::{CLIArgs, CLICommands},
    functions::*,
};

pub(crate) fn simple_deploy(cli_args: CLIArgs) -> anyhow::Result<()> {
    let CLICommands::SimpleDeploy {
        gitlab_project_id: project_id,
        commit_sha,
        project_name,
        reference_name,
        phase,
        gitlab_api_url_prefix: api_url_prefix,
        gitlab_api_token: api_token,
    } = cli_args.command
    else {
        unreachable!();
    };

    check_command("ssh", "-V")?;
    check_command("wget", "--version")?;

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

        let ssh_project_dir = get_ssh_project_dir(
            get_ssh_project_root(ssh_user_host)?.as_str(),
            &project_name,
            project_id,
        );

        let ssh_project = get_ssh_project(ssh_project_dir.as_str(), &reference_name, &commit_sha);

        {
            let mut command = create_ssh_command(
                ssh_user_host,
                format!("mkdir -p {ssh_project}", ssh_project = shell_quote(ssh_project.as_str())),
            );

            ensure_command_success(&mut command, || {
                anyhow!(
                    "Cannot create the directory {ssh_project:?} for storing the project files."
                )
            })?;
        }

        log::info!("Unpacking the archive file");

        {
            let mut command = create_ssh_command(
                ssh_user_host,
                format!(
                    "tar --strip-components 1 -z -x -v -f - -C {ssh_project}",
                    ssh_project = shell_quote(ssh_project.as_str())
                ),
            );

            let status = command
                .execute_input_reader_output(&mut File::open(archive_file_path.as_path())?)?
                .status
                .code();

            ensure_exit_success(status, || anyhow!("Cannot deploy the project"))?;
        }
    }

    log::info!("Successfully!");

    Ok(())
}
