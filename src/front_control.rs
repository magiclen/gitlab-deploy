use std::{path::PathBuf, process::Stdio};

use anyhow::anyhow;
use execute::Execute;
use trim_in_place::TrimInPlace;
use validators::prelude::*;

use crate::{
    cli::FrontendControlArgs, constants::*, front_publish::Publication, functions::*, models::*,
};

pub(crate) fn front_control(args: FrontendControlArgs) -> anyhow::Result<()> {
    let FrontendControlArgs {
        gitlab_project_id: project_id,
        commit_sha,
        project_name,
        reference_name,
        phase,
    } = args;

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
            get_project_root(ssh_home.as_str()).as_str(),
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

        // The name comes from the remote host, so it has to pass the same validation as the name that the deployment wrote.
        let public_name = match Name::parse_str(public_name) {
            Ok(public_name) => public_name,
            Err(_) => {
                return Err(anyhow!("{tarball_path:?} is not a correct archive file"));
            },
        };

        check_public_name(&public_name)?;

        let ssh_www_path = format!(
            "{ssh_home}/{SERVICE_DIRECTORY}/www/{public_name}",
            public_name = public_name.as_ref()
        );
        let ssh_html_path = format!("{ssh_www_path}/html");

        let publication = Publication::prepare(ssh_user_host, &ssh_www_path)?;

        let mut command = create_ssh_pipeline_command(
            ssh_user_host,
            &format!(
                "cd {ssh_project} && zstd -d -c -- {tarball} | tar -xf - -C {html}",
                ssh_project = shell_quote(&ssh_project),
                tarball = shell_quote(tarball.as_ref()),
                html = shell_quote(&publication.html_path()),
            ),
        );

        if let Err(error) = ensure_command_success(&mut command, || {
            anyhow!("Cannot extract the public static files")
        }) {
            publication.discard();
            return Err(error);
        }

        publication.publish()?;

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
                    "echo {log_message} >> {ssh_control_log}",
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
