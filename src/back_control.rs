use std::process::Stdio;

use anyhow::anyhow;
use execute::Execute;
use trim_in_place::TrimInPlace;

use crate::{
    cli::{CLIArgs, CLICommands},
    functions::*,
    models::*,
};

pub(crate) fn back_control(cli_args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(cli_args.command, CLICommands::BackendControl { .. }));

    if let CLICommands::BackendControl {
        gitlab_project_id: project_id,
        commit_sha,
        project_name,
        reference_name,
        phase,
        command,
    } = cli_args.command
    {
        check_command("ssh", "-V")?;

        let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

        if ssh_user_hosts.is_empty() {
            log::warn!("No hosts to control!");
            return Ok(());
        }

        for ssh_user_host in ssh_user_hosts.iter() {
            log::info!("Controlling to {ssh_user_host} ({command})", command = command.as_str());

            let ssh_project_dir = format!(
                "{ssh_root}/{project_name}-{project_id}",
                ssh_root = get_ssh_project_root(ssh_user_host)?,
                project_name = project_name.as_ref(),
            );

            let ssh_project = format!(
                "{ssh_project_dir}/{reference_name}-{commit_sha}",
                reference_name = reference_name.as_ref(),
                commit_sha = commit_sha.get_short_sha(),
            );

            let ssh_control_log = format!("{ssh_project_dir}/control.log");
            let ssh_last_up = format!("{ssh_project_dir}/last-up");

            let command_str = command.get_command_str();

            if command == Command::DownAndUp {
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
                let log_message = format!(
                    "{timestamp} {command} {reference_name}-{commit_sha}",
                    timestamp = current_timestamp(),
                    command = command.as_str(),
                    reference_name = reference_name.as_ref(),
                    commit_sha = commit_sha.get_short_sha(),
                );

                let mut command = create_ssh_command(
                    ssh_user_host,
                    format!(
                        "cd {ssh_project} && echo {log_message} >> {ssh_control_log} && \
                         {command_str}",
                        ssh_project = shell_quote(ssh_project.as_str()),
                        log_message = shell_quote(log_message.as_str()),
                        ssh_control_log = shell_quote(ssh_control_log.as_str()),
                    ),
                );

                let output = command.execute_output()?;

                if !output.status.success() {
                    return Err(anyhow!("Control failed!"));
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

                if command.execute()? != Some(0) {
                    log::warn!("The latest version information cannot be written");
                }
            }
        }

        log::info!("Successfully!");
    }

    Ok(())
}
