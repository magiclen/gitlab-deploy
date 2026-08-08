use anyhow::anyhow;
use execute::{Execute, command_args};
use tempfile::tempdir;

use crate::{
    cli::{CLIArgs, CLICommands},
    functions::*,
};

pub(crate) fn back_deploy(cli_args: CLIArgs) -> anyhow::Result<()> {
    let CLICommands::BackendDeploy {
        gitlab_project_id: project_id,
        commit_sha,
        project_name,
        reference_name,
        build_target,
        phase,
        gitlab_api_url_prefix: api_url_prefix,
        gitlab_api_token: api_token,
    } = cli_args.command
    else {
        unreachable!();
    };

    check_command("zstd", "--version")?;
    // scp comes with ssh, so it is checked implicitly
    check_command("ssh", "-V")?;
    check_command("wget", "--version")?;
    check_command("tar", "--version")?;
    check_command("bash", "--version")?;
    check_command("docker", "--version")?;

    let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

    if ssh_user_hosts.is_empty() {
        log::warn!("No hosts to deploy!");
        return Ok(());
    }

    let temp_dir = tempdir()?;

    download_and_extract_archive(&temp_dir, api_url_prefix, api_token, project_id, &commit_sha)?;

    let (image_name, docker_compose) =
        check_back_deploy(&temp_dir, &commit_sha, build_target.as_ref())?;

    run_back_build(&temp_dir, &commit_sha, build_target.as_ref())?;

    let tarball_path = check_build_archive(&temp_dir, image_name.as_ref())?;

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
                    "Cannot create the directory {ssh_project:?} for storing the archive of \
                     public static files."
                )
            })?;
        }

        let ssh_docker_compose_path = format!("{ssh_project}/docker-compose.yml");

        {
            let mut command = create_ssh_command(
                ssh_user_host,
                format!(
                    "cat - > {ssh_docker_compose_path}",
                    ssh_docker_compose_path = shell_quote(ssh_docker_compose_path.as_str())
                ),
            );

            let status = command.execute_input_output(docker_compose.as_str())?.status.code();

            ensure_exit_success(status, || {
                anyhow!("Cannot create the docker compose file {ssh_docker_compose_path:?}.")
            })?;
        }

        let ssh_tarball_path =
            format!("{ssh_project}/{image_name}.tar.zst", image_name = image_name.as_ref());

        {
            let mut command =
                create_scp_command(ssh_user_host, tarball_path.as_str(), ssh_tarball_path.as_str());

            command.current_dir(temp_dir.path());

            ensure_command_success(&mut command, || {
                anyhow!(
                    "Cannot copy {tarball_path:?} to {ssh_user_host}:{ssh_tarball_path:?} \
                     ({ssh_user_host_port}).",
                    ssh_user_host = ssh_user_host.user_host(),
                    ssh_user_host_port = ssh_user_host.get_port(),
                )
            })?;
        }

        log::info!("Extracting {tarball_path}");

        {
            let mut command1 = command_args!("zstd", "-d", "-c", "-f", tarball_path.as_str());

            command1.current_dir(temp_dir.path());

            let mut command2 = create_ssh_command(ssh_user_host, "docker image load");

            ensure_pipeline_success(&mut command1, &mut command2, || {
                anyhow!("Cannot deploy the docker image")
            })?;
        }
    }

    log::info!("Successfully!");

    Ok(())
}
