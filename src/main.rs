mod common;
mod relay;
mod server;
mod cli;

use std::env;
use std::process;

use clap::{Arg, Command};

use crate::server::engine;
use crate::relay::instance;
use crate::cli::{incident, status, init};
use crate::common::error::Error;

// TODO Should not launch Tokio for CLI
#[tokio::main]
async fn main() {

    let matches = build_args().get_matches();

    match matches.subcommand() {
        Some(("init", _)) =>  {

            let cli_result = init::init_config();
            handle_cli_failure(cli_result);

        },
        Some(("server", server_matches)) => {

            match server_matches.get_one::<String>("config") {
                Some(config_path) => {

                    let port: u16 = match server_matches.get_one::<String>("port") {
                        Some(port) => port.parse().unwrap_or(engine::DEFAULT_PORT),
                        None => engine::DEFAULT_PORT
                    };
                    let token: String = env::var("WATCHDOG_TOKEN").ok().unwrap_or_else(|| {
                        eprintln!("Expecting a WATCHDOG_TOKEN environment variable for API authentication");
                        process::exit(1);
                    });

                    let server_conf = engine::ServerConf {
                        config_path: config_path.to_string(),
                        port,
                        token,
                        telegram_token: env::var("TELEGRAM_TOKEN").ok(),
                        telegram_chat: env::var("TELEGRAM_CHAT").ok()
                    };

                    let server_result = engine::launch(server_conf).await;

                    if let Err(server_err) = server_result {
                        eprintln!("The watchdog server process failed, see details below");
                        eprintln!("{}", server_err);
                        if let Some(err_details) = server_err.details {
                            eprintln!("{}", err_details);
                        }
                        process::exit(1);
                    }                    

                }
                None => {
                    eprintln!("The watchdog server needs a YAML configuration file to run");
                    eprintln!("Provide a file path with the --config option");
                    process::exit(1);
                }
            };
            
        },
        Some(("relay", relay_matches)) => {

            let (base_url, token) = extract_watchdog_env_or_fail();

            match relay_matches.get_one::<String>("region") {
                Some(region_name) => {

                    let relay_result = instance::launch(base_url, token, region_name.to_string()).await;

                    if let Err(relay_err) = relay_result {
                        eprintln!("The watchdog relay process failed, see details below");
                        eprintln!("{}", relay_err);
                        if let Some(err_details) = relay_err.details {
                            eprintln!("{}", err_details);
                        }
                        process::exit(1);
                    }

                },
                None => {
                    eprintln!("Expected relay region");
                    process::exit(1)
                }
            };

        },
        Some(("status", _)) =>  {

            let (base_url, token) = extract_watchdog_env_or_fail();

            let cli_result = status::display_status(&base_url, &token).await;
            handle_cli_failure(cli_result);

        },
        Some(("incident", incident_matches)) => {

            let (base_url, token) = extract_watchdog_env_or_fail();

            match incident_matches.subcommand() {
                Some(("ls", _)) => {
                    let cli_result = incident::list_incidents(&base_url, &token).await;
                    handle_cli_failure(cli_result);
                },
                Some(("get", get_command)) => {
                    let incident_id = get_command.subcommand_name().expect("Expecting at least an incident ID");
                    let cli_result = incident::inspect_incident(&base_url, &token, incident_id).await;
                    handle_cli_failure(cli_result);
                },
                _ => {
                    eprintln!("Could not find command to launch");
                    process::exit(1)
                }
            }

        },
        _ => {
            eprintln!("Could not find command to launch");
            process::exit(1)
        }
    };
}

fn extract_watchdog_env_or_fail() -> (String, String) {

    let base_url = match env::var("WATCHDOG_ADDR") {
        Ok(url_result) => url_result,
        Err(_err) => {
            eprintln!("Expecting server base URL in the WATCHDOG_ADDR variable");
            eprintln!("Define an URL such as http://localhost:3030 in an environment variable");
            process::exit(1);
        }
    };
    let token = match env::var("WATCHDOG_TOKEN") {
        Ok(token_result) => token_result,
        Err(_err) => {
            eprintln!("Expecting server token in the WATCHDOG_TOKEN variable");
            eprintln!("Define a token such as ******** in an environment variable");
            process::exit(1);
        }
    };

    (base_url, token)
}

fn handle_cli_failure(cli_result: Result<(), Error>) {

    if let Err(cli_error) = cli_result {
        eprintln!("The watchdog command failed, see details below");
        eprintln!("{}", cli_error);
        if let Some(err_details) = cli_error.details {
            eprintln!("{}", err_details);
        }
        process::exit(1);
    }
}

fn build_args() -> clap::Command {

    Command::new("Network watchdog")
        .version("0.2.0")
        .about("Detect network incidents accross regions")
        .arg_required_else_help(true)
        .subcommand(Command::new("init")
            .about("Initialize config")
        )
        .subcommand(Command::new("server")
            .about("Launch server daemon")
            .arg(Arg::new("config")
                .short('c')
                .long("config")
                .help("YAML config path")
            )
            .arg(Arg::new("port")
                .short('p')
                .long("port")
                .help("TCP port used by the server"))
        )
        .subcommand(Command::new("relay")
            .about("Launch relay daemon")
            .arg(Arg::new("region")
                .short('r')
                .long("region")
                .help("Network region covered by relay")
            )
        )
        .subcommand(Command::new("status")
            .about("Status overview for all regions")
        )
        .subcommand(Command::new("incident")
            .about("Manage incident history")
            .arg_required_else_help(true)
            .subcommand(
                Command::new("ls")
                    .about("List all incidents")
            )
            .subcommand(
                Command::new("get")
                    .about("Get & inspect an incident")
                    .allow_external_subcommands(true)
                    .arg_required_else_help(true)
                    .alias("get")
            )
        )
}
