use std::process::Stdio;

use anyhow::anyhow;
use execute::Execute;
use trim_in_place::TrimInPlace;

use crate::{cli::BackendControlArgs, functions::*, models::*};

pub(crate) fn back_control(args: BackendControlArgs) -> anyhow::Result<()> {
    let BackendControlArgs {
        gitlab_project_id: project_id,
        commit_sha,
        project_name,
        reference_name,
        phase,
        command,
    } = args;

    check_command("ssh", "-V")?;

    let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

    if ssh_user_hosts.is_empty() {
        log::warn!("No hosts to control!");
        return Ok(());
    }

    for ssh_user_host in ssh_user_hosts.iter() {
        log::info!("Controlling to {ssh_user_host} ({command})", command = command.as_str());

        let (ssh_project_dir, ssh_project) = get_ssh_project_dirs(
            ssh_user_host,
            &project_name,
            project_id,
            &reference_name,
            &commit_sha,
        )?;

        let ssh_control_log = format!("{ssh_project_dir}/control.log");
        let ssh_last_up = format!("{ssh_project_dir}/last-up");

        let command_str = command.get_command_str();

        if matches!(command, Command::DownAndUp) {
            let mut command = create_ssh_command(
                ssh_user_host,
                format!("cat {ssh_last_up}", ssh_last_up = shell_quote(ssh_last_up.as_str())),
            );

            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());

            let output = command.execute_output()?;

            if output.status.success() {
                let mut folder = String::from_utf8(output.stdout)?;

                folder.trim_in_place();

                log::info!("Trying to shut down {folder} first");

                {
                    let ssh_last_up_project = format!("{ssh_project_dir}/{folder}");

                    let mut command = create_ssh_command(
                        ssh_user_host,
                        format!(
                            "cd {ssh_last_up_project} && {command}",
                            ssh_last_up_project = shell_quote(ssh_last_up_project.as_str()),
                            command = Command::Down.get_command_str(),
                        ),
                    );

                    let output = command.execute_output()?;

                    if !output.status.success() {
                        log::warn!("{folder} cannot be fully shut down");
                    }
                }
            }
        }

        {
            let mut ssh_command = create_ssh_command(
                ssh_user_host,
                format!(
                    "cd {ssh_project} && {command_str}",
                    ssh_project = shell_quote(ssh_project.as_str()),
                ),
            );

            ensure_command_success(&mut ssh_command, || anyhow!("Control failed!"))?;
        }

        {
            let log_message = format!(
                "{timestamp} {command} {reference_name}-{commit_sha}",
                timestamp = current_timestamp(),
                command = command.as_str(),
                reference_name = reference_name.as_ref(),
                commit_sha = commit_sha.get_short_sha(),
            );

            let mut ssh_command = create_ssh_command(
                ssh_user_host,
                format!(
                    "cd {ssh_project} && echo {log_message} >> {ssh_control_log}",
                    ssh_project = shell_quote(ssh_project.as_str()),
                    log_message = shell_quote(log_message.as_str()),
                    ssh_control_log = shell_quote(ssh_control_log.as_str()),
                ),
            );

            let output = ssh_command.execute_output()?;

            // The control command itself has already succeeded, so a log that cannot be written should not fail the whole run.
            if !output.status.success() {
                log::warn!("The control log {ssh_control_log:?} cannot be written");
            }
        }

        if matches!(command, Command::Up | Command::DownAndUp) {
            let last_up = format!(
                "{reference_name}-{commit_sha}",
                reference_name = reference_name.as_ref(),
                commit_sha = commit_sha.get_short_sha(),
            );

            let mut command = create_ssh_command(
                ssh_user_host,
                format!(
                    "cd {ssh_project} && echo {last_up} > {ssh_last_up}",
                    ssh_project = shell_quote(ssh_project.as_str()),
                    last_up = shell_quote(last_up.as_str()),
                    ssh_last_up = shell_quote(ssh_last_up.as_str()),
                ),
            );

            let output = command.execute_output()?;

            // The control command itself has already succeeded, so a version file that cannot be written should not fail the whole run.
            if !output.status.success() {
                log::warn!("The latest version information cannot be written");
            }
        }
    }

    log::info!("Successfully!");

    Ok(())
}
