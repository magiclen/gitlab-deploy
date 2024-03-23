use anyhow::anyhow;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use concat_with::concat_line;
use terminal_size::terminal_size;
use validators::{
    errors::{HttpURLError, LineError, RegexError},
    prelude::*,
};

use crate::models::*;

const APP_NAME: &str = "Gitlab Deploy";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

const AFTER_HELP: &str = "Enjoy it! https://magiclen.org";

const APP_ABOUT: &str = concat!(
    "GitLab Deploy is used for deploying software projects to multiple hosts during different \
     phases\n\nEXAMPLES:\n",
    concat_line!(prefix "gitlab-deploy ",
        "frontend-develop --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --build-target develop",
        "frontend-deploy  --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test --build-target test",
        "frontend-control --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test",
        "backend-develop  --gitlab-project-id 123 --gitlab-project-path website-api                     --project-name website --reference develop",
        "backend-deploy   --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test",
        "backend-control  --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test --command up",
        "simple-deploy    --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test",
        "simple-control   --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test sudo /usr/local/bin/apply-nginx.sh dev.env",
    )
);

#[derive(Debug, Parser)]
#[command(name = APP_NAME)]
#[command(term_width = terminal_size().map(|(width, _)| width.0 as usize).unwrap_or(0))]
#[command(version = CARGO_PKG_VERSION)]
#[command(author = CARGO_PKG_AUTHORS)]
#[command(after_help = AFTER_HELP)]
pub struct CLIArgs {
    #[command(subcommand)]
    pub command: CLICommands,
}

#[derive(Debug, Subcommand)]
pub enum CLICommands {
    #[command(about = "Fetch the project via GitLab API and then build it and use the public \
                       static files on a development host")]
    #[command(after_help = AFTER_HELP)]
    FrontendDevelop {
        #[arg(long, visible_aliases = ["project-id", "id"], env = "CI_PROJECT_ID")]
        #[arg(help = "Set the ID on GitLab of this project")]
        gitlab_project_id:     u64,
        #[arg(long, visible_aliases = ["sha"], env = "CI_COMMIT_SHA")]
        #[arg(value_parser = parse_commit_sha)]
        #[arg(help = "Set the sha of the commit")]
        commit_sha:            CommitSha,
        #[arg(long, visible_aliases = ["target"])]
        #[arg(value_parser = parse_build_target)]
        #[arg(help = "Set the target of this build")]
        build_target:          BuildTarget,
        #[arg(long, visible_aliases = ["api-url-prefix"], env = "GITLAB_API_URL_PREFIX")]
        #[arg(value_parser = parse_api_url_prefix)]
        #[arg(help = "Set the URL prefix for GitLab APIs")]
        gitlab_api_url_prefix: ApiUrlPrefix,
        #[arg(long, visible_aliases = ["api-token"], env = "GITLAB_API_TOKEN")]
        #[arg(value_parser = parse_api_token)]
        #[arg(help = "Set the token of GitLab APIs")]
        gitlab_api_token:      ApiToken,
        #[arg(long, visible_aliases = ["ssh-user-host"], env = "DEVELOP_SSH_HOST")]
        #[arg(value_parser = parse_ssh_user_host)]
        #[arg(help = "Set the SSH user, host and the optional port for development")]
        develop_ssh_user_host: SshUserHost,
    },
    #[command(about = "Fetch the project via GitLab API and then build it and deploy the \
                       archive of public static files on multiple hosts according to the phase")]
    #[command(after_help = AFTER_HELP)]
    FrontendDeploy {
        #[arg(long, visible_aliases = ["project-id", "id"], env = "CI_PROJECT_ID")]
        #[arg(help = "Set the ID on GitLab of this project")]
        gitlab_project_id:     u64,
        #[arg(long, visible_aliases = ["sha"], env = "CI_COMMIT_SHA")]
        #[arg(value_parser = parse_commit_sha)]
        #[arg(help = "Set the sha of the commit")]
        commit_sha:            CommitSha,
        #[arg(long, env = "CI_PROJECT_NAME")]
        #[arg(value_parser = parse_name)]
        #[arg(help = "Set the name of this project")]
        project_name:          Name,
        #[arg(long, env = "CI_COMMIT_REF_NAME")]
        #[arg(value_parser = parse_name)]
        #[arg(help = "Set the reference name of the commit")]
        reference_name:        Name,
        #[arg(long, visible_aliases = ["target"])]
        #[arg(value_parser = parse_build_target)]
        #[arg(help = "Set the target of this build")]
        build_target:          BuildTarget,
        #[arg(long, visible_aliases = ["phase"])]
        #[arg(value_parser = parse_phase)]
        #[arg(help = "Set the phase")]
        phase:                 Phase,
        #[arg(long, visible_aliases = ["api-url-prefix"], env = "GITLAB_API_URL_PREFIX")]
        #[arg(value_parser = parse_api_url_prefix)]
        #[arg(help = "Set the URL prefix for GitLab APIs")]
        gitlab_api_url_prefix: ApiUrlPrefix,
        #[arg(long, visible_aliases = ["api-token"], env = "GITLAB_API_TOKEN")]
        #[arg(value_parser = parse_api_token)]
        #[arg(help = "Set the token of GitLab APIs")]
        gitlab_api_token:      ApiToken,
    },
    #[command(about = "Control the project on multiple hosts according to the phase")]
    #[command(after_help = AFTER_HELP)]
    FrontendControl {
        #[arg(long, visible_aliases = ["project-id", "id"], env = "CI_PROJECT_ID")]
        #[arg(help = "Set the ID on GitLab of this project")]
        gitlab_project_id: u64,
        #[arg(long, visible_aliases = ["sha"], env = "CI_COMMIT_SHA")]
        #[arg(value_parser = parse_commit_sha)]
        #[arg(help = "Set the sha of the commit")]
        commit_sha:        CommitSha,
        #[arg(long, env = "CI_PROJECT_NAME")]
        #[arg(value_parser = parse_name)]
        #[arg(help = "Set the name of this project")]
        project_name:      Name,
        #[arg(long, env = "CI_COMMIT_REF_NAME")]
        #[arg(value_parser = parse_name)]
        #[arg(help = "Set the reference name of the commit")]
        reference_name:    Name,
        #[arg(long, visible_aliases = ["phase"])]
        #[arg(value_parser = parse_phase)]
        #[arg(help = "Set the phase")]
        phase:             Phase,
    },
    #[command(about = "Fetch the project via Git and checkout to a specific branch and then \
                       start up the service on a development host")]
    #[command(after_help = AFTER_HELP)]
    BackendDevelop {
        #[arg(long, visible_aliases = ["project-id", "id"], env = "CI_PROJECT_ID")]
        #[arg(help = "Set the ID on GitLab of this project")]
        gitlab_project_id:     u64,
        #[arg(long, env = "CI_PROJECT_NAME")]
        #[arg(value_parser = parse_name)]
        #[arg(help = "Set the name of this project")]
        project_name:          Name,
        #[arg(long, visible_aliases = ["project-path"], env = "CI_PROJECT_PATH")]
        #[arg(value_parser = parse_project_path)]
        #[arg(help = "Set the path of this project on GitLab")]
        gitlab_project_path:   ProjectPath,
        #[arg(long, visible_aliases = ["ref"], env = "CI_COMMIT_BRANCH")]
        #[arg(value_parser = parse_reference)]
        #[arg(help = "Set the reference of the commit")]
        reference:             Reference,
        #[arg(long, visible_aliases = ["ssh-url-prefix"], env = "GITLAB_SSH_URL_PREFIX")]
        #[arg(value_parser = parse_ssh_url_prefix)]
        #[arg(help = "Set the SSH URL prefix")]
        gitlab_ssh_url_prefix: SshUrlPrefix,
        #[arg(long, visible_aliases = ["ssh-user-host"], env = "DEVELOP_SSH_HOST")]
        #[arg(value_parser = parse_ssh_user_host)]
        #[arg(help = "Set the SSH user, host and the optional port for development")]
        develop_ssh_user_host: SshUserHost,
    },
    #[command(about = "Fetch the project via GitLab API and then build it and deploy the docker \
                       image on multiple hosts according to the phase")]
    #[command(after_help = AFTER_HELP)]
    BackendDeploy {
        #[arg(long, visible_aliases = ["project-id", "id"], env = "CI_PROJECT_ID")]
        #[arg(help = "Set the ID on GitLab of this project")]
        gitlab_project_id:     u64,
        #[arg(long, visible_aliases = ["sha"], env = "CI_COMMIT_SHA")]
        #[arg(value_parser = parse_commit_sha)]
        #[arg(help = "Set the sha of the commit")]
        commit_sha:            CommitSha,
        #[arg(long, env = "CI_PROJECT_NAME")]
        #[arg(value_parser = parse_name)]
        #[arg(help = "Set the name of this project")]
        project_name:          Name,
        #[arg(long, env = "CI_COMMIT_REF_NAME")]
        #[arg(value_parser = parse_name)]
        #[arg(help = "Set the reference name of the commit")]
        reference_name:        Name,
        #[arg(long, visible_aliases = ["target"])]
        #[arg(value_parser = parse_build_target)]
        #[arg(help = "Set the target of this build")]
        build_target:          Option<BuildTarget>,
        #[arg(long, visible_aliases = ["phase"])]
        #[arg(value_parser = parse_phase)]
        #[arg(help = "Set the phase")]
        phase:                 Phase,
        #[arg(long, visible_aliases = ["api-url-prefix"], env = "GITLAB_API_URL_PREFIX")]
        #[arg(value_parser = parse_api_url_prefix)]
        #[arg(help = "Set the URL prefix for GitLab APIs")]
        gitlab_api_url_prefix: ApiUrlPrefix,
        #[arg(long, visible_aliases = ["api-token"], env = "GITLAB_API_TOKEN")]
        #[arg(value_parser = parse_api_token)]
        #[arg(help = "Set the token of GitLab APIs")]
        gitlab_api_token:      ApiToken,
    },
    #[command(about = "Control the project on multiple hosts according to the phase")]
    #[command(after_help = AFTER_HELP)]
    BackendControl {
        #[arg(long, visible_aliases = ["project-id", "id"], env = "CI_PROJECT_ID")]
        #[arg(help = "Set the ID on GitLab of this project")]
        gitlab_project_id: u64,
        #[arg(long, visible_aliases = ["sha"], env = "CI_COMMIT_SHA")]
        #[arg(value_parser = parse_commit_sha)]
        #[arg(help = "Set the sha of the commit")]
        commit_sha:        CommitSha,
        #[arg(long, env = "CI_PROJECT_NAME")]
        #[arg(value_parser = parse_name)]
        #[arg(help = "Set the name of this project")]
        project_name:      Name,
        #[arg(long, env = "CI_COMMIT_REF_NAME")]
        #[arg(value_parser = parse_name)]
        #[arg(help = "Set the reference name of the commit")]
        reference_name:    Name,
        #[arg(long, visible_aliases = ["phase"])]
        #[arg(value_parser = parse_phase)]
        #[arg(help = "Set the phase")]
        phase:             Phase,
        #[arg(long)]
        #[arg(value_parser = parse_command)]
        #[arg(help = "Set the command")]
        command:           Command,
    },
    #[command(about = "Fetch the project via GitLab API and deploy the project files on \
                       multiple hosts according to the phase")]
    #[command(after_help = AFTER_HELP)]
    SimpleDeploy {
        #[arg(long, visible_aliases = ["project-id", "id"], env = "CI_PROJECT_ID")]
        #[arg(help = "Set the ID on GitLab of this project")]
        gitlab_project_id:     u64,
        #[arg(long, visible_aliases = ["sha"], env = "CI_COMMIT_SHA")]
        #[arg(value_parser = parse_commit_sha)]
        #[arg(help = "Set the sha of the commit")]
        commit_sha:            CommitSha,
        #[arg(long, env = "CI_PROJECT_NAME")]
        #[arg(value_parser = parse_name)]
        #[arg(help = "Set the name of this project")]
        project_name:          Name,
        #[arg(long, env = "CI_COMMIT_REF_NAME")]
        #[arg(value_parser = parse_name)]
        #[arg(help = "Set the reference name of the commit")]
        reference_name:        Name,
        #[arg(long, visible_aliases = ["phase"])]
        #[arg(value_parser = parse_phase)]
        #[arg(help = "Set the phase")]
        phase:                 Phase,
        #[arg(long, visible_aliases = ["api-url-prefix"], env = "GITLAB_API_URL_PREFIX")]
        #[arg(value_parser = parse_api_url_prefix)]
        #[arg(help = "Set the URL prefix for GitLab APIs")]
        gitlab_api_url_prefix: ApiUrlPrefix,
        #[arg(long, visible_aliases = ["api-token"], env = "GITLAB_API_TOKEN")]
        #[arg(value_parser = parse_api_token)]
        #[arg(help = "Set the token of GitLab APIs")]
        gitlab_api_token:      ApiToken,
    },
    #[command(about = "Control the project on multiple hosts according to the phase")]
    #[command(after_help = AFTER_HELP)]
    SimpleControl {
        #[arg(long, visible_aliases = ["project-id", "id"], env = "CI_PROJECT_ID")]
        #[arg(help = "Set the ID on GitLab of this project")]
        gitlab_project_id:        u64,
        #[arg(long, visible_aliases = ["sha"], env = "CI_COMMIT_SHA")]
        #[arg(value_parser = parse_commit_sha)]
        #[arg(help = "Set the sha of the commit")]
        commit_sha:               CommitSha,
        #[arg(long, env = "CI_PROJECT_NAME")]
        #[arg(value_parser = parse_name)]
        #[arg(help = "Set the name of this project")]
        project_name:             Name,
        #[arg(long, env = "CI_COMMIT_REF_NAME")]
        #[arg(value_parser = parse_name)]
        #[arg(help = "Set the reference name of the commit")]
        reference_name:           Name,
        #[arg(long, visible_aliases = ["phase"])]
        #[arg(value_parser = parse_phase)]
        #[arg(help = "Set the phase")]
        phase:                    Phase,
        #[arg(long, visible_aliases = ["api-url-prefix"], env = "GITLAB_API_URL_PREFIX")]
        #[arg(value_parser = parse_api_url_prefix)]
        #[arg(help = "Set the URL prefix for GitLab APIs")]
        gitlab_api_url_prefix:    ApiUrlPrefix,
        #[arg(long, visible_aliases = ["api-token"], env = "GITLAB_API_TOKEN")]
        #[arg(value_parser = parse_api_token)]
        #[arg(help = "Set the token of GitLab APIs")]
        gitlab_api_token:         ApiToken,
        #[arg(long)]
        #[arg(help = "Inject the project directory as the first argument to the command")]
        inject_project_directory: bool,
        #[arg(required = true)]
        #[arg(last = true)]
        #[arg(value_hint = clap::ValueHint::CommandWithArguments)]
        #[arg(help = "Command to execute")]
        command:                  Vec<String>,
    },
}

#[inline]
fn parse_commit_sha(arg: &str) -> Result<CommitSha, RegexError> {
    CommitSha::parse_str(arg)
}

#[inline]
fn parse_name(arg: &str) -> Result<Name, RegexError> {
    Name::parse_str(arg)
}

#[inline]
fn parse_phase(arg: &str) -> Result<Phase, RegexError> {
    Phase::parse_str(arg)
}

#[inline]
fn parse_api_url_prefix(arg: &str) -> Result<ApiUrlPrefix, HttpURLError> {
    ApiUrlPrefix::parse_str(arg)
}

#[inline]
fn parse_api_token(arg: &str) -> Result<ApiToken, RegexError> {
    ApiToken::parse_str(arg)
}

#[inline]
fn parse_project_path(arg: &str) -> Result<ProjectPath, LineError> {
    ProjectPath::parse_str(arg)
}

#[inline]
fn parse_reference(arg: &str) -> Result<Reference, LineError> {
    Reference::parse_str(arg)
}

#[inline]
fn parse_ssh_url_prefix(arg: &str) -> Result<SshUrlPrefix, RegexError> {
    SshUrlPrefix::parse_str(arg)
}

#[inline]
fn parse_ssh_user_host(arg: &str) -> anyhow::Result<SshUserHost> {
    SshUserHost::parse_str(arg).map_err(|_| anyhow!("{arg:?} is not a correct SSH user and host"))
}

#[inline]
fn parse_build_target(arg: &str) -> Result<BuildTarget, RegexError> {
    BuildTarget::parse_str(arg)
}

#[inline]
fn parse_command(arg: &str) -> anyhow::Result<Command> {
    Command::parse_str(arg).map_err(|_| anyhow!("{arg:?} is not a correct command"))
}

pub fn get_args() -> CLIArgs {
    let args = CLIArgs::command();

    let about = format!("{APP_NAME} {CARGO_PKG_VERSION}\n{CARGO_PKG_AUTHORS}\n{APP_ABOUT}");

    let args = args.about(about);

    let matches = args.get_matches();

    match CLIArgs::from_arg_matches(&matches) {
        Ok(args) => args,
        Err(err) => {
            err.exit();
        },
    }
}
