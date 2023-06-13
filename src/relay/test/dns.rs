use std::str;

use crate::{common::error::Error, relay::model::TestResult};

pub struct DnsTest {}

impl DnsTest {

    pub fn new() -> Self {

        DnsTest {}
    }

    pub fn matches(&self, test: &str) -> bool {

        test.starts_with("dns")
    }

    pub async fn execute(&self, _test: &str) -> Result<TestResult, Error> {

        let error_message = Error::new("DNS test failed", "The 'dns' command is not supported yet"); 
        Err(error_message)
    }

}
