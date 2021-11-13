use std::error::Error;

use crate::clap::ArgMatches;

use crate::tempfile::tempdir;

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
    let commit_sha = commit_sha.get_sha();

    let build_target = parse_build_target(matches);

    let api_url_prefix = parse_api_url_prefix(matches);

    let api_token = parse_api_token(matches);

    let ssh_user_host = parse_ssh_user_host(matches);

    let temp_dir = tempdir()?;

    download_and_extract_archive(&temp_dir, api_url_prefix, api_token, project_id, commit_sha)?;

    let public_name = check_front_deploy(&temp_dir)?;

    run_front_build(&temp_dir, build_target)?;

    let user_host = ssh_user_host.user_host();

    info!("Deploying to {}", user_host);

    let ssh_root = {
       // use command to ssh
    };

    let tarball_path = format!("deploy/{PUBLIC_NAME}.tar.zst", PUBLIC_NAME = public_name.as_ref());

    Ok(())
}
