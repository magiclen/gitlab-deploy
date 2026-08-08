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
    let CLICommands::FrontendControl {
        gitlab_project_id: project_id,
        commit_sha,
        project_name,
        reference_name,
        phase,
    } = cli_args.command
    else {
        unreachable!();
    };

    check_command("ssh", "-V")?;

    let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

    if ssh_user_hosts.is_empty() {
        log::warn!("No hosts to control!");
        return Ok(());
    }

    for ssh_user_host in ssh_user_hosts.iter() {
        log::info!("Controlling to {ssh_user_host} (apply)");

        let ssh_home = get_ssh_home(ssh_user_host)?;

        let ssh_project_dir = get_ssh_project_dir(
            format!("{ssh_home}/{PROJECT_DIRECTORY}").as_str(),
            &project_name,
            project_id,
        );

        let ssh_project = get_ssh_project(ssh_project_dir.as_str(), &reference_name, &commit_sha);

        let tarball_path = {
            let mut command = create_ssh_command(
                ssh_user_host,
                format!(
                    "find {ssh_project} -mindepth 1 -maxdepth 1 -name '*.tar.zst' | head -1",
                    ssh_project = shell_quote(ssh_project.as_str()),
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
                log_stderr(output.stderr.as_slice(), log::Level::Error);

                return Err(anyhow!(
                    "The archive file cannot be found in the project {ssh_project:?}"
                ));
            }
        };

        let tarball = match tarball_path.file_name() {
            Some(tarball) => tarball.to_string_lossy(),
            None => {
                return Err(anyhow!("{tarball_path:?} is not a correct archive file"));
            },
        };

        let public_name = match tarball.strip_suffix(".tar.zst") {
            Some(public_name) => public_name,
            None => {
                return Err(anyhow!("{tarball_path:?} is not a correct archive file"));
            },
        };

        let ssh_www_path = format!("{ssh_home}/{SERVICE_DIRECTORY}/www/{public_name}");
        let ssh_html_path = format!("{ssh_www_path}/html");

        {
            let ssh_control_log = format!("{ssh_project_dir}/control.log");

            let log_message = format!(
                "{timestamp} apply {reference_name}-{commit_sha}",
                timestamp = current_timestamp(),
                reference_name = reference_name.as_ref(),
                commit_sha = commit_sha.get_short_sha(),
            );

            let mut command = create_ssh_command(
                ssh_user_host,
                format!(
                    "cd {ssh_project} && mkdir -p public && (zstd -T0 -d -c {tarball} | tar -xf - \
                     -C public) && mkdir -p {ssh_www_path} && ((test -d {ssh_html_path} && rm -r \
                     {ssh_html_path}) || true) && cp -r public {ssh_html_path} && rm -r public && \
                     echo {log_message} >> {ssh_control_log}",
                    ssh_project = shell_quote(ssh_project.as_str()),
                    tarball = shell_quote(tarball.as_ref()),
                    ssh_www_path = shell_quote(ssh_www_path.as_str()),
                    ssh_html_path = shell_quote(ssh_html_path.as_str()),
                    log_message = shell_quote(log_message.as_str()),
                    ssh_control_log = shell_quote(ssh_control_log.as_str()),
                ),
            );

            ensure_command_success(&mut command, || anyhow!("Cannot apply the project"))?;
        }

        log::info!("Listing the public static files...");

        list_ssh_files(ssh_user_host, ssh_html_path)?;
    }

    log::info!("Successfully!");

    Ok(())
}
