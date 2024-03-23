use std::fmt::Write;

use anyhow::anyhow;
use execute::Execute;

use crate::{
    cli::{CLIArgs, CLICommands},
    constants::*,
    functions::*,
};

pub(crate) fn simple_control(cli_args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(cli_args.command, CLICommands::SimpleControl { .. }));

    if let CLICommands::SimpleControl {
        gitlab_project_id: project_id,
        commit_sha,
        project_name,
        reference_name,
        phase,
        gitlab_api_url_prefix: _,
        gitlab_api_token: _,
        inject_project_directory,
        command,
    } = cli_args.command
    {
        check_ssh()?;

        let command_string = command.join(" ");

        let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

        if ssh_user_hosts.is_empty() {
            log::warn!("No hosts to control!");
            return Ok(());
        }

        for ssh_user_host in ssh_user_hosts.iter() {
            log::info!("Controlling to {ssh_user_host} ({command_string})");

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
                let command_in_ssh = if inject_project_directory {
                    let mut command_in_ssh =
                        String::with_capacity(command_string.len() + ssh_project.len() + 1);

                    if command[0] == "sudo" {
                        command_in_ssh.push_str("sudo ");

                        if command.len() > 1 {
                            command_in_ssh.push_str(&command[1]);
                            command_in_ssh.write_fmt(format_args!(" {ssh_project:?} "))?;
                            command_in_ssh.push_str(&command[2..].join(" "));
                        }
                    } else {
                        command_in_ssh.push_str(&command[0]);
                        command_in_ssh.write_fmt(format_args!(" {ssh_project:?} "))?;
                        command_in_ssh.push_str(&command[1..].join(" "));
                    }

                    command_in_ssh
                } else {
                    command_string.clone()
                };

                let mut command = create_ssh_command(ssh_user_host, command_in_ssh);

                let output = command.execute_output()?;

                if !output.status.success() {
                    return Err(anyhow!("Control failed!"));
                }
            }

            {
                let mut command = create_ssh_command(
                    ssh_user_host,
                    format!(
                        "cd {ssh_project:?} && echo \"{timestamp} {command_string:?} \
                         {reference_name}-{commit_sha}\" >> {ssh_project:?}/../control.log",
                        timestamp = current_timestamp(),
                        reference_name = reference_name.as_ref(),
                        commit_sha = commit_sha.get_short_sha(),
                    ),
                );

                let output = command.execute_output()?;

                if !output.status.success() {
                    return Err(anyhow!("Control failed!"));
                }
            }
        }

        log::info!("Successfully!");
    }

    Ok(())
}
