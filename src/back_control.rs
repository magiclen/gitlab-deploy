use std::{fmt::Write, process::Stdio};

use anyhow::anyhow;
use execute::Execute;
use trim_in_place::TrimInPlace;

use crate::{
    cli::{CLIArgs, CLICommands},
    constants::*,
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
        check_ssh()?;

        let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

        if ssh_user_hosts.is_empty() {
            log::warn!("No hosts to control!");
            return Ok(());
        }

        for ssh_user_host in ssh_user_hosts.iter() {
            log::info!("Controlling to {ssh_user_host} ({command})", command = command.as_str());

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

            let command_str = command.get_command_str();

            if command == Command::DownAndUp {
                let mut command =
                    create_ssh_command(ssh_user_host, format!("cat {ssh_project:?}/../last-up"));

                command.stdout(Stdio::piped());
                command.stderr(Stdio::piped());

                let output = command.execute_output()?;

                if output.status.success() {
                    let mut folder = String::from_utf8(output.stdout)?;

                    folder.trim_in_place();

                    log::info!("Trying to shut down {folder} first");

                    {
                        let mut command = create_ssh_command(
                            ssh_user_host,
                            format!(
                                "cd {ssh_project:?}/../{folder} && {command}",
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
                let mut command = create_ssh_command(
                    ssh_user_host,
                    format!(
                        "cd {ssh_project:?} && echo \"{timestamp} {command} \
                         {reference_name}-{commit_sha}\" >> {ssh_project:?}/../control.log && \
                         {command_str}",
                        reference_name = reference_name.as_ref(),
                        timestamp = current_timestamp(),
                        commit_sha = commit_sha.get_short_sha(),
                        command = command.as_str(),
                    ),
                );

                let output = command.execute_output()?;

                if !output.status.success() {
                    return Err(anyhow!("Control failed!"));
                }
            }

            if matches!(command, Command::Up | Command::DownAndUp) {
                let mut command = create_ssh_command(
                    ssh_user_host,
                    format!(
                        "cd {ssh_project:?} && echo \"{reference_name}-{commit_sha}\" > \
                         {ssh_project:?}/../last-up",
                        reference_name = reference_name.as_ref(),
                        commit_sha = commit_sha.get_short_sha(),
                    ),
                );

                let status = command.execute()?;

                if let Some(0) = status {
                    // do nothing
                } else {
                    log::warn!("The latest version information cannot be written");
                }
            }
        }

        log::info!("Successfully!");
    }

    Ok(())
}
