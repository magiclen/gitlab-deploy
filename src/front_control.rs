use std::error::Error;
use std::path::PathBuf;
use std::process::Stdio;

use execute::Execute;

use crate::clap::ArgMatches;

use crate::trim_in_place::TrimInPlace;

use crate::constants::*;
use crate::functions::*;
use crate::parse::*;

pub(crate) fn front_control(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    check_ssh()?;

    let project_id = parse_parse_id(matches);

    let commit_sha = parse_commit_sha(matches);

    let project_name = parse_project_name(matches);

    let reference_name = parse_reference_name(matches);

    let phase = parse_phase(matches);

    let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

    if ssh_user_hosts.is_empty() {
        warn!("No hosts to control!");
        return Ok(());
    }

    for ssh_user_host in ssh_user_hosts.iter() {
        info!("Controlling to {}", ssh_user_host);

        let ssh_home = get_ssh_home(ssh_user_host)?;

        let ssh_root = format!(
            "{SSH_HOME}/{PROJECT_DIRECTORY}",
            SSH_HOME = ssh_home,
            PROJECT_DIRECTORY = PROJECT_DIRECTORY
        );

        let ssh_project = format!(
            "{SSH_ROOT}/{PROJECT_NAME}-{PROJECT_ID}/{REFERENCE_NAME}-{SHORT_SHA}",
            SSH_ROOT = ssh_root,
            PROJECT_NAME = project_name.as_ref(),
            PROJECT_ID = project_id,
            REFERENCE_NAME = reference_name.as_ref(),
            SHORT_SHA = commit_sha.get_short_sha(),
        );

        let tarball_path = {
            let mut command = create_ssh_command(
                ssh_user_host,
                format!(
                    "find {SSH_PROJECT:?} -mindepth 1 -maxdepth 1 -iname '*.tar.zst' | head -1",
                    SSH_PROJECT = ssh_project
                ),
            );

            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());

            let output = command.execute_output()?;

            if output.status.success() {
                let mut files = String::from_utf8(output.stdout)?;

                files.trim_in_place();

                if files.is_empty() {
                    return Err(format!(
                        "The archive file cannot be found in the project {SSH_PROJECT:?}",
                        SSH_PROJECT = ssh_project
                    )
                    .into());
                }

                PathBuf::from(files)
            } else {
                String::from_utf8_lossy(output.stderr.as_slice()).split('\n').for_each(|line| {
                    if !line.is_empty() {
                        error!("{}", line);
                    }
                });

                return Err(format!(
                    "The archive file cannot be found in the project {SSH_PROJECT:?}",
                    SSH_PROJECT = ssh_project
                )
                .into());
            }
        };

        let tarball = tarball_path.file_name().unwrap().to_string_lossy();
        let public_name = tarball.strip_suffix(".tar.zst").unwrap();

        let ssh_html_path = format!(
            "{SSH_HOME}/{SERVICE_DIRECTORY}/{PUBLIC_NAME}/html",
            SSH_HOME = ssh_home,
            SERVICE_DIRECTORY = SERVICE_DIRECTORY,
            PUBLIC_NAME = public_name
        );

        {
            let mut command =
                create_ssh_command(ssh_user_host, format!("cd {SSH_PROJECT:?} && (([ ! -d public ] && mkdir public) || true) && (zstd -T0 -d -c {TARBALL:?} | tar -xf - -C public) && mkdir -p {HTML_PATH:?} && (([ -d {HTML_PATH:?} ] && rm -r {HTML_PATH:?}) || true) && cp -r public {HTML_PATH:?} && rm -r public && echo \"{TIMESTAMP} apply {REFERENCE_NAME}-{SHORT_SHA}\" >> {SSH_PROJECT:?}/../control.log",
                    SSH_PROJECT = ssh_project,
                    HTML_PATH = ssh_html_path,
                    REFERENCE_NAME = reference_name.as_ref(),
                    TARBALL = tarball,
                    TIMESTAMP = current_timestamp(),
                    SHORT_SHA = commit_sha.get_short_sha(),
                ));

            let status = command.execute()?;

            if let Some(0) = status {
                // do nothing
            } else {
                return Err("Cannot apply the project".into());
            }
        }
    }

    info!("Successfully!");

    Ok(())
}
