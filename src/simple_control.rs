use anyhow::anyhow;
use execute::Execute;

use crate::{
    cli::{CLIArgs, CLICommands},
    functions::*,
};

/// Builds the command line to run on the remote host, optionally with the project directory injected as the first argument.
fn build_remote_command(
    command: &[String],
    ssh_project: &str,
    inject_project_directory: bool,
) -> anyhow::Result<String> {
    // Every argument is quoted so that the remote shell keeps the arguments exactly as they are given here.
    let mut arguments: Vec<String> =
        command.iter().map(|argument| shell_quote(argument).to_string()).collect();

    if inject_project_directory {
        // The project directory is injected right after the program name, which `sudo` is not.
        let program_index = usize::from(command[0] == "sudo");

        if command.len() <= program_index {
            return Err(anyhow!(
                "--inject-project-directory needs a program to run after {program:?}",
                program = command[0],
            ));
        }

        arguments.insert(program_index + 1, shell_quote(ssh_project).to_string());
    }

    Ok(arguments.join(" "))
}

pub(crate) fn simple_control(cli_args: CLIArgs) -> anyhow::Result<()> {
    let CLICommands::SimpleControl {
        gitlab_project_id: project_id,
        commit_sha,
        project_name,
        reference_name,
        phase,
        inject_project_directory,
        command,
    } = cli_args.command
    else {
        unreachable!();
    };

    check_command("ssh", "-V")?;

    let command_string = command.join(" ");

    let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

    if ssh_user_hosts.is_empty() {
        log::warn!("No hosts to control!");
        return Ok(());
    }

    for ssh_user_host in ssh_user_hosts.iter() {
        log::info!("Controlling to {ssh_user_host} ({command_string})");

        let ssh_project_dir = get_ssh_project_dir(
            get_ssh_project_root(ssh_user_host)?.as_str(),
            &project_name,
            project_id,
        );

        let ssh_project = get_ssh_project(ssh_project_dir.as_str(), &reference_name, &commit_sha);

        {
            let command_in_ssh = build_remote_command(
                command.as_slice(),
                ssh_project.as_str(),
                inject_project_directory,
            )?;

            let mut command = create_ssh_command(ssh_user_host, command_in_ssh);

            ensure_command_success(&mut command, || anyhow!("Control failed!"))?;
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

            // The control command itself has already succeeded, so a log that cannot be written should not fail the whole run.
            if !output.status.success() {
                log::warn!("The control log {ssh_control_log:?} cannot be written");
            }
        }
    }

    log::info!("Successfully!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SSH_PROJECT: &str = "/home/deploy/projects/website-123/pre-release-0b14cd4f";

    fn command<const N: usize>(command: [&str; N]) -> Vec<String> {
        command.iter().map(|argument| String::from(*argument)).collect()
    }

    #[test]
    fn build_remote_command_quotes_every_argument() {
        assert_eq!(
            "'/usr/local/bin/apply-nginx.sh' 'dev.env'",
            build_remote_command(
                command(["/usr/local/bin/apply-nginx.sh", "dev.env"]).as_slice(),
                SSH_PROJECT,
                false
            )
            .unwrap()
        );
    }

    #[test]
    fn build_remote_command_injects_the_project_directory() {
        assert_eq!(
            format!("'/usr/local/bin/apply-nginx.sh' '{SSH_PROJECT}' 'dev.env'"),
            build_remote_command(
                command(["/usr/local/bin/apply-nginx.sh", "dev.env"]).as_slice(),
                SSH_PROJECT,
                true
            )
            .unwrap()
        );
    }

    #[test]
    fn build_remote_command_injects_the_project_directory_after_sudo() {
        assert_eq!(
            format!("'sudo' '/usr/local/bin/apply-nginx.sh' '{SSH_PROJECT}' 'dev.env'"),
            build_remote_command(
                command(["sudo", "/usr/local/bin/apply-nginx.sh", "dev.env"]).as_slice(),
                SSH_PROJECT,
                true
            )
            .unwrap()
        );
    }
}
