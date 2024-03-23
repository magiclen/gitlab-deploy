use std::{path::PathBuf, process::Stdio};

use anyhow::anyhow;
use execute::Execute;
use trim_in_place::TrimInPlace;

use crate::{
    cli::{CLIArgs, CLICommands},
    constants::*,
    functions::*,
};

pub(crate) fn front_control(cli_args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(cli_args.command, CLICommands::FrontendControl { .. }));

    if let CLICommands::FrontendControl {
        gitlab_project_id: project_id,
        commit_sha,
        project_name,
        reference_name,
        phase,
    } = cli_args.command
    {
        check_ssh()?;

        let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

        if ssh_user_hosts.is_empty() {
            log::warn!("No hosts to control!");
            return Ok(());
        }

        for ssh_user_host in ssh_user_hosts.iter() {
            log::info!("Controlling to {ssh_user_host} (apply)");

            let ssh_home = get_ssh_home(ssh_user_host)?;

            let ssh_root = format!("{ssh_home}/{PROJECT_DIRECTORY}");

            let ssh_project = format!(
                "{ssh_root}/{project_name}-{project_id}/{reference_name}-{commit_sha}",
                project_name = project_name.as_ref(),
                reference_name = reference_name.as_ref(),
                commit_sha = commit_sha.get_short_sha(),
            );

            let tarball_path = {
                let mut command = create_ssh_command(
                    ssh_user_host,
                    format!(
                        "find {ssh_project:?} -mindepth 1 -maxdepth 1 -iname '*.tar.zst' | head -1"
                    ),
                );

                command.stdout(Stdio::piped());
                command.stderr(Stdio::piped());

                let output = command.execute_output()?;

                if output.status.success() {
                    let mut files = String::from_utf8(output.stdout)?;

                    files.trim_in_place();

                    if files.is_empty() {
                        return Err(anyhow!(
                            "The archive file cannot be found in the project {ssh_project:?}",
                        ));
                    }

                    PathBuf::from(files)
                } else {
                    String::from_utf8_lossy(output.stderr.as_slice()).split('\n').for_each(
                        |line| {
                            if !line.is_empty() {
                                log::error!("{line}");
                            }
                        },
                    );

                    return Err(anyhow!(
                        "The archive file cannot be found in the project {ssh_project:?}"
                    ));
                }
            };

            let tarball = tarball_path.file_name().unwrap().to_string_lossy();
            let public_name = tarball.strip_suffix(".tar.zst").unwrap();

            let ssh_html_path = format!("{ssh_home}/{SERVICE_DIRECTORY}/www/{public_name}/html");

            {
                let mut command = create_ssh_command(
                    ssh_user_host,
                    format!(
                        "cd {ssh_project:?} && (([ ! -d public ] && mkdir public) || true) && \
                         (zstd -T0 -d -c {tarball:?} | tar -xf - -C public) && mkdir -p \
                         {ssh_html_path:?} && (([ -d {ssh_html_path:?} ] && rm -r \
                         {ssh_html_path:?}) || true) && cp -r public {ssh_html_path:?} && rm -r \
                         public && echo \"{timestamp} apply {reference_name}-{commit_sha}\" >> \
                         {ssh_project:?}/../control.log",
                        reference_name = reference_name.as_ref(),
                        timestamp = current_timestamp(),
                        commit_sha = commit_sha.get_short_sha(),
                    ),
                );

                let status = command.execute()?;

                if let Some(0) = status {
                    // do nothing
                } else {
                    return Err(anyhow!("Cannot apply the project"));
                }
            }

            log::info!("Listing the public static files...");

            list_ssh_files(ssh_user_host, ssh_html_path)?;
        }

        log::info!("Successfully!");
    }

    Ok(())
}
