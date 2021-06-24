use clap::{Arg, App};

mod error;
mod relay;
mod server;

#[tokio::main]
async fn main() {

    let matches = build_args().get_matches();

    match matches.subcommand() {
        ("server", Some(server_matches)) => {

            match server_matches.value_of("config") {
                Some(config_path) => server::server::launch(config_path).await,
                None => {
                    eprintln!("Expected config path");
                    std::process::exit(1)
                }
            }
            
        },
        ("relay", Some(relay_matches)) => {

            // TODO
            let base_url = "http://localhost:3030".to_string();
            let token = "secret".to_string();

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
        .subcommand(App::new("server")
            .about("Launch server daemon")
            .arg(Arg::with_name("config")
                .short("c")
                .long("conf")
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
