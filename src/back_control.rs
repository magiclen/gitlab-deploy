use std::{error::Error, fmt::Write as FmtWrite, process::Stdio};

use clap::ArgMatches;
use execute::Execute;
use trim_in_place::TrimInPlace;

use crate::{constants::*, functions::*, parse::*};

pub(crate) fn back_control(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    check_ssh()?;

    let project_id = parse_parse_id(matches);

    let commit_sha = parse_commit_sha(matches);

    let project_name = parse_project_name(matches);

    let reference_name = parse_reference_name(matches);

    let phase = parse_phase(matches);

    let command = parse_command(matches);

    let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

    if ssh_user_hosts.is_empty() {
        warn!("No hosts to control!");
        return Ok(());
    }

    for ssh_user_host in ssh_user_hosts.iter() {
        info!("Controlling to {} ({})", ssh_user_host, command.as_str());

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

        let command_str = command.get_command_str();

        if command == Command::DownAndUp {
            let mut command = create_ssh_command(
                ssh_user_host,
                format!("cat {SSH_PROJECT:?}/../last-up", SSH_PROJECT = ssh_project,),
            );

            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());

            let output = command.execute_output()?;

            if output.status.success() {
                let mut folder = String::from_utf8(output.stdout)?;

                folder.trim_in_place();

                info!("Trying to shut down {} first", folder);

                {
                    let mut command = create_ssh_command(
                        ssh_user_host,
                        format!(
                            "cd {SSH_PROJECT:?}/../{FOLDER} && {COMMAND}",
                            SSH_PROJECT = ssh_project,
                            FOLDER = folder,
                            COMMAND = Command::Down.get_command_str(),
                        ),
                    );

                    let output = command.execute_output()?;

                    if !output.status.success() {
                        warn!("{} cannot be fully shut down", folder);
                    }
                }
            }
        }

        {
            let mut command = create_ssh_command(
                ssh_user_host,
                format!(
                    "cd {SSH_PROJECT:?} && echo \"{TIMESTAMP} {COMMAND} \
                     {REFERENCE_NAME}-{SHORT_SHA}\" >> {SSH_PROJECT:?}/../control.log && \
                     {COMMAND_STR}",
                    SSH_PROJECT = ssh_project,
                    REFERENCE_NAME = reference_name.as_ref(),
                    TIMESTAMP = current_timestamp(),
                    SHORT_SHA = commit_sha.get_short_sha(),
                    COMMAND = command.as_str(),
                    COMMAND_STR = command_str,
                ),
            );

            let output = command.execute_output()?;

            if !output.status.success() {
                return Err("Control failed!".into());
            }
        }

        if matches!(command, Command::Up | Command::DownAndUp) {
            let mut command = create_ssh_command(
                ssh_user_host,
                format!(
                    "cd {SSH_PROJECT:?} && echo \"{REFERENCE_NAME}-{SHORT_SHA}\" > \
                     {SSH_PROJECT:?}/../last-up",
                    SSH_PROJECT = ssh_project,
                    REFERENCE_NAME = reference_name.as_ref(),
                    SHORT_SHA = commit_sha.get_short_sha(),
                ),
            );

            let status = command.execute()?;

            if let Some(0) = status {
                // do nothing
            } else {
                warn!("The latest version information cannot be written");
            }
        }
    }

    info!("Successfully!");

    Ok(())
}
