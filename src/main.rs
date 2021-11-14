#[macro_use]
extern crate concat_with;
extern crate clap;
extern crate terminal_size;

extern crate once_cell;
extern crate regex;

#[macro_use]
extern crate validators_derive;
extern crate validators;

extern crate chrono;
extern crate scanner_rust;
extern crate slash_formatter;
extern crate tempfile;
extern crate trim_in_place;

#[macro_use]
extern crate execute;

#[macro_use]
extern crate log;

extern crate simplelog;

mod constants;
mod functions;
mod parse;

mod back_deploy;
mod back_develop;
mod front_control;
mod front_deploy;
mod front_develop;

use std::env;
use std::process;

use clap::{App, Arg, ArgMatches, SubCommand};
use terminal_size::terminal_size;

use back_deploy::*;
use back_develop::*;
use front_control::*;
use front_deploy::*;
use front_develop::*;

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

    if let Some(sub_matches) = matches.subcommand_matches("front-develop") {
        info!("Running {} {} for front-end development", APP_NAME, CARGO_PKG_VERSION);

        if let Err(err) = front_develop(sub_matches) {
            err.to_string().split('\n').for_each(|line| {
                if !line.is_empty() {
                    error!("{}", line);
                }
            });
        }
    } else if let Some(sub_matches) = matches.subcommand_matches("front-deploy") {
        info!("Running {} {} for front-end deployment", APP_NAME, CARGO_PKG_VERSION);

        if let Err(err) = front_deploy(sub_matches) {
            err.to_string().split('\n').for_each(|line| {
                if !line.is_empty() {
                    error!("{}", line);
                }
            });
        }
    } else if let Some(sub_matches) = matches.subcommand_matches("front-control") {
        info!("Running {} {} for front-end control", APP_NAME, CARGO_PKG_VERSION);

        if let Err(err) = front_control(sub_matches) {
            err.to_string().split('\n').for_each(|line| {
                if !line.is_empty() {
                    error!("{}", line);
                }
            });
        }
    } else if let Some(sub_matches) = matches.subcommand_matches("back-develop") {
        info!("Running {} {} for back-end development", APP_NAME, CARGO_PKG_VERSION);

        if let Err(err) = back_develop(sub_matches) {
            err.to_string().split('\n').for_each(|line| {
                if !line.is_empty() {
                    error!("{}", line);
                }
            });
        }
    } else if let Some(sub_matches) = matches.subcommand_matches("back-deploy") {
        info!("Running {} {} for back-end deployment", APP_NAME, CARGO_PKG_VERSION);

        if let Err(err) = back_deploy(sub_matches) {
            err.to_string().split('\n').for_each(|line| {
                if !line.is_empty() {
                    error!("{}", line);
                }
            });
        }
    } else {
        error!("You need to input a subcommand!");
        process::exit(-1);
    }
}

fn get_matches<'a>() -> ArgMatches<'a> {
    let app = App::new(APP_NAME)
        .set_term_width(terminal_size().map(|(width, _)| width.0 as usize).unwrap_or(0))
        .version(CARGO_PKG_VERSION)
        .author(CARGO_PKG_AUTHORS)
        .about(concat!("GitLab Deploy is used for deploying your software projects to multiple hosts into different phases\n\nEXAMPLES:\n", concat_line!(prefix "gitlab-deploy ",
            "front-develop   --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --build-target develop",
            "front-deploy    --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test --build-target test",
            "front-control   --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test",
            "backend-develop --gitlab-project-id 123 --gitlab-project-path magic/website --project-name website --reference develop",
            "backend-deploy  --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test",
            "backend-control --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test --command up",
        )));

    let arg_gitlab_project_id = Arg::with_name("GITLAB_PROJECT_ID")
        .display_order(0)
        .required(true)
        .long("gitlab-project-id")
        .visible_aliases(&["project-id", "id"])
        .takes_value(true)
        .env("CI_PROJECT_ID")
        .help("Sets the ID on GitLab of this project");

    let arg_commit_sha = Arg::with_name("COMMIT_SHA")
        .display_order(1)
        .required(true)
        .long("commit-sha")
        .visible_aliases(&["sha"])
        .takes_value(true)
        .env("CI_COMMIT_SHA")
        .help("Sets the sha of the commit");

    let arg_project_name = Arg::with_name("PROJECT_NAME")
        .display_order(2)
        .required(true)
        .long("project-name")
        .takes_value(true)
        .env("CI_PROJECT_NAME")
        .help("Sets the name of this project");

    let arg_reference_name = Arg::with_name("REFERENCE_NAME")
        .display_order(3)
        .required(true)
        .long("reference-name")
        .takes_value(true)
        .env("CI_COMMIT_REF_NAME")
        .help("Sets the reference name of the commit");

    let arg_gitlab_project_path = Arg::with_name("GITLAB_PROJECT_PATH")
        .display_order(4)
        .required(true)
        .long("gitlab-project-path")
        .visible_aliases(&["project-path"])
        .takes_value(true)
        .env("CI_PROJECT_PATH")
        .help("Sets the path of this project on GitLab");

    let arg_reference = Arg::with_name("REFERENCE")
        .display_order(10)
        .required(true)
        .long("reference")
        .visible_aliases(&["ref"])
        .takes_value(true)
        .env("CI_COMMIT_BRANCH")
        .help("Sets the reference of the commit");

    let arg_build_target = Arg::with_name("BUILD_TARGET")
        .display_order(11)
        .required(true)
        .long("build-target")
        .visible_aliases(&["target"])
        .takes_value(true)
        .help("Sets the target of this build");

    let arg_phase = Arg::with_name("PHASE")
        .display_order(12)
        .required(true)
        .long("phase")
        .visible_aliases(&["phase"])
        .takes_value(true)
        .help("Sets the phase");

    let arg_gitlab_api_url_prefix = Arg::with_name("GITLAB_API_URL_PREFIX")
        .display_order(100)
        .required(true)
        .long("gitlab-api-url-prefix")
        .visible_aliases(&["api-url-prefix"])
        .env("GITLAB_API_URL_PREFIX")
        .help("Sets the URL prefix for GitLab APIs");

    let arg_gitlab_api_token = Arg::with_name("GITLAB_API_TOKEN")
        .display_order(101)
        .required(true)
        .long("gitlab-api-token")
        .visible_aliases(&["api-token"])
        .env("GITLAB_API_TOKEN")
        .help("Sets the token of GitLab APIs");

    let arg_gitlab_ssh_url_prefix = Arg::with_name("GITLAB_SSH_URL_PREFIX")
        .display_order(102)
        .required(true)
        .long("gitlab-ssh-url-prefix")
        .visible_aliases(&["ssh-url-prefix"])
        .env("GITLAB_SSH_URL_PREFIX")
        .help("Sets the SSH URL prefix");

    let arg_develop_ssh_user_host = Arg::with_name("DEVELOP_SSH_USR_HOST")
        .display_order(103)
        .required(true)
        .long("develop-ssh-user-host")
        .visible_aliases(&["ssh-user-host"])
        .env("DEVELOP_SSH_HOST")
        .help("Sets the SSH user, host and the optional port for development");

    let front_develop = SubCommand::with_name("front-develop")
        .about("Fetch the project via GitLab API and then build it and use the public static files on a development host")
        .args(&[
            arg_gitlab_project_id.clone(),
            arg_commit_sha.clone(),
            arg_build_target.clone(),
            arg_gitlab_api_url_prefix.clone(),
            arg_gitlab_api_token.clone(),
            arg_develop_ssh_user_host.clone(),
        ]);

    let front_deploy = SubCommand::with_name("front-deploy")
        .about("Fetch the project via GitLab API and then build it and deploy the archive of public static files on multiple hosts according to the phase")
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

    let front_control = SubCommand::with_name("front-control")
        .about("Control the project on multiple hosts according to the phase")
        .args(&[
            arg_gitlab_project_id.clone(),
            arg_commit_sha.clone(),
            arg_project_name.clone(),
            arg_reference_name.clone(),
            arg_phase.clone(),
        ]);

    let back_develop = SubCommand::with_name("back-develop")
        .about("Fetch the project via Git and checkout to a specific branch and then start up the service on a development host")
        .args(&[
            arg_gitlab_project_id.clone(),
            arg_project_name.clone(),
            arg_gitlab_project_path.clone(),
            arg_reference.clone(),
            arg_gitlab_ssh_url_prefix.clone(),
            arg_develop_ssh_user_host.clone(),
        ]);

    let back_deploy = SubCommand::with_name("back-deploy")
        .about("Fetch the project via GitLab API and then build it and deploy the docker image on multiple hosts according to the phase")
        .args(&[
            arg_gitlab_project_id.clone(),
            arg_commit_sha.clone(),
            arg_project_name.clone(),
            arg_reference_name.clone(),
            arg_phase.clone(),
            arg_gitlab_api_url_prefix.clone(),
            arg_gitlab_api_token.clone(),
        ]);

    let app =
        app.subcommands([front_develop, front_deploy, front_control, back_develop, back_deploy]);

    app.after_help("Enjoy it! https://magiclen.org").get_matches()
}
