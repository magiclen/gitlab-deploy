use std::fmt::Write as FmtWrite;

use anyhow::anyhow;
use execute::{command_args, Execute};
use tempfile::tempdir;

use crate::{
    cli::{CLIArgs, CLICommands},
    constants::*,
    functions::*,
};

pub(crate) fn front_develop(cli_args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(cli_args.command, CLICommands::FrontendDevelop { .. }));

    if let CLICommands::FrontendDevelop {
        gitlab_project_id: project_id,
        commit_sha,
        build_target,
        gitlab_api_url_prefix: api_url_prefix,
        gitlab_api_token: api_token,
        develop_ssh_user_host: ssh_user_host,
    } = cli_args.command
    {
        check_zstd()?;
        check_ssh()?;
        check_wget()?;
        check_tar()?;
        check_bash()?;

        let temp_dir = tempdir()?;

        download_and_extract_archive(
            &temp_dir,
            api_url_prefix,
            api_token,
            project_id,
            &commit_sha,
        )?;

        let public_name = check_front_deploy(&temp_dir)?;

        run_front_build(&temp_dir, build_target)?;

        log::info!("Deploying to {ssh_user_host}");

        let ssh_root = {
            let mut ssh_home = get_ssh_home(&ssh_user_host)?;

            ssh_home.write_fmt(format_args!(
                "/{SERVICE_DIRECTORY}/www/{public_name}",
                public_name = public_name.as_ref()
            ))?;

            ssh_home
        };

        let ssh_html_path = format!("{ssh_root}/html");

        {
            let mut command = create_ssh_command(
                &ssh_user_host,
                format!(
                    "mkdir -p {ssh_root:?} && ((test -d {ssh_html_path:?} && rm -r \
                     {ssh_html_path:?}) || true) && mkdir -p {ssh_html_path:?}",
                ),
            );

            let status = command.execute()?;

            if let Some(0) = status {
                // do nothing
            } else {
                return Err(anyhow!(
                    "Cannot create the directory {ssh_html_path:?} for storing the public static \
                     files."
                ));
            }
        }

        let tarball_path =
            format!("deploy/{public_name}.tar.zst", public_name = public_name.as_ref());

        {
            let mut command1 = command_args!("zstd", "-T0", "-d", "-c", tarball_path);

            command1.current_dir(temp_dir.path());

            let mut command2 =
                create_ssh_command(&ssh_user_host, format!("tar -xf - -C {ssh_html_path:?}"));

            log::info!("Extracting {tarball_path}");

            let result = command1.execute_multiple(&mut [&mut command2])?;

            if let Some(0) = result {
                // do nothing
            } else {
                return Err(anyhow!("Extract failed."));
            }
        }

        log::info!("Listing the public static files...");

        list_ssh_files(&ssh_user_host, ssh_html_path)?;

        log::info!("Successfully!");
    }

    Ok(())
}
