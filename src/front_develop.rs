use std::error::Error;
use std::fmt::Write as FmtWrite;

use execute::{command_args, Execute};

use clap::ArgMatches;

use tempfile::tempdir;

use crate::constants::*;
use crate::functions::*;
use crate::parse::*;

pub(crate) fn front_develop(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    check_zstd()?;
    check_ssh()?;
    check_wget()?;
    check_tar()?;
    check_bash()?;

    let project_id = parse_parse_id(matches);

    let commit_sha = parse_commit_sha(matches);

    let build_target = parse_build_target(matches);

    let api_url_prefix = parse_api_url_prefix(matches);

    let api_token = parse_api_token(matches);

    let ssh_user_host = parse_ssh_user_host(matches);

    let temp_dir = tempdir()?;

    download_and_extract_archive(&temp_dir, api_url_prefix, api_token, project_id, &commit_sha)?;

    let public_name = check_front_deploy(&temp_dir)?;

    run_front_build(&temp_dir, build_target)?;

    info!("Deploying to {}", ssh_user_host);

    let ssh_root = {
        let mut ssh_home = get_ssh_home(&ssh_user_host)?;

        ssh_home.write_fmt(format_args!(
            "/{SERVICE_DIRECTORY}/www/{PUBLIC_NAME}",
            SERVICE_DIRECTORY = SERVICE_DIRECTORY,
            PUBLIC_NAME = public_name.as_ref()
        ))?;

        ssh_home
    };

    let ssh_html_path = format!("{}/html", ssh_root);

    {
        let mut command = create_ssh_command(&ssh_user_host, format!("mkdir -p {ROOT_PATH:?} && ((test -d {HTML_PATH:?} && rm -r {HTML_PATH:?}) || true) && mkdir -p {HTML_PATH:?}",
             ROOT_PATH = ssh_root,
             HTML_PATH = ssh_html_path,
        ));

        let status = command.execute()?;

        if let Some(0) = status {
            // do nothing
        } else {
            return Err(format!(
                "Cannot create the directory {:?} for storing the public static files.",
                ssh_html_path
            )
            .into());
        }
    }

    let tarball_path = format!("deploy/{PUBLIC_NAME}.tar.zst", PUBLIC_NAME = public_name.as_ref());

    {
        let mut command1 = command_args!("zstd", "-T0", "-d", "-c", tarball_path);

        command1.current_dir(temp_dir.path());

        let mut command2 = create_ssh_command(
            &ssh_user_host,
            format!("tar -xf - -C {HTML_PATH:?}", HTML_PATH = ssh_html_path),
        );

        info!("Extracting {}", tarball_path);

        let result = command1.execute_multiple(&mut [&mut command2])?;

        if let Some(0) = result {
            // do nothing
        } else {
            return Err("Extract failed.".into());
        }
    }

    info!("Listing the public static files...");

    list_ssh_files(&ssh_user_host, ssh_html_path)?;

    info!("Successfully!");

    Ok(())
}
