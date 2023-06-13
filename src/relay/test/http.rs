use std::str;

use reqwest::Client;

use crate::{common::error::Error, relay::model::TestResult};

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
                let request_result = self.client.get(url)
                    .header("user-agent", "watchdog-relay")
                    .header("cache-control", "no-store")
                    .send()
                    .await;

                match request_result {
                    Ok(response) => {

                        let http_status = &response.status();
                        if http_status.is_client_error() || http_status.is_server_error() {
                            return Ok(TestResult::warning(domain));
                        }

                        return Ok(TestResult::success(domain));

                    },
                    Err(_) => Ok(TestResult::fail(domain))
                }
            },
            None => {
                let error_message = Error::new("HTTP test failed", "The HTTP command expects a target"); 
                Err(error_message)
            }
        };
    }

}
