use std::time::Duration;
use std::thread::sleep;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::process;

use serde::{Deserialize, Serialize};
use tokio::signal;
use tokio::task;

use crate::server::config::RegionConfig;
use crate::error::RelayError;

#[derive(Deserialize,Serialize)]
pub struct GroupResult {
    pub name: String,
    pub working: bool
}

pub async fn launch(base_url: String, token: String, region_name: String) {
    
    let is_ending = Arc::new(AtomicBool::new(false));
    let is_ending_task = is_ending.clone();

    let region_config = fetch_region_conf(&base_url, &token, &region_name).await.unwrap_or_else(|err| {
        eprintln!("Could not load relay config: {}", err);
        process::exit(1);
    });

    let scheduler_task = task::spawn(async move {

        println!("");
        println!(" ✓ Watchdog relay is now UP");
        println!("");

        loop {

            if is_ending_task.load(Ordering::Relaxed) {
                break;
            }
            
            let mut group_results: Vec<GroupResult> = vec![];
            for group in &region_config.groups {
    
                let mut working = true;
                for test in &group.tests {

                    let test_result = execute_test(test).await;
                    if test_result == false {
                        working = false;
                    }
                }

                group_results.push(GroupResult {
                    name: group.name.clone(),
                    working
                });
            }
            
            update_region_state(&base_url, &token, &region_name, group_results).await;

            sleep(Duration::from_millis(region_config.interval_ms));
        }

    });

    signal::ctrl_c().await.expect("Should handle CTRL+C");

    is_ending.store(true, Ordering::Relaxed);
    scheduler_task.await.unwrap()
}

async fn fetch_region_conf(base_url: &str, token: &str, region_name: &str) -> Result<RegionConfig, RelayError> {

    let config_route = format!("{}/api/v1/relay/{}", base_url, region_name);
    let authorization_header = format!("Bearer {}", token);

    let http_client = reqwest::Client::new();
    let http_response = http_client.get(&config_route)
        .header("Content-Type", "application/json")
        .header("Authorization", &authorization_header)
        .send()
        .await
        .map_err(|err| RelayError {
            message: format!("Could not fetch configuration from server: {}", err)
        })?;

    let body = http_response.text()
        .await
        .map_err(|err| RelayError {
            message: format!("Could not decode configuration from server: {}", err)
        })?;
    
    serde_json::from_str::<RegionConfig>(&body).map_err(|err| RelayError {
        message: format!("{}", err)
    })
}

async fn update_region_state(base_url: &str, token: &str, region_name: &str, group_results: Vec<GroupResult>) -> () {

    let update_route = format!("{}/api/v1/relay/{}", base_url, region_name);
    let authorization_header = format!("Bearer {}", token);

    let http_client = reqwest::Client::new();
    http_client.put(&update_route)
        .header("Content-Type", "application/json")
        .header("Authorization", &authorization_header)
        .body(serde_json::to_string(&group_results).unwrap())
        .send()
        .await.unwrap();
}

async fn execute_test(test: &str) -> bool {

    if test.starts_with("ping") {
        //println!("Execute ping test");
        return true;    // TODO
    }

    if test.starts_with("dns") {
        //println!("Execute DNS test");
        return true;    // TODO
    }

    if test.starts_with("http") {

        let result: Vec<String> = test.split(" ").map(|item| item.to_string()).collect();

        return match result.get(1) {
            Some(domain) => {

                return reqwest::get(format!("http://{}", domain)).await
                    .map(|_response| {
                        //println!("{}", _response.status());
                        return true;
                    })
                    .unwrap_or_else(|err| {
                        eprintln!("{}", err);
                        false
                    });

            },
            None => false
        };
    }

    return false;   // TODO
}