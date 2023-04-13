#[macro_use]
extern crate log;

mod constants;
mod functions;
mod parse;

mod back_control;
mod back_deploy;
mod back_develop;
mod front_control;
mod front_deploy;
mod front_develop;
mod simple_control;
mod simple_deploy;

use std::{env, process};

use back_control::*;
use back_deploy::*;
use back_develop::*;
use clap::{Arg, ArgMatches, Command};
use concat_with::concat_line;
use front_control::*;
use front_deploy::*;
use front_develop::*;
use simple_control::*;
use simple_deploy::*;
use terminal_size::terminal_size;

const APP_NAME: &str = "gitlab-deploy";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

fn main() {
    let mut log_config = simplelog::ConfigBuilder::new();

    log_config.set_time_level(simplelog::LevelFilter::Debug);

    simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,
        log_config.build(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    let matches = get_matches();

    if let Some(sub_matches) = matches.subcommand_matches("frontend-develop") {
        info!("Running {} {} for front-end development", APP_NAME, CARGO_PKG_VERSION);

        if let Err(err) = front_develop(sub_matches) {
            err.to_string().split('\n').for_each(|line| {
                if !line.is_empty() {
                    error!("{}", line);
                }
            });

            process::exit(-3);
        }
    } else if let Some(sub_matches) = matches.subcommand_matches("frontend-deploy") {
        info!("Running {} {} for front-end deployment", APP_NAME, CARGO_PKG_VERSION);

        if let Err(err) = front_deploy(sub_matches) {
            err.to_string().split('\n').for_each(|line| {
                if !line.is_empty() {
                    error!("{}", line);
                }
            });

            process::exit(-3);
        }
    } else if let Some(sub_matches) = matches.subcommand_matches("frontend-control") {
        info!("Running {} {} for front-end control", APP_NAME, CARGO_PKG_VERSION);

        if let Err(err) = front_control(sub_matches) {
            err.to_string().split('\n').for_each(|line| {
                if !line.is_empty() {
                    error!("{}", line);
                }
            });

            process::exit(-3);
        }
    } else if let Some(sub_matches) = matches.subcommand_matches("backend-develop") {
        info!("Running {} {} for back-end development", APP_NAME, CARGO_PKG_VERSION);

        if let Err(err) = back_develop(sub_matches) {
            err.to_string().split('\n').for_each(|line| {
                if !line.is_empty() {
                    error!("{}", line);
                }
            });

            process::exit(-3);
        }
    } else if let Some(sub_matches) = matches.subcommand_matches("backend-deploy") {
        info!("Running {} {} for back-end deployment", APP_NAME, CARGO_PKG_VERSION);

        if let Err(err) = back_deploy(sub_matches) {
            err.to_string().split('\n').for_each(|line| {
                if !line.is_empty() {
                    error!("{}", line);
                }
            });

            process::exit(-3);
        }
    } else if let Some(sub_matches) = matches.subcommand_matches("backend-control") {
        info!("Running {} {} for back-end control", APP_NAME, CARGO_PKG_VERSION);

        if let Err(err) = back_control(sub_matches) {
            err.to_string().split('\n').for_each(|line| {
                if !line.is_empty() {
                    error!("{}", line);
                }
            });

            process::exit(-3);
        }
    } else if let Some(sub_matches) = matches.subcommand_matches("simple-deploy") {
        info!("Running {} {} for simple deployment", APP_NAME, CARGO_PKG_VERSION);

        if let Err(err) = simple_deploy(sub_matches) {
            err.to_string().split('\n').for_each(|line| {
                if !line.is_empty() {
                    error!("{}", line);
                }
            });

            process::exit(-3);
        }
    } else if let Some(sub_matches) = matches.subcommand_matches("simple-control") {
        info!("Running {} {} for simple control", APP_NAME, CARGO_PKG_VERSION);

        if let Err(err) = simple_control(sub_matches) {
            err.to_string().split('\n').for_each(|line| {
                if !line.is_empty() {
                    error!("{}", line);
                }
            });

            process::exit(-3);
        }
    } else {
        error!("You need to input a subcommand!");
        process::exit(-1);
    }
}

fn get_matches() -> ArgMatches {
    let app = Command::new(APP_NAME)
        .term_width(terminal_size().map(|(width, _)| width.0 as usize).unwrap_or(0))
        .version(CARGO_PKG_VERSION)
        .author(CARGO_PKG_AUTHORS)
        .about(concat!("GitLab Deploy is used for deploying software projects to multiple hosts during different phases\n\nEXAMPLES:\n", concat_line!(prefix "gitlab-deploy ",
            "frontend-develop   --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --build-target develop",
            "frontend-deploy    --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test --build-target test",
            "frontend-control   --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test",
            "backend-develop    --gitlab-project-id 123 --gitlab-project-path website-api                     --project-name website --reference develop",
            "backend-deploy     --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test",
            "backend-control    --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test --command up",
            "simple-deploy      --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test",
            "simple-control     --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test sudo /usr/local/bin/apply-nginx.sh dev.env",
        )));

    let arg_gitlab_project_id = Arg::new("GITLAB_PROJECT_ID")
        .display_order(0)
        .required(true)
        .long("gitlab-project-id")
        .visible_aliases(&["project-id", "id"])
        .takes_value(true)
        .env("CI_PROJECT_ID")
        .help("Set the ID on GitLab of this project");

    let arg_commit_sha = Arg::new("COMMIT_SHA")
        .display_order(1)
        .required(true)
        .long("commit-sha")
        .visible_aliases(&["sha"])
        .takes_value(true)
        .env("CI_COMMIT_SHA")
        .help("Set the sha of the commit");

    let arg_project_name = Arg::new("PROJECT_NAME")
        .display_order(2)
        .required(true)
        .long("project-name")
        .takes_value(true)
        .env("CI_PROJECT_NAME")
        .help("Set the name of this project");

    let arg_reference_name = Arg::new("REFERENCE_NAME")
        .display_order(3)
        .required(true)
        .long("reference-name")
        .takes_value(true)
        .env("CI_COMMIT_REF_NAME")
        .help("Set the reference name of the commit");

    let arg_gitlab_project_path = Arg::new("GITLAB_PROJECT_PATH")
        .display_order(4)
        .required(true)
        .long("gitlab-project-path")
        .visible_aliases(&["project-path"])
        .takes_value(true)
        .env("CI_PROJECT_PATH")
        .help("Set the path of this project on GitLab");

    let arg_reference = Arg::new("REFERENCE")
        .display_order(10)
        .required(true)
        .long("reference")
        .visible_aliases(&["ref"])
        .takes_value(true)
        .env("CI_COMMIT_BRANCH")
        .help("Set the reference of the commit");

    let arg_build_target = Arg::new("BUILD_TARGET")
        .display_order(11)
        .required(true)
        .long("build-target")
        .visible_aliases(&["target"])
        .takes_value(true)
        .help("Set the target of this build");

    let arg_build_target_allow_null = Arg::new("BUILD_TARGET")
        .display_order(11)
        .long("build-target")
        .visible_aliases(&["target"])
        .takes_value(true)
        .help("Set the target of this build");

    let arg_phase = Arg::new("PHASE")
        .display_order(12)
        .required(true)
        .long("phase")
        .visible_aliases(&["phase"])
        .takes_value(true)
        .help("Set the phase");

    let arg_command = Arg::new("COMMAND")
        .display_order(13)
        .required(true)
        .long("command")
        .takes_value(true)
        .help("Set the command");

    let arg_gitlab_api_url_prefix = Arg::new("GITLAB_API_URL_PREFIX")
        .display_order(100)
        .required(true)
        .long("gitlab-api-url-prefix")
        .visible_aliases(&["api-url-prefix"])
        .env("GITLAB_API_URL_PREFIX")
        .help("Set the URL prefix for GitLab APIs");

    let arg_gitlab_api_token = Arg::new("GITLAB_API_TOKEN")
        .display_order(101)
        .required(true)
        .long("gitlab-api-token")
        .visible_aliases(&["api-token"])
        .env("GITLAB_API_TOKEN")
        .help("Set the token of GitLab APIs");

    let arg_gitlab_ssh_url_prefix = Arg::new("GITLAB_SSH_URL_PREFIX")
        .display_order(102)
        .required(true)
        .long("gitlab-ssh-url-prefix")
        .visible_aliases(&["ssh-url-prefix"])
        .env("GITLAB_SSH_URL_PREFIX")
        .help("Set the SSH URL prefix");

    let arg_develop_ssh_user_host = Arg::new("DEVELOP_SSH_USR_HOST")
        .display_order(103)
        .required(true)
        .long("develop-ssh-user-host")
        .visible_aliases(&["ssh-user-host"])
        .env("DEVELOP_SSH_HOST")
        .help("Set the SSH user, host and the optional port for development");

    let arg_command_arg =
        Arg::new("COMMAND").required(true).help("Command to execute").multiple_values(true);

    let arg_inject_project_directory = Arg::new("INJECT_PROJECT_DIRECTORY")
        .display_order(1000)
        .long("inject-project-directory")
        .help("Inject the project directory as the first argument to the command");

    let front_develop = Command::new("frontend-develop")
        .display_order(10)
        .about(
            "Fetch the project via GitLab API and then build it and use the public static files \
             on a development host",
        )
        .args(&[
            arg_gitlab_project_id.clone(),
            arg_commit_sha.clone(),
            arg_build_target.clone(),
            arg_gitlab_api_url_prefix.clone(),
            arg_gitlab_api_token.clone(),
            arg_develop_ssh_user_host.clone(),
        ]);

    let front_deploy = Command::new("frontend-deploy")
        .display_order(11)
        .about(
            "Fetch the project via GitLab API and then build it and deploy the archive of public \
             static files on multiple hosts according to the phase",
        )
        .args(&[
            arg_gitlab_project_id.clone(),
            arg_commit_sha.clone(),
            arg_project_name.clone(),
            arg_reference_name.clone(),
            arg_build_target.clone(),
            arg_phase.clone(),
            arg_gitlab_api_url_prefix.clone(),
            arg_gitlab_api_token.clone(),
        ]);

    let front_control = Command::new("frontend-control")
        .display_order(12)
        .about("Control the project on multiple hosts according to the phase")
        .args(&[
            arg_gitlab_project_id.clone(),
            arg_commit_sha.clone(),
            arg_project_name.clone(),
            arg_reference_name.clone(),
            arg_phase.clone(),
        ]);

    let back_develop = Command::new("backend-develop")
        .display_order(13)
        .about(
            "Fetch the project via Git and checkout to a specific branch and then start up the \
             service on a development host",
        )
        .args(&[
            arg_gitlab_project_id.clone(),
            arg_project_name.clone(),
            arg_gitlab_project_path.clone(),
            arg_reference.clone(),
            arg_gitlab_ssh_url_prefix.clone(),
            arg_develop_ssh_user_host.clone(),
        ]);

    let back_deploy = Command::new("backend-deploy")
        .display_order(14)
        .about(
            "Fetch the project via GitLab API and then build it and deploy the docker image on \
             multiple hosts according to the phase",
        )
        .args(&[
            arg_gitlab_project_id.clone(),
            arg_commit_sha.clone(),
            arg_project_name.clone(),
            arg_reference_name.clone(),
            arg_build_target_allow_null.clone(),
            arg_phase.clone(),
            arg_gitlab_api_url_prefix.clone(),
            arg_gitlab_api_token.clone(),
        ]);

    let back_control = Command::new("backend-control")
        .display_order(15)
        .about("Control the project on multiple hosts according to the phase")
        .args(&[
            arg_gitlab_project_id.clone(),
            arg_commit_sha.clone(),
            arg_project_name.clone(),
            arg_reference_name.clone(),
            arg_phase.clone(),
            arg_command.clone(),
        ]);

    let simple_deploy = Command::new("simple-deploy")
        .display_order(16)
        .about(
            "Fetch the project via GitLab API and deploy the project files on multiple hosts \
             according to the phase",
        )
        .args(&[
            arg_gitlab_project_id.clone(),
            arg_commit_sha.clone(),
            arg_project_name.clone(),
            arg_reference_name.clone(),
            arg_phase.clone(),
            arg_gitlab_api_url_prefix.clone(),
            arg_gitlab_api_token.clone(),
        ]);

    let simple_control = Command::new("simple-control")
        .display_order(17)
        .about("Control the project on multiple hosts according to the phase")
        .args(&[
            arg_gitlab_project_id.clone(),
            arg_commit_sha.clone(),
            arg_project_name.clone(),
            arg_reference_name.clone(),
            arg_phase.clone(),
            arg_gitlab_api_url_prefix.clone(),
            arg_gitlab_api_token.clone(),
            arg_inject_project_directory.clone(),
            arg_command_arg.clone(),
        ]);

    let app = app.subcommands([
        front_develop,
        front_deploy,
        front_control,
        back_develop,
        back_deploy,
        back_control,
        simple_deploy,
        simple_control,
    ]);

    app.after_help("Enjoy it! https://magiclen.org").get_matches()
}
