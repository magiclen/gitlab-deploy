use std::error::Error;
use std::fmt::Write as FmtWrite;

use execute::Execute;

use crate::clap::ArgMatches;

use crate::constants::*;
use crate::functions::*;
use crate::parse::*;

pub(crate) fn back_develop(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    check_ssh()?;
    check_bash()?;

    let project_id = parse_parse_id(matches);

    let project_name = parse_project_name(matches);

    let project_path = parse_project_path(matches);

    let reference = parse_reference(matches);

    let ssh_url_prefix = parse_ssh_url_prefix(matches);

    let ssh_user_host = parse_ssh_user_host(matches);

    info!("Deploying to {}", ssh_user_host);

    let mut ssh_home = get_ssh_home(&ssh_user_host)?;
    let ssh_root = {
        ssh_home.write_fmt(format_args!(
            "/{DEV_PROJECTS}/{PROJECT_NAME}-{PROJECT_ID}",
            DEV_PROJECTS = PROJECT_DIRECTORY,
            PROJECT_NAME = project_name.as_ref(),
            PROJECT_ID = project_id
        ))?;

        ssh_home
    };

    let git_path = format!("{}/.git", ssh_root);

    let exist = check_directory_exist(&ssh_user_host, git_path)?;

    if exist {
        info!("The project exists, trying to pull");

        check_back_deploy_via_ssh(&ssh_user_host, ssh_root.as_str())?;

        info!("Running deploy/develop-down.sh");

        let mut command = create_ssh_command(&ssh_user_host, format!("cd {SSH_ROOT:?} && (bash 'deploy/develop-down.sh' || true) && git checkout {REFERENCE:?} && git pull origin {REFERENCE:?}",
             SSH_ROOT = ssh_root,
             REFERENCE = reference.as_ref(),
        ));

        let output = command.execute_output()?;

        if !output.status.success() {
            return Err(format!(
                "Cannot pull and checkout out {REFERENCE:?}",
                REFERENCE = reference.as_ref()
            )
            .into());
        }
    } else {
        let ssh_url = format!(
            "{SSH_URL_PREFIX}/{PROJECT_PATH}.git",
            SSH_URL_PREFIX = ssh_url_prefix.as_ref(),
            PROJECT_PATH = project_path.as_ref()
        );

        info!("The project does not exist, trying to clone {:?}", ssh_url);

        let mut command = create_ssh_command(&ssh_user_host, format!("mkdir -p {SSH_ROOT:?} && cd {SSH_ROOT:?} && git clone --recursive {SSH_URL:?} . && git checkout {REFERENCE:?}",
             SSH_ROOT = ssh_root,
             SSH_URL = ssh_url,
             REFERENCE = reference.as_ref(),
        ));

        let output = command.execute_output()?;

        if !output.status.success() {
            return Err(format!(
                "Cannot clone {SSH_URL:?} and checkout out {REFERENCE:?}",
                SSH_URL = ssh_url,
                REFERENCE = reference.as_ref()
            )
            .into());
        }
    }

    check_back_deploy_via_ssh(&ssh_user_host, ssh_root.as_str())?;

    info!("Running deploy/develop-up.sh");

    let mut command = create_ssh_command(
        &ssh_user_host,
        format!("cd {SSH_ROOT:?} && bash 'deploy/develop-up.sh'", SSH_ROOT = ssh_root),
    );

    let output = command.execute_output()?;

    if !output.status.success() {
        return Err("Failed!".into());
    }

    info!("Successfully!");

    Ok(())
}
