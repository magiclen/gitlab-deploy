mod cli;

mod constants;
mod functions;
mod models;

mod back_control;
mod back_deploy;
mod back_develop;
mod front_control;
mod front_deploy;
mod front_develop;
mod simple_control;
mod simple_deploy;

use back_control::*;
use back_deploy::*;
use back_develop::*;
use cli::*;
use front_control::*;
use front_deploy::*;
use front_develop::*;
use simple_control::*;
use simple_deploy::*;

fn main() -> anyhow::Result<()> {
    let args = get_args();

    let mut log_config = simplelog::ConfigBuilder::new();

    log_config.set_time_level(simplelog::LevelFilter::Debug);

    simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,
        log_config.build(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    match &args.command {
        CLICommands::FrontendDevelop {
            ..
        } => {
            front_develop(args)?;
        },
        CLICommands::FrontendDeploy {
            ..
        } => {
            front_deploy(args)?;
        },
        CLICommands::FrontendControl {
            ..
        } => {
            front_control(args)?;
        },
        CLICommands::BackendDevelop {
            ..
        } => {
            back_develop(args)?;
        },
        CLICommands::BackendDeploy {
            ..
        } => {
            back_deploy(args)?;
        },
        CLICommands::BackendControl {
            ..
        } => {
            back_control(args)?;
        },
        CLICommands::SimpleDeploy {
            ..
        } => {
            simple_deploy(args)?;
        },
        CLICommands::SimpleControl {
            ..
        } => {
            simple_control(args)?;
        },
    }

    Ok(())
}
