use std::vec;

use crate::common::error::Error;
use crate::server::config::{ConfigInput, RegionConfigInput, GroupConfigInput};

pub fn init_config() -> Result<(), Error> {

    println!();
    println!("The Watchdog init process will generate a default configuration file.");
    println!("Enter your network region names below and leave empty to end the process.");
    println!();
    println!("A \"network region\" refers to a dedicated network location. For example:");
    println!(" - A region named \"region-north\" with range 10.20.0.0/24");
    println!(" - A region named \"region-south\" with range 10.50.0.0/22");

    let mut config = ConfigInput {
        regions: vec![]
    };

    loop {

        let region_name = request_user_input("Enter region name:");
        if region_name.is_empty() {
            break;
        }

        config.regions.push(RegionConfigInput {
            groups: vec![GroupConfigInput {
                name: "default".to_string(),
                tests: vec![
                    "ping 1.1.1.1".to_string(),
                    "dns example.org".to_string(),
                    "http example.org".to_string()
                ],
                mediums: "telegram".to_string(),
                threshold: 4
            }],
            name: region_name,
            interval: "5s".to_string(),
            threshold: 3
        })
    }

    let yaml_content = serde_yaml::to_string(&config)?;
    println!("{}", yaml_content);

    Ok(())
}

fn request_user_input<M>(message: M) -> String where M: Into<String> {

    println!();
    println!("{}", message.into());

    let mut buffer = String::new();
    let stdin = std::io::stdin(); // We get `Stdin` here.
    stdin.read_line(&mut buffer).unwrap();

    buffer.trim().to_string()
}
