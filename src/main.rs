mod error;
mod relay;
mod server;

use std::env;

use clap::{Arg, App, AppSettings};

use crate::server::engine;

#[tokio::main]
async fn main() {

    let matches = build_args().get_matches();

    match matches.subcommand() {
        ("server", Some(server_matches)) => {

            match server_matches.value_of("config") {
                Some(config_path) => engine::launch(config_path).await,
                None => {
                    eprintln!("The watchdog server needs a YAML configuration file to run");
                    eprintln!("Provide a file path with the --config option");
                    std::process::exit(1);
                }
            }
            
        },
        ("relay", Some(relay_matches)) => {

            let base_url = match env::var("WATCHDOG_ADDR") {
                Ok(url_result) => url_result,
                Err(_err) => {
                    eprintln!("Expecting server base URL in the WATCHDOG_ADDR variable");
                    eprintln!("Define an URL such as http://localhost:3030 in an environment variable");
                    std::process::exit(1);
                }
            };
            let token = match env::var("WATCHDOG_TOKEN") {
                Ok(token_result) => token_result,
                Err(_err) => {
                    eprintln!("Expecting server token in the WATCHDOG_TOKEN variable");
                    eprintln!("Define a token such as ******** in an environment variable");
                    std::process::exit(1);
                }
            };

            match relay_matches.value_of("region") {
                Some(region_name) => relay::relay::launch(base_url, token,region_name.to_string()).await,
                None => {
                    eprintln!("Expected relay region");
                    std::process::exit(1)
                }
            }

        },
        ("silence", Some(_)) =>  (),
        ("incident", Some(_)) => (),
        _ => {
            eprintln!("Could not find command to launch");
            std::process::exit(1)
        }
    };
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
        )
}
