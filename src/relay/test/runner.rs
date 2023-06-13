use std::str;

use crate::{common::error::Error, relay::model::TestResult};

use super::{ping::PingTest, http::HttpTest, dns::DnsTest};

pub struct TestRunner {
    ping: PingTest,
    http: HttpTest,
    dns: DnsTest
}

impl TestRunner {

    pub fn new() -> Self {

        TestRunner {
            ping: PingTest::new(),
            http: HttpTest::new(),
            dns: DnsTest::new()
        }
    }

    pub async fn execute_test(&self, test: &str) -> Result<TestResult, Error> {

        if self.ping.matches(test) {
            return self.ping.execute(test).await;
        }
    
        if self.dns.matches(test) {
            return self.dns.execute(test).await;   
        }
    
        if self.http.matches(test)  {
            return self.http.execute(test).await;
        }
    
        let error_message = format!("Test '{}' failed, command not found", test);
        Err(Error::basic(error_message))
    }

}

#[cfg(test)]
mod tests {

    use crate::relay::model::ResultCategory;

    use super::*;

    #[tokio::test]
    async fn should_request_http_domain() {
        
        let runner = TestRunner::new();
        assert_eq!(runner.execute_test("http kongbytes.io").await, Ok(TestResult::success("kongbytes.io")));
    }

    #[tokio::test]
    async fn should_request_http_path() {
        
        let runner = TestRunner::new();
        assert_eq!(runner.execute_test("http github.com/kongbytes").await, Ok(TestResult::success("github.com/kongbytes")));
    }

    #[tokio::test]
    async fn should_fail_http_invalid_domain() {
        
        let runner = TestRunner::new();
        assert_eq!(runner.execute_test("http www.this-does-not-exist.be").await, Ok(TestResult::fail("www.this-does-not-exist.be")));
    }

    #[tokio::test]
    async fn should_fail_http_unknown_page() {
        
        let runner = TestRunner::new();
        assert_eq!(runner.execute_test("http kongbytes.io/unknown.html").await, Ok(TestResult::warning("kongbytes.io/unknown.html")));
    }

    #[tokio::test]
    async fn should_perform_valid_ping() {
        
        let runner = TestRunner::new();
        let test_result = runner.execute_test("ping 1.1.1.1").await;

        assert_eq!(test_result.is_ok(), true);
        let result = test_result.unwrap();

        assert_eq!(result.target, "1.1.1.1");
        assert_eq!(matches!(result.result, ResultCategory::Success), true);
        
        assert_eq!(result.metrics.is_some(), true);
        let metrics = result.metrics.unwrap();

        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics.get("ping_rtt").unwrap() > &0.00, true);
    }

    #[tokio::test]
    async fn should_fail_invalid_ping() {
        
        let runner = TestRunner::new();
        assert_eq!(runner.execute_test("ping 10.99.99.99").await, Ok(TestResult::fail("10.99.99.99")));
    }

    #[tokio::test]
    async fn should_fail_unknown_test_type() {
        
        let runner = TestRunner::new();
        assert_eq!(runner.execute_test("unknown").await, Err(Error::basic(
            "Test 'unknown' failed, command not found".to_string()
        )));
    }

    #[tokio::test]
    async fn should_fail_empty_test() {
        
        let runner = TestRunner::new();
        assert_eq!(runner.execute_test("").await, Err(Error::basic(
            "Test '' failed, command not found".to_string()
        )));
    }

}
