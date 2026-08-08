use anyhow::anyhow;

use crate::{cli::BackendDevelopArgs, functions::*};

pub(crate) fn back_develop(args: BackendDevelopArgs) -> anyhow::Result<()> {
    let BackendDevelopArgs {
        gitlab_project_id: project_id,
        project_name,
        gitlab_project_path: project_path,
        reference,
        gitlab_ssh_url_prefix: ssh_url_prefix,
        develop_ssh_user_host: ssh_user_host,
    } = args;

    check_command("ssh", "-V")?;
    check_command("bash", "--version")?;

    log::info!("Deploying to {ssh_user_host}");

    let ssh_root = get_ssh_project_dir(
        get_ssh_project_root(&ssh_user_host)?.as_str(),
        &project_name,
        project_id,
    );

    let git_path = format!("{ssh_root}/.git");

    let exist = check_directory_exist(&ssh_user_host, git_path)?;

    if exist {
        log::info!("The project exists, trying to pull");

        check_back_deploy_via_ssh(&ssh_user_host, ssh_root.as_str())?;

        log::info!("Running deploy/develop-down.sh");

        {
            let mut command = create_ssh_command(
                &ssh_user_host,
                format!(
                    "cd {ssh_root} && bash {script}",
                    ssh_root = shell_quote(ssh_root.as_str()),
                    script = shell_quote("deploy/develop-down.sh"),
                ),
            );

            ensure_command_success(&mut command, || anyhow!("Cannot run deploy/develop-down.sh"))?;
        }

        log::info!(
            "Trying to checkout {reference:?} and pull the branch",
            reference = reference.as_ref()
        );

        {
            let mut command = create_ssh_command(
                &ssh_user_host,
                format!(
                    "cd {ssh_root} && git checkout {reference} && git pull origin {reference}",
                    ssh_root = shell_quote(ssh_root.as_str()),
                    reference = shell_quote(reference.as_ref()),
                ),
            );

            ensure_command_success(&mut command, || {
                anyhow!(
                    "Cannot checkout out and pull {reference:?}",
                    reference = reference.as_ref()
                )
            })?;
        }
    } else {
        let ssh_url = format!(
            "{ssh_url_prefix}/{project_path}.git",
            ssh_url_prefix = ssh_url_prefix.as_ref(),
            project_path = project_path.as_ref()
        );

        log::info!(
            "The project does not exist, trying to clone {ssh_url:?} and checkout {reference:?}",
            reference = reference.as_ref(),
        );

        let mut command = create_ssh_command(
            &ssh_user_host,
            format!(
                "mkdir -p {ssh_root} && cd {ssh_root} && git clone --recursive {ssh_url} . && git \
                 checkout {reference}",
                ssh_root = shell_quote(ssh_root.as_str()),
                ssh_url = shell_quote(ssh_url.as_str()),
                reference = shell_quote(reference.as_ref()),
            ),
        );

        ensure_command_success(&mut command, || {
            anyhow!(
                "Cannot clone {ssh_url:?} and checkout out {reference:?}",
                reference = reference.as_ref()
            )
        })?;
    }

    check_back_deploy_via_ssh(&ssh_user_host, ssh_root.as_str())?;

    log::info!("Running deploy/develop-up.sh");

    let mut command = create_ssh_command(
        &ssh_user_host,
        format!(
            "cd {ssh_root} && bash {script}",
            ssh_root = shell_quote(ssh_root.as_str()),
            script = shell_quote("deploy/develop-up.sh"),
        ),
    );

    ensure_command_success(&mut command, || anyhow!("Failed!"))?;

    log::info!("Successfully!");

    Ok(())
}
