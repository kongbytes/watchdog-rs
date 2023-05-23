use std::time::Duration;
use std::thread::sleep;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::signal;
use tokio::task;

use crate::server::config::RegionConfig;
use crate::common::error::Error;

use super::tests::execute_test;

#[derive(Deserialize,Serialize)]
pub struct GroupResultInput {
    pub name: String,
    pub working: bool,
    pub has_warnings: bool,
    pub error_message: Option<String>,
    pub error_detail: Option<String>
}

pub async fn launch(base_url: String, token: String, region_name: String) -> Result<(), Error> {

    let is_ending = Arc::new(AtomicBool::new(false));
    let is_ending_task = is_ending.clone();

    let scheduler_task = task::spawn(async move {

        let mut region_config = match fetch_region_conf(&base_url, &token, &region_name).await {
            Ok(config) => config,
            Err(err) => err.exit(
                "Could not fetch configuration from Watchdog API",
                "Check your token and region name"
            )
        };

        println!();
        println!(" ✓ Watchdog relay is now UP");
        println!(" ✓ Found {} group(s) with a {}ms refresh interval", region_config.groups.len(), region_config.interval_ms, );
        println!();

        let mut last_update = String::new();

        loop {

            if is_ending_task.load(Ordering::Relaxed) {
                break;
            }
            
            let mut group_results: Vec<GroupResultInput> = vec![];
            for group in &region_config.groups {
    
                let mut is_group_working = true;
                let mut has_group_warnings: bool = false;
                let mut error_message = None;
                let mut error_detail = None;

                for test in &group.tests {

                    let test_result = execute_test(test).await;

                    match test_result {
                        Ok(test) => {

                            if !test.is_success {
                                is_group_working = false;
                            }

                            if test.has_warning {
                                has_group_warnings = true;
                            }

                        },
                        Err(err) => {
                            eprintln!("{}", err);
                            is_group_working = false;
                            error_message = Some(err.message);
                            error_detail = err.details;
                        }
                    }
                }

                group_results.push(GroupResultInput {
                    name: group.name.clone(),
                    working: is_group_working,
                    has_warnings: has_group_warnings,
                    error_message,
                    error_detail
                });
            }
            
            let update_result = update_region_state(&base_url, &token, &region_name, group_results, &last_update).await;
            match update_result {
                Ok(Some(watchdog_update)) => {

                    if !last_update.is_empty() {
                        region_config = fetch_region_conf(&base_url, &token, &region_name).await.unwrap();
                        println!("Relay config reloaded");
                    }

                    last_update = watchdog_update;

                },
                Err(update_err) => {
                    eprintln!("{}", update_err);
                },
                _ => {}
            }

            sleep(Duration::from_millis(region_config.interval_ms));
        }

    });

    signal::ctrl_c().await.map_err(|err| Error::new("Could not handle graceful shutdown signal", err))?;

    is_ending.store(true, Ordering::Relaxed);
    scheduler_task.await.map_err(|err| Error::new("Could not end scheduler task", err))?;

    Ok(())
}

async fn fetch_region_conf(base_url: &str, token: &str, region_name: &str) -> Result<RegionConfig, Error> {

    let config_route = format!("{}/api/v1/relay/{}", base_url, region_name);
    let authorization_header = format!("Bearer {}", token);

    let http_client = reqwest::Client::new();
    let http_response = http_client.get(&config_route)
        .header("Content-Type", "application/json")
        .header("Authorization", &authorization_header)
        .send()
        .await
        .map_err(|err| Error::new("Could not fetch configuration from server", err))?;

    if http_response.status() != 200 {
        return Err(
            Error::basic(format!("Expected status code 200, found {}", http_response.status()))
        );
    }

    let body = http_response.text()
        .await
        .map_err(|err| Error::new("Could not decode configuration from server", err))?;
    
    serde_json::from_str::<RegionConfig>(&body).map_err(|err| Error::new("Failed to decode JSON region config", err))
}

async fn update_region_state(base_url: &str, token: &str, region_name: &str, group_results: Vec<GroupResultInput>, last_update: &str) -> Result<Option<String>, Error> {

    let update_route = format!("{}/api/v1/relay/{}", base_url, region_name);
    let authorization_header = format!("Bearer {}", token);

    let json_state = serde_json::to_string(&group_results)
        .map_err(|err| Error::new("Could not parse region state to JSON", err))?;

    let http_client = reqwest::Client::new();
    let response = http_client.put(&update_route)
        .header("Content-Type", "application/json")
        .header("Authorization", &authorization_header)
        .body(json_state)
        .send()
        .await
        .map_err(|err| Error::new("Could not update region state", err))?;

    if let Some(header_value) = response.headers().get("X-Watchdog-Update") {

        let watchdog_update = header_value.to_str().unwrap_or("unknown");

        if watchdog_update != last_update {
            return Ok(Some(watchdog_update.to_string()))
        }
    }

    Ok(None)
}
