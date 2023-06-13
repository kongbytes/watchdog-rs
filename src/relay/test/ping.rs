use std::{str, collections::HashMap};

use tokio::process::Command;

use crate::{common::error::Error, relay::model::{TestResult, ResultCategory}};

pub struct PingTest {}

impl PingTest {

    pub fn new() -> Self {

        PingTest {}
    }

    pub fn matches(&self, test: &str) -> bool {

        test.starts_with("ping")
    }

    pub async fn execute(&self, test: &str) -> Result<TestResult, Error> {

        let ping_components: Vec<&str> = test.split(' ').collect();

        let target = ping_components.get(1)
            .cloned()
            .ok_or(Error::new("Ping test failed", "The ping command expects a valid target"))?;

        let command_output = Command::new("/usr/bin/ping")
            .arg("-c")
            .arg("1")
            .arg("-w")
            .arg("2")
            .arg(target)
            .output()
            .await;

        let output = match command_output {
            Ok(output) => output,
            Err(err) => {
                return Err(Error::new("Failed to ping", err));
            }
        };
        
        if !output.status.success() {
            return Ok(TestResult::fail(target));
        }

        let stdout = match String::from_utf8(output.stdout) {
            Ok(stdout) => stdout,
            Err(err) => {
                return Err(Error::new("Failed to ping", err));
            }
        };

        let rtt_result = stdout.lines()
            .find(|s| s.starts_with("rtt"))
            .unwrap_or_default()
            .split(" = ")
            .collect::<Vec<&str>>()
            .get(1)
            .map(|s| s.split('/').next())
            .unwrap_or_default()
            .unwrap_or_default()
            .parse::<f32>();

        match rtt_result {
            Ok(rtt) => {

                let mut metrics: HashMap<String, f32> = HashMap::new();
                metrics.insert("ping_rtt".into(), rtt);

                let category = if rtt >= 100.0 { ResultCategory::Warning } else { ResultCategory::Success };

                Ok(TestResult::build(target, category, Some(metrics)))

            },
            Err(err) => Err(Error::new("Failed to ping", err)),
        }
    }

}
