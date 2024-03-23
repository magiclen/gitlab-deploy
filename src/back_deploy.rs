use std::fmt::Write as FmtWrite;

use anyhow::anyhow;
use execute::{command_args, Execute};
use tempfile::tempdir;

use crate::{
    cli::{CLIArgs, CLICommands},
    constants::*,
    functions::*,
};

pub(crate) fn back_deploy(cli_args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(cli_args.command, CLICommands::BackendDeploy { .. }));

    if let CLICommands::BackendDeploy {
        gitlab_project_id: project_id,
        commit_sha,
        project_name,
        reference_name,
        build_target,
        phase,
        gitlab_api_url_prefix: api_url_prefix,
        gitlab_api_token: api_token,
    } = cli_args.command
    {
        check_zstd()?;
        check_ssh()?;
        check_wget()?;
        check_tar()?;
        check_bash()?;
        check_docker()?;

        let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

        if ssh_user_hosts.is_empty() {
            log::warn!("No hosts to deploy!");
            return Ok(());
        }

        let temp_dir = tempdir()?;

        download_and_extract_archive(
            &temp_dir,
            api_url_prefix,
            api_token,
            project_id,
            &commit_sha,
        )?;

        let (image_name, docker_compose) =
            check_back_deploy(&temp_dir, &commit_sha, build_target.as_ref())?;

        run_back_build(&temp_dir, &commit_sha, build_target.as_ref())?;

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
                        "Cannot create the directory {ssh_project:?} for storing the archive of \
                         public static files."
                    ));
                }
            }

            let ssh_docker_compose_path = format!("{ssh_project}/docker-compose.yml");

            {
                let mut command =
                    create_ssh_command(ssh_user_host, format!("cat - > {ssh_docker_compose_path}"));

                let status = command.execute_input(docker_compose.as_str())?;

                if let Some(0) = status {
                    // do nothing
                } else {
                    return Err(anyhow!(
                        "Cannot create the docker compose file {ssh_docker_compose_path:?}."
                    ));
                }
            }

            let tarball_path =
                format!("deploy/{image_name}.tar.zst", image_name = image_name.as_ref());

            let ssh_tarball_path =
                format!("{ssh_project}/{image_name}.tar.zst", image_name = image_name.as_ref());

            {
                let mut command = create_scp_command(
                    ssh_user_host,
                    tarball_path.as_str(),
                    ssh_tarball_path.as_str(),
                );

                command.current_dir(temp_dir.path());

                let status = command.execute()?;

                if let Some(0) = status {
                    // do nothing
                } else {
                    return Err(anyhow!(
                        "Cannot copy {tarball_path:?} to {ssh_user_host}:{ssh_tarball_path:?} \
                         ({ssh_user_host_port}).",
                        ssh_user_host = ssh_user_host.user_host(),
                        ssh_user_host_port = ssh_user_host.get_port(),
                    ));
                }
            }

            log::info!("Extracting {tarball_path}");

            {
                let mut command1 = command_args!("zstd", "-d", "-c", "-f", tarball_path);

                command1.current_dir(temp_dir.path());

                let mut command2 = create_ssh_command(ssh_user_host, "docker image load");

                let status = command1.execute_multiple(&mut [&mut command2])?;

                if let Some(0) = status {
                    // do nothing
                } else {
                    return Err(anyhow!("Cannot deploy the docker image"));
                }
            }
        }

        log::info!("Successfully!");
    }

    Ok(())
}
