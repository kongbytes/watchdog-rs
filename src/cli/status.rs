use chrono::prelude::*;
use ansi_term::{Colour, Style};

use crate::common::error::Error;
use crate::server::storage::RegionSummary;
use super::utils::api_get;

pub async fn display_status(base_url: &str, token: &str) -> Result<(), Error> {

    let region_summary: RegionSummary = api_get(base_url, token, "api/v1/analytics").await?;

    let bold = Style::new().bold();

    println!();
    println!("{}            {}               {}", bold.paint("Region & groups"), bold.paint("Status"), bold.paint("Last update (success)"));
    println!();

    for region_item in region_summary.regions.iter() {

        let formatted_date: String = match &(region_item.last_update).parse::<DateTime<Utc>>() {
            Ok(date) => date.format("%Y-%m-%d %H:%M:%S").to_string(),
            Err(_) => region_item.last_update.to_string()
        };

        let region_status: String = match region_item.status.as_str() {
            "initial" => format!("{}  INITIAL", Colour::Blue.paint("◼")),
            "up" => format!("{}  UP", Colour::Green.paint("◼")),
            "warn" => format!("{}  WARN", Colour::Yellow.paint("◼")),
            "down" => format!("{}  DOWN", Colour::Red.paint("◼")),
            _ => format!("{}  UNKNOWN", Colour::Purple.paint("◼")) 
        };

        println!("Region {: <n_max$}{: <s_max$}{: <d_max$}", region_item.name, region_status, formatted_date, n_max=20, s_max=30, d_max=20);

        for group in region_summary.groups.iter() {

            if !group.name.starts_with(&region_item.name) {
                continue;
            }

            let group_name = match group.name.split('.').last() {
                Some(name) => format!("Group {}", name),
                None => group.name.to_string()
            };

            let group_status: String = match group.status.as_str() {
                "initial" => format!("{}  INITIAL", Colour::Blue.paint("◼")),
                "up" => format!("{}  UP", Colour::Green.paint("◼")),
                "warn" => format!("{} WARN", Colour::Yellow.paint("◼")),
                "incident" => format!("{}  INCIDENT", Colour::Red.paint("◼")),
                "down" => format!("{}  DOWN", Colour::Red.paint("◼")),
                _ => format!("{}  UNKNOWN", Colour::Purple.paint("◼")) 
            };

            println!(" - {: <n_max$}{: <s_max$}", group_name, group_status, n_max=24, s_max=30);
            
        }
        println!();
    }

    Ok(())
}
