use anyhow::anyhow;
use execute::Execute;

use crate::{
    cli::{CLIArgs, CLICommands},
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
        check_command("ssh", "-V")?;

        let command_string = command.join(" ");

        let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

        if ssh_user_hosts.is_empty() {
            log::warn!("No hosts to control!");
            return Ok(());
        }

        for ssh_user_host in ssh_user_hosts.iter() {
            log::info!("Controlling to {ssh_user_host} ({command_string})");

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

            {
                let command_in_ssh = if inject_project_directory {
                    // The project directory is injected right after the program name, which `sudo` is not.
                    let program_index = usize::from(command[0] == "sudo");

                    if command.len() <= program_index {
                        return Err(anyhow!(
                            "--inject-project-directory needs a program to run after {program:?}",
                            program = command[0],
                        ));
                    }

                    format!(
                        "{prefix}{program} {ssh_project} {arguments}",
                        prefix = if program_index == 0 { "" } else { "sudo " },
                        program = command[program_index],
                        ssh_project = shell_quote(ssh_project.as_str()),
                        arguments = command[(program_index + 1)..].join(" "),
                    )
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
                let ssh_control_log = format!("{ssh_project_dir}/control.log");

                let log_message = format!(
                    "{timestamp} {command_string:?} {reference_name}-{commit_sha}",
                    timestamp = current_timestamp(),
                    reference_name = reference_name.as_ref(),
                    commit_sha = commit_sha.get_short_sha(),
                );

                let mut command = create_ssh_command(
                    ssh_user_host,
                    format!(
                        "cd {ssh_project} && echo {log_message} >> {ssh_control_log}",
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
        }

        log::info!("Successfully!");
    }

    Ok(())
}
