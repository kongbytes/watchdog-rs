use std::{str, collections::HashMap};
use tokio::time::Instant;

use reqwest::Client;

use crate::{common::error::Error, relay::model::{TestResult, ResultCategory}};

pub struct HttpTest {
    client: Client
}

impl HttpTest {

    pub fn new() -> Self {

        HttpTest {
            client: Client::new()
        }
    }

    pub fn matches(&self, test: &str) -> bool {

        test.starts_with("http")
    }

    pub async fn execute(&self, test: &str) -> Result<TestResult, Error> {

        let result: Vec<String> = test.split(' ').map(|item| item.to_string()).collect();
    
        return match result.get(1) {
            Some(domain) => {

                let url = format!("http://{}", domain);
                let builder = self.client.get(url)
                    .header("user-agent", "watchdog-relay")
                    .header("cache-control", "no-store");

                // Measure the time between the request sent out time and the first byte
                // received time (not 100% accurate - but still reasonable workaround)
                let latency_chrono = Instant::now();
                let request_result = builder.send().await;
                let duration = latency_chrono.elapsed();

                match request_result {
                    Ok(response) => {

                        let http_status = &response.status();

                        let category = if http_status.is_client_error() || http_status.is_server_error() {
                            ResultCategory::Warning
                        } else {
                            ResultCategory::Success
                        };

                        let duration_ms: f32 = duration.as_millis() as f32;

                        let metrics: HashMap<String, f32> = HashMap::from([
                            ("http_latency".to_string(), duration_ms)
                        ]);

                        return Ok(TestResult::build(domain, category, Some(metrics)));

                    },
                    Err(_err) => {
                        // TODO Error lost (DNS failure, ...)
                        Ok(TestResult::fail(domain))
                    }
                }
            },
            None => {
                let error_message = Error::new("HTTP test failed", "The HTTP command expects a target"); 
                Err(error_message)
            }
        };
    }

}
