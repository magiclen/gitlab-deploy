use std::error::Error;
use std::fmt::Write as FmtWrite;

use execute::Execute;

use crate::clap::ArgMatches;

use crate::tempfile::tempdir;

use crate::constants::*;
use crate::functions::*;
use crate::parse::*;

pub(crate) fn back_deploy(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    check_zstd()?;
    check_ssh()?;
    check_wget()?;
    check_tar()?;
    check_bash()?;
    check_docker()?;

    let project_id = parse_parse_id(matches);

    let commit_sha = parse_commit_sha(matches);

    let project_name = parse_project_name(matches);

    let reference_name = parse_reference_name(matches);

    let build_target = parse_build_target_allow_null(matches);

    let phase = parse_phase(matches);

    let api_url_prefix = parse_api_url_prefix(matches);

    let api_token = parse_api_token(matches);

    let ssh_user_hosts = find_ssh_user_hosts(phase, project_id)?;

    if ssh_user_hosts.is_empty() {
        warn!("No hosts to deploy!");
        return Ok(());
    }

    let temp_dir = tempdir()?;

    download_and_extract_archive(&temp_dir, api_url_prefix, api_token, project_id, &commit_sha)?;

    let (image_name, docker_compose) =
        check_back_deploy(&temp_dir, &commit_sha, build_target.as_ref())?;

    run_back_build(&temp_dir, &commit_sha, build_target.as_ref())?;

    for ssh_user_host in ssh_user_hosts.iter() {
        info!("Deploying to {}", ssh_user_host);

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
            let mut command = create_ssh_command(
                ssh_user_host,
                format!("mkdir -p {SSH_PROJECT:?}", SSH_PROJECT = ssh_project),
            );

            let status = command.execute()?;

            if let Some(0) = status {
                // do nothing
            } else {
                return Err(format!(
                    "Cannot create the directory {:?} for storing the archive of public static files.",
                    ssh_project
                )
                .into());
            }
        }

        let ssh_docker_compose_path = format!("{}/docker-compose.yml", ssh_project);

        {
            let mut command = create_ssh_command(
                ssh_user_host,
                format!(
                    "cat - > {SSH_DOCKER_COMPOSE_PATH}",
                    SSH_DOCKER_COMPOSE_PATH = ssh_docker_compose_path
                ),
            );

            let status = command.execute_input(docker_compose.as_str())?;

            if let Some(0) = status {
                // do nothing
            } else {
                return Err(format!(
                    "Cannot create the docker compose file {:?}.",
                    ssh_docker_compose_path
                )
                .into());
            }
        }

        let tarball_path = format!("deploy/{IMAGE_NAME}.tar.zst", IMAGE_NAME = image_name.as_ref());

        let ssh_tarball_path = format!(
            "{SSH_PROJECT}/{IMAGE_NAME}.tar.zst",
            SSH_PROJECT = ssh_project,
            IMAGE_NAME = image_name.as_ref()
        );

        {
            let mut command =
                create_scp_command(ssh_user_host, tarball_path.as_str(), ssh_tarball_path.as_str());

            command.current_dir(temp_dir.path());

            let status = command.execute()?;

            if let Some(0) = status {
                // do nothing
            } else {
                return Err(format!(
                    "Cannot copy {FROM:?} to {USER_HOST}:{TO:?} ({PORT}).",
                    FROM = tarball_path,
                    USER_HOST = ssh_user_host.user_host(),
                    TO = ssh_tarball_path,
                    PORT = ssh_user_host.get_port(),
                )
                .into());
            }
        }

        info!("Extracting {}", tarball_path);

        {
            let mut command1 = command_args!("zstd", "-d", "-c", "-f", tarball_path);

            command1.current_dir(temp_dir.path());

            let mut command2 = create_ssh_command(ssh_user_host, "docker image load");

            let status = command1.execute_multiple(&mut [&mut command2])?;

            if let Some(0) = status {
                // do nothing
            } else {
                return Err("Cannot deploy the docker image".into());
            }
        }
    }

    info!("Successfully!");

    Ok(())
}
