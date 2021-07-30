mod common;
mod relay;
mod server;
mod cli;

use std::env;
use std::process;

use clap::{Arg, App, AppSettings};

use crate::server::engine;
use crate::relay::instance;
use crate::cli::incident;
use crate::common::error::Error;

#[tokio::main]
async fn main() {

    let matches = build_args().get_matches();

    match matches.subcommand() {
        ("server", Some(server_matches)) => {

            match server_matches.value_of("config") {
                Some(config_path) => {

                    let port: u16 = match server_matches.value_of("port") {
                        Some(port) => port.parse().unwrap_or(engine::DEFAULT_PORT),
                        None => engine::DEFAULT_PORT
                    };

                    let server_conf = engine::ServerConf {
                        config_path: config_path.to_string(),
                        port,
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
        ("relay", Some(relay_matches)) => {

            let (base_url, token) = extract_watchdog_env_or_fail();

            match relay_matches.value_of("region") {
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
        ("silence", Some(_)) =>  (),
        ("incident", Some(incident_matches)) => {

            let (base_url, token) = extract_watchdog_env_or_fail();
            let mut cli_result: Result<(), Error> = Ok(());

            if let Some(_) = incident_matches.subcommand_matches("ls") {
                cli_result = incident::list_incidents(&base_url, &token).await;
            }
            else if let Some(x) = incident_matches.subcommand_matches("inspect") {
                dbg!(&x.args);
                cli_result = incident::inspect_incident("TODO").await;
            }

            if let Err(cli_error) = cli_result {
                eprintln!("The incident CLI failed, see details below");
                eprintln!("{}", cli_error);
                if let Some(err_details) = cli_error.details {
                    eprintln!("{}", err_details);
                }
                process::exit(1);
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

fn build_args<'a, 'b>() -> clap::App<'a, 'b> {

    App::new("Network watchdog")
        .version("0.1.0")
        .about("Detect network incidents accross regions")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(App::new("server")
            .about("Launch server daemon")
            .arg(Arg::with_name("config")
                .short("c")
                .long("config")
                .takes_value(true)
                .help("YAML config path")
            )
            .arg(Arg::with_name("port")
                .short("p")
                .long("port")
                .takes_value(true)
                .help("TCP port used by the server"))
        )
        .subcommand(App::new("relay")
            .about("Launch relay daemon")
            .arg(Arg::with_name("region")
                .short("r")
                .long("region")
                .takes_value(true)
                .help("Network region covered by relay")
            )
        )
        .subcommand(App::new("silence")
            .about("Manage alert silences")
        )
        .subcommand(App::new("incident")
            .about("Manage incident history")
            .setting(AppSettings::ArgRequiredElseHelp)
            .subcommand(
                App::new("ls")
                    .about("List all incidents")
            )
            .subcommand(
                App::new("inspect")
                    .about("Inspect an incident")
                    .alias("i")
            )
        )
}
