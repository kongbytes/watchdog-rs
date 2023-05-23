use std::process::Stdio;

use tokio::process::Command;
use reqwest::Client;

use crate::common::error::Error;

#[derive(PartialEq, Debug)]
pub struct TestResult {
    pub is_success: bool,
    pub has_warning: bool
}

impl TestResult {
    
    pub fn new(is_success: bool) -> TestResult {

        TestResult {
            is_success,
            has_warning: false
        }
    }

    pub fn new_with_warning(is_success: bool) -> TestResult {

        TestResult {
            is_success,
            has_warning: true
        }
    }

}

pub async fn execute_test(test: &str) -> Result<TestResult, Error> {

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

                Ok(TestResult::new(is_success))
            }
            None => {
                let error_message = Error::new("Ping test failed", "The ping command expects a valid target"); 
                Err(error_message)
            }
        }
    }

    if test.starts_with("dns") {
        // TODO
        let error_message = Error::new("DNS test failed", "The 'dns' command is not supported yet"); 
        return Err(error_message);
    }

    if test.starts_with("http") {

        let result: Vec<String> = test.split(' ').map(|item| item.to_string()).collect();

        return match result.get(1) {
            Some(domain) => {

                let client = Client::new();
                let url = format!("http://{}", domain);
                let request_result = client.get(url)
                    .header("user-agent", "watchdog-relay")
                    .header("cache-control", "no-store")
                    .send()
                    .await;

                match request_result {
                    Ok(response) => {

                        let http_status = &response.status();
                        if http_status.is_client_error() || http_status.is_server_error() {
                            return Ok(TestResult::new_with_warning(true));
                        }

                        return Ok(TestResult::new(true));

                    },
                    Err(_) => Ok(TestResult::new(false))
                }
            },
            None => {
                let error_message = Error::new("HTTP test failed", "The HTTP command expects a target"); 
                Err(error_message)
            }
        };
    }

    let error_message = format!("Test '{}' failed, command not found", test);
    Err(Error::basic(error_message))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn should_request_http_domain() {
        
        assert_eq!(execute_test("http example.org").await, Ok(TestResult::new(true)));
    }

    #[tokio::test]
    async fn should_request_http_path() {
        
        assert_eq!(execute_test("http github.com").await, Ok(TestResult::new(true)));
    }

    #[tokio::test]
    async fn should_fail_http_invalid_domain() {
        
        assert_eq!(execute_test("http www.this-does-not-exist.be").await, Ok(TestResult::new(false)));
    }

    #[tokio::test]
    async fn should_fail_http_unknown_page() {
        
        assert_eq!(execute_test("http example.org/fail").await, Ok(TestResult::new(false)));
    }

    #[tokio::test]
    async fn should_perform_valid_ping() {
        
        assert_eq!(execute_test("ping 1.1.1.1").await, Ok(TestResult::new(true)));
    }

    #[tokio::test]
    async fn should_fail_invalid_ping() {
        
        assert_eq!(execute_test("ping 10.99.99.99").await, Ok(TestResult::new(false)));
    }

    #[tokio::test]
    async fn should_fail_unknown_test_type() {
        
        assert_eq!(execute_test("unknown").await, Err(Error::basic(
            "Test 'unknown' failed, command not found".to_string()
        )));
    }

    #[tokio::test]
    async fn should_fail_empty_test() {
        
        assert_eq!(execute_test("").await, Err(Error::basic(
            "Test '' failed, command not found".to_string()
        )));
    }

}
