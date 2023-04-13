use std::{error::Error, fmt::Write as FmtWrite};

use clap::{ArgMatches, Values};
use execute::Execute;

use crate::{constants::*, functions::*, parse::*};

#[inline]
fn handle_command(values: Option<Values>) -> Result<Vec<&str>, &'static str> {
    match values {
        Some(values) => Ok(values.collect()),
        None => Err("A command is needed."),
    }
}

pub(crate) fn simple_control(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    check_ssh()?;

    let project_id = parse_parse_id(matches);

    let commit_sha = parse_commit_sha(matches);

    let project_name = parse_project_name(matches);

    let reference_name = parse_reference_name(matches);

    let phase = parse_phase(matches);

    let command = handle_command(matches.values_of("COMMAND"))?;
    let command_string: String = command.join(" ");

    let inject_project_directory = matches.is_present("INJECT_PROJECT_DIRECTORY");

    let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

    if ssh_user_hosts.is_empty() {
        warn!("No hosts to control!");
        return Ok(());
    }

    for ssh_user_host in ssh_user_hosts.iter() {
        info!("Controlling to {} ({})", ssh_user_host, command_string);

        let ssh_root = {
            let mut ssh_home = get_ssh_home(ssh_user_host)?;

            ssh_home.write_fmt(format_args!(
                "/{PROJECT_DIRECTORY}",
                PROJECT_DIRECTORY = PROJECT_DIRECTORY,
            ))?;

            ssh_home
        };

        let ssh_project = format!(
            "{SSH_ROOT}/{PROJECT_NAME}-{PROJECT_ID}/{REFERENCE_NAME}-{SHORT_SHA}",
            SSH_ROOT = ssh_root,
            PROJECT_NAME = project_name.as_ref(),
            PROJECT_ID = project_id,
            REFERENCE_NAME = reference_name.as_ref(),
            SHORT_SHA = commit_sha.get_short_sha(),
        );

        {
            let command_in_ssh = if inject_project_directory {
                let mut command_in_ssh =
                    String::with_capacity(command_string.len() + ssh_project.len() + 1);

                if command[0] == "sudo" {
                    command_in_ssh.push_str("sudo ");

                    if command.len() > 1 {
                        command_in_ssh.push_str(command[1]);
                        command_in_ssh.write_fmt(format_args!(" {:?} ", ssh_project))?;
                        command_in_ssh.push_str(&command[2..].join(" "));
                    }
                } else {
                    command_in_ssh.push_str(command[0]);
                    command_in_ssh.write_fmt(format_args!(" {:?} ", ssh_project))?;
                    command_in_ssh.push_str(&command[1..].join(" "));
                }

                command_in_ssh
            } else {
                command_string.clone()
            };

            let mut command = create_ssh_command(ssh_user_host, command_in_ssh);

            let output = command.execute_output()?;

            if !output.status.success() {
                return Err("Control failed!".into());
            }
        }

        {
            let mut command = create_ssh_command(
                ssh_user_host,
                format!(
                    "cd {SSH_PROJECT:?} && echo \"{TIMESTAMP} {COMMAND:?} \
                     {REFERENCE_NAME}-{SHORT_SHA}\" >> {SSH_PROJECT:?}/../control.log",
                    SSH_PROJECT = ssh_project,
                    REFERENCE_NAME = reference_name.as_ref(),
                    TIMESTAMP = current_timestamp(),
                    SHORT_SHA = commit_sha.get_short_sha(),
                    COMMAND = command_string,
                ),
            );

            let output = command.execute_output()?;

            if !output.status.success() {
                return Err("Control failed!".into());
            }
        }
    }

    info!("Successfully!");

    Ok(())
}
