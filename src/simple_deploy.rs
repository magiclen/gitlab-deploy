use std::fs::File;

use anyhow::anyhow;
use execute::Execute;
use tempfile::tempdir;

use crate::{cli::SimpleDeployArgs, functions::*};

pub(crate) fn simple_deploy(args: SimpleDeployArgs) -> anyhow::Result<()> {
    let SimpleDeployArgs {
        gitlab_project_id: project_id,
        commit_sha,
        project_name,
        reference_name,
        phase,
        gitlab_api_url_prefix: api_url_prefix,
        gitlab_api_token: api_token,
    } = args;

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

        let (_, ssh_project) = get_ssh_project_dirs(
            ssh_user_host,
            &project_name,
            project_id,
            &reference_name,
            &commit_sha,
        )?;

        create_ssh_directory(ssh_user_host, ssh_project.as_str(), "the project files")?;

        log::info!("Unpacking the archive file");

        {
            let mut command = create_ssh_command_with_stdin(
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
