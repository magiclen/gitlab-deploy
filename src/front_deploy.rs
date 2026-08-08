use anyhow::anyhow;
use tempfile::tempdir;

use crate::{
    cli::{CLIArgs, CLICommands},
    functions::*,
};

pub(crate) fn front_deploy(cli_args: CLIArgs) -> anyhow::Result<()> {
    let CLICommands::FrontendDeploy {
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

    let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

    if ssh_user_hosts.is_empty() {
        log::warn!("No hosts to deploy!");
        return Ok(());
    }

    let temp_dir = tempdir()?;

    download_and_extract_archive(&temp_dir, api_url_prefix, api_token, project_id, &commit_sha)?;

    let public_name = check_front_deploy(&temp_dir)?;

    run_front_build(&temp_dir, build_target)?;

    let tarball_path = check_build_archive(&temp_dir, public_name.as_ref())?;

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

        let ssh_tarball_path =
            format!("{ssh_project}/{public_name}.tar.zst", public_name = public_name.as_ref());

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
    }

    log::info!("Successfully!");

    Ok(())
}
