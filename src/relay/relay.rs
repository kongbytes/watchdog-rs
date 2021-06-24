use std::time::Duration;
use std::thread::sleep;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::signal;
use tokio::task;

#[derive(Deserialize,Serialize)]
pub struct GroupResult {
    pub name: String,
    pub working: bool
}

pub async fn launch(region_name: &str) {

    let terminate = Arc::new(AtomicBool::new(false));

    let body = reqwest::get(format!("http://localhost:3030/api/v1/relay/{}", region_name))
        .await.unwrap_or_else(|err| {
            eprintln!("Could not fetch configuration from server: {}", err);
            std::process::exit(1);
        })
        .text()
        .await.unwrap_or_else(|err| {
            eprintln!("Could not decode configuration from server: {}", err);
            std::process::exit(1);
        });
    let region_config: crate::server::config::RegionConfig = serde_json::from_str(&body).unwrap();
    println!("{}", region_config.name);

    let scheduler_terminate = terminate.clone();
    let update_route = format!("http://localhost:3030/api/v1/relay/{}", region_name);
    let scheduler_handle = task::spawn(async move {
        
        println!("Spawning relay");
        loop {

            if scheduler_terminate.load(Ordering::Relaxed) {
                break;
            }
            
            let mut group_results: Vec<GroupResult> = vec![];
            for group in region_config.groups.iter() {
    
                let mut working = true;
                for test in group.tests.iter() {

                    let test_result = execute_test(test).await;
                    if test_result == false {
                        working = false;
                    }
                }

                group_results.push(GroupResult {
                    name: group.name.clone(),
                    working
                })
            }
            
            let client = reqwest::Client::new();
            client.put(&update_route)
                .body(serde_json::to_string(&group_results).unwrap())
                .send()
                .await.unwrap();

            sleep(Duration::from_secs(5));
        }

    });

    signal::ctrl_c().await.expect("Should handle CTRL+C");

    terminate.store(true, Ordering::Relaxed);

    scheduler_handle.await.expect("Should end scheduler task");
}

async fn execute_test(test: &str) -> bool {

    if test.starts_with("ping") {
        //println!("Execute ping test");
        return false;    // TODO
    }

    if test.starts_with("dns") {
        //println!("Execute DNS test");
        return false;    // TODO
    }

    if test.starts_with("http") {

        let result: Vec<String> = test.split(" ").map(|item| item.to_string()).collect();

        return match result.get(1) {
            Some(domain) => {
                return reqwest::get(format!("http://{}/", domain)).await
                    .map(|_response| true)
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