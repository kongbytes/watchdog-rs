use std::time::Duration;
use std::thread::sleep;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::signal;
use tokio::task;
use tokio::process::Command;
use reqwest::Client;

use crate::server::config::RegionConfig;
use crate::common::error::Error;

#[derive(Deserialize,Serialize)]
pub struct GroupResult {
    pub name: String,
    pub working: bool
}

pub async fn launch(base_url: String, token: String, region_name: String) -> Result<(), Error> {
    
    let region_config = fetch_region_conf(&base_url, &token, &region_name).await?;

    let is_ending = Arc::new(AtomicBool::new(false));
    let is_ending_task = is_ending.clone();

    let scheduler_task = task::spawn(async move {

        println!();
        println!(" ✓ Watchdog relay is now UP");
        println!(" ✓ Found {} group(s) with a {}ms refresh interval", region_config.groups.len(), region_config.interval_ms, );
        println!();

        loop {

            if is_ending_task.load(Ordering::Relaxed) {
                break;
            }
            
            let mut group_results: Vec<GroupResult> = vec![];
            for group in &region_config.groups {
    
                let mut is_group_working = true;
                for test in &group.tests {

                    let test_result = execute_test(test).await;
                    if !test_result {
                        is_group_working = false;
                    }
                }

                group_results.push(GroupResult {
                    name: group.name.clone(),
                    working: is_group_working
                });
            }
            
            let update_result = update_region_state(&base_url, &token, &region_name, group_results).await;
            if let Err(update_err) = update_result {
                eprintln!("{}", update_err);
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

    let body = http_response.text()
        .await
        .map_err(|err| Error::new("Could not decode configuration from server", err))?;
    
    serde_json::from_str::<RegionConfig>(&body).map_err(|err| Error::new("Failed to decode JSON region config", err))
}

async fn update_region_state(base_url: &str, token: &str, region_name: &str, group_results: Vec<GroupResult>) -> Result<(), Error> {

    let update_route = format!("{}/api/v1/relay/{}", base_url, region_name);
    let authorization_header = format!("Bearer {}", token);

    let json_state = serde_json::to_string(&group_results)
        .map_err(|err| Error::new("Could not parse region state to JSON", err))?;

    let http_client = reqwest::Client::new();
    http_client.put(&update_route)
        .header("Content-Type", "application/json")
        .header("Authorization", &authorization_header)
        .body(json_state)
        .send()
        .await
        .map_err(|err| Error::new("Could not update region state", err))?;

    Ok(())
}

async fn execute_test(test: &str) -> bool {

    if test.starts_with("ping") {

        let ping_components: Vec<String> = test.split(' ').map(|item| item.to_string()).collect();

        return match ping_components.get(1) {
            Some(ip_address) => {

                let is_success = Command::new("/usr/bin/ping")
                    .arg("-c")
                    .arg("1")
                    .arg("-w")
                    .arg("2")
                    .arg(ip_address)
                    .stdout(Stdio::null())
                    .status()
                    .await
                    .map(|status| status.success())
                    .unwrap_or(false);

                return is_success;
            }
            None => false
        }
    }

    if test.starts_with("dns") {
        //println!("Execute DNS test");
        return true;    // TODO
    }

    if test.starts_with("http") {

        let result: Vec<String> = test.split(' ').map(|item| item.to_string()).collect();

        return match result.get(1) {
            Some(domain) => {

                let client = Client::new();
                let url = format!("http://{}", domain);
                let request_result = client.get(url)
                    .header("user-agent", "watchdog-relay")
                    .header("cache-control", "no-cache")
                    .send()
                    .await;

                match request_result {
                    Ok(response) => {

                        let http_status = &response.status();
                        if http_status.is_client_error() || http_status.is_server_error() {
                            return false;
                        }

                        return true;

                    },
                    Err(_) => false
                }
            },
            None => false
        };
    }

    return false;   // TODO
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn should_request_http_domain() {
        
        assert_eq!(execute_test("http lasemo.be").await, true);
    }

    #[tokio::test]
    async fn should_request_http_path() {
        
        assert_eq!(execute_test("http www.lasemo.be/mentions-legales").await, true);
    }

    #[tokio::test]
    async fn should_fail_http_invalid_domain() {
        
        assert_eq!(execute_test("http www.lasemo-does-not-exist.be").await, false);
    }

    #[tokio::test]
    async fn should_fail_http_unknown_page() {
        
        assert_eq!(execute_test("http www.lasemo.be/unknown").await, false);
    }

    #[tokio::test]
    async fn should_perform_valid_ping() {
        
        assert_eq!(execute_test("ping 1.1.1.1").await, true);
    }

    #[tokio::test]
    async fn should_fail_invalid_ping() {
        
        assert_eq!(execute_test("ping 10.99.99.99").await, false);
    }

    #[tokio::test]
    async fn should_fail_unknown_test_type() {
        
        assert_eq!(execute_test("unknown").await, false);
    }

    #[tokio::test]
    async fn should_fail_empty_test() {
        
        assert_eq!(execute_test("").await, false);
    }

}
