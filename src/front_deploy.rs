use std::fmt::Write as FmtWrite;

use anyhow::anyhow;
use execute::Execute;
use tempfile::tempdir;

use crate::{
    cli::{CLIArgs, CLICommands},
    constants::*,
    functions::*,
};

pub(crate) fn front_deploy(cli_args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(cli_args.command, CLICommands::FrontendDeploy { .. }));

    if let CLICommands::FrontendDeploy {
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

        let public_name = check_front_deploy(&temp_dir)?;

        run_front_build(&temp_dir, build_target)?;

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

            let tarball_path =
                format!("deploy/{public_name}.tar.zst", public_name = public_name.as_ref());

            let ssh_tarball_path =
                format!("{ssh_project}/{public_name}.tar.zst", public_name = public_name.as_ref());

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
        }

        log::info!("Successfully!");
    }

    Ok(())
}
