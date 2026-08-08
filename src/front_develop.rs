use anyhow::anyhow;
use execute::command_args;
use tempfile::tempdir;

use crate::{cli::FrontendDevelopArgs, constants::*, functions::*};

pub(crate) fn front_develop(args: FrontendDevelopArgs) -> anyhow::Result<()> {
    let FrontendDevelopArgs {
        gitlab_project_id: project_id,
        commit_sha,
        build_target,
        gitlab_api_url_prefix: api_url_prefix,
        gitlab_api_token: api_token,
        no_check_certificate,
        develop_ssh_user_host: ssh_user_host,
    } = args;

    check_command("zstd", "--version")?;
    check_command("ssh", "-V")?;
    check_command("wget", "--version")?;
    check_command("tar", "--version")?;
    check_command("bash", "--version")?;

    let temp_dir = tempdir()?;

    download_and_extract_archive(
        &temp_dir,
        &api_url_prefix,
        &api_token,
        no_check_certificate,
        project_id,
        &commit_sha,
    )?;

    let public_name = check_front_deploy(&temp_dir)?;

    run_front_build(&temp_dir, build_target)?;

    let tarball_path = check_build_archive(&temp_dir, public_name.as_ref())?;

    log::info!("Deploying to {ssh_user_host}");

    let ssh_root = format!(
        "{ssh_home}/{SERVICE_DIRECTORY}/www/{public_name}",
        ssh_home = get_ssh_home(&ssh_user_host)?,
        public_name = public_name.as_ref(),
    );

    let ssh_html_path = format!("{ssh_root}/html");

    {
        let mut command = create_ssh_command(
            &ssh_user_host,
            format!(
                "mkdir -p {ssh_root} && ((test -d {ssh_html_path} && rm -rf {ssh_html_path}) || \
                 true) && mkdir -p {ssh_html_path}",
                ssh_root = shell_quote(ssh_root.as_str()),
                ssh_html_path = shell_quote(ssh_html_path.as_str()),
            ),
        );

        ensure_command_success(&mut command, || {
            anyhow!(
                "Cannot create the directory {ssh_html_path:?} for storing the public static \
                 files."
            )
        })?;
    }

    {
        let mut command1 = command_args!("zstd", "-d", "-c", tarball_path.as_str());

        command1.current_dir(temp_dir.path());

        let mut command2 = create_ssh_command_with_stdin(
            &ssh_user_host,
            format!(
                "tar -xf - -C {ssh_html_path}",
                ssh_html_path = shell_quote(ssh_html_path.as_str())
            ),
        );

        log::info!("Extracting {tarball_path}");

        ensure_pipeline_success(&mut command1, &mut command2, || anyhow!("Extract failed."))?;
    }

    log::info!("Listing the public static files...");

    list_ssh_files(&ssh_user_host, ssh_html_path)?;

    log::info!("Successfully!");

    Ok(())
}
