mod api_token;
mod api_url_prefix;
mod build_target;
mod commit_sha;
mod public_name;
mod reference;
mod ssh_url_prefix;
mod ssh_user_host;

use std::process;

use crate::clap::ArgMatches;

use crate::validators::prelude::*;

pub(crate) use api_token::*;
pub(crate) use api_url_prefix::*;
pub(crate) use build_target::*;
pub(crate) use commit_sha::*;
pub(crate) use public_name::*;
pub(crate) use reference::*;
pub(crate) use ssh_url_prefix::*;
pub(crate) use ssh_user_host::*;

pub(crate) fn parse_parse_id(matches: &ArgMatches) -> u64 {
    match matches.value_of("GITLAB_PROJECT_ID") {
        Some(project_id) => {
            match project_id.parse::<u64>() {
                Ok(project_id) => project_id,
                Err(_) => {
                    error!("{:?} is not a correct GitLab project ID", project_id);
                    process::exit(-2);
                }
            }
        }
        None => {
            error!("`--gitlab-project-id` needs to be set.");
            process::exit(-2);
        }
    }
}

pub(crate) fn parse_commit_sha(matches: &ArgMatches<'_>) -> CommitSha {
    match matches.value_of("COMMIT_SHA") {
        Some(sha) => {
            match CommitSha::parse_str(sha) {
                Ok(sha) => sha,
                Err(_) => {
                    error!("{:?} is not a correct commit sha.", sha);
                    process::exit(-2);
                }
            }
        }
        None => {
            error!("`--commit-short-sha` needs to be set.");
            process::exit(-2);
        }
    }
}

pub(crate) fn parse_reference(matches: &ArgMatches<'_>) -> Reference {
    match matches.value_of("REFERENCE") {
        Some(reference) => {
            match Reference::parse_str(reference) {
                Ok(reference) => reference,
                Err(_) => {
                    error!("{:?} is not a correct reference.", reference);
                    process::exit(-2);
                }
            }
        }
        None => {
            error!("`--reference` needs to be set.");
            process::exit(-2);
        }
    }
}

pub(crate) fn parse_build_target(matches: &ArgMatches<'_>) -> BuildTarget {
    match matches.value_of("BUILD_TARGET") {
        Some(target) => {
            match BuildTarget::parse_str(target) {
                Ok(target) => target,
                Err(_) => {
                    error!("{:?} is not a correct build target.", target);
                    process::exit(-2);
                }
            }
        }
        None => {
            error!("`--build-target` needs to be set.");
            process::exit(-2);
        }
    }
}

pub(crate) fn parse_api_url_prefix(matches: &ArgMatches<'_>) -> ApiUrlPrefix {
    match matches.value_of("GITLAB_API_URL_PREFIX") {
        Some(api_url_prefix) => {
            match ApiUrlPrefix::parse_str(api_url_prefix) {
                Ok(api_url_prefix) => api_url_prefix,
                Err(_) => {
                    error!("{:?} is not a correct GitLab API URL prefix.", api_url_prefix);
                    process::exit(-2);
                }
            }
        }
        None => {
            error!("`--gitlab-api-url-prefix` needs to be set.");
            process::exit(-2);
        }
    }
}

pub(crate) fn parse_api_token(matches: &ArgMatches<'_>) -> ApiToken {
    match matches.value_of("GITLAB_API_TOKEN") {
        Some(api_token) => {
            match ApiToken::parse_str(api_token) {
                Ok(api_token) => api_token,
                Err(_) => {
                    error!("{:?} is not a correct GitLab API token.", api_token);
                    process::exit(-2);
                }
            }
        }
        None => {
            error!("`--gitlab-api-token` needs to be set.");
            process::exit(-2);
        }
    }
}

pub(crate) fn parse_ssh_url_prefix(matches: &ArgMatches<'_>) -> SshUrlPrefix {
    match matches.value_of("GITLAB_SSH_URL_PREFIX") {
        Some(ssh_url_prefix) => {
            match SshUrlPrefix::parse_str(ssh_url_prefix) {
                Ok(ssh_url_prefix) => ssh_url_prefix,
                Err(_) => {
                    error!("{:?} is not a correct SSH URL prefix.", ssh_url_prefix);
                    process::exit(-2);
                }
            }
        }
        None => {
            error!("`--gitlab-ssh-url-prefix` needs to be set.");
            process::exit(-2);
        }
    }
}

pub(crate) fn parse_ssh_user_host(matches: &ArgMatches<'_>) -> SshUserHost {
    match matches.value_of("DEVELOP_SSH_USR_HOST") {
        Some(ssh_user_host) => {
            match SshUserHost::parse_str(ssh_user_host) {
                Ok(ssh_user_host) => ssh_user_host,
                Err(_) => {
                    error!("{:?} is not a correct SSH user and host.", ssh_user_host);
                    process::exit(-2);
                }
            }
        }
        None => {
            error!("`--develop-ssh-user-host` needs to be set.");
            process::exit(-2);
        }
    }
}
