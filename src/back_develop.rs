use std::fmt::Write;

use anyhow::anyhow;
use execute::Execute;

use crate::{
    cli::{CLIArgs, CLICommands},
    constants::*,
    functions::*,
};

pub(crate) fn back_develop(cli_args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(cli_args.command, CLICommands::BackendDevelop { .. }));

    if let CLICommands::BackendDevelop {
        gitlab_project_id: project_id,
        project_name,
        gitlab_project_path: project_path,
        reference,
        gitlab_ssh_url_prefix: ssh_url_prefix,
        develop_ssh_user_host: ssh_user_host,
    } = cli_args.command
    {
        check_ssh()?;
        check_bash()?;

        log::info!("Deploying to {ssh_user_host}");

        let ssh_root = {
            let mut ssh_home = get_ssh_home(&ssh_user_host)?;

            ssh_home.write_fmt(format_args!(
                "/{PROJECT_DIRECTORY}/{project_name}-{project_id}",
                project_name = project_name.as_ref(),
            ))?;

            ssh_home
        };

        let git_path = format!("{ssh_root}/.git");

        let exist = check_directory_exist(&ssh_user_host, git_path)?;

        if exist {
            log::info!("The project exists, trying to pull");

            check_back_deploy_via_ssh(&ssh_user_host, ssh_root.as_str())?;

            log::info!("Running deploy/develop-down.sh");

            {
                let mut command = create_ssh_command(
                    &ssh_user_host,
                    format!("cd {ssh_root:?} && bash 'deploy/develop-down.sh'",),
                );

                command.execute_output()?;
            }

            log::info!(
                "Trying to checkout {reference:?} and pull the branch",
                reference = reference.as_ref()
            );

            {
                let mut command = create_ssh_command(
                    &ssh_user_host,
                    format!(
                        "cd {ssh_root:?} && git checkout {reference:?} && git pull origin \
                         {reference:?}",
                        reference = reference.as_ref(),
                    ),
                );

                let output = command.execute_output()?;

                if !output.status.success() {
                    return Err(anyhow!(
                        "Cannot checkout out and pull {reference:?}",
                        reference = reference.as_ref()
                    ));
                }
            }
        } else {
            let ssh_url = format!(
                "{ssh_url_prefix}/{project_path}.git",
                ssh_url_prefix = ssh_url_prefix.as_ref(),
                project_path = project_path.as_ref()
            );

            log::info!(
                "The project does not exist, trying to clone {ssh_url:?} and checkout \
                 {reference:?}",
                reference = reference.as_ref(),
            );

            let mut command = create_ssh_command(
                &ssh_user_host,
                format!(
                    "mkdir -p {ssh_root:?} && cd {ssh_root:?} && git clone --recursive \
                     {ssh_url:?} . && git checkout {reference:?}",
                    ssh_root = ssh_root,
                    reference = reference.as_ref(),
                ),
            );

            let output = command.execute_output()?;

            if !output.status.success() {
                return Err(anyhow!(
                    "Cannot clone {ssh_url:?} and checkout out {reference:?}",
                    reference = reference.as_ref()
                ));
            }
        }

        check_back_deploy_via_ssh(&ssh_user_host, ssh_root.as_str())?;

        log::info!("Running deploy/develop-up.sh");

        let mut command = create_ssh_command(
            &ssh_user_host,
            format!("cd {SSH_ROOT:?} && bash 'deploy/develop-up.sh'", SSH_ROOT = ssh_root),
        );

        let output = command.execute_output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed!"));
        }

        log::info!("Successfully!");
    }

    Ok(())
}
