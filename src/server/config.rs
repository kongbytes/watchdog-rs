use tokio::fs;
use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use crate::common::error::Error;

pub struct ServerConf {

    pub config_path: String,
    pub port: u16,
    pub address: String,
    pub token: String,

    pub telegram_token: Option<String>,
    pub telegram_chat: Option<String>

}

// The 'input' models below will only be used once to parse the YAML
// configuration file. This data is rather human-friendly and will not be used
// accross watchdog services, except for the init CLI (see below).

#[derive(Deserialize, Serialize)]
pub struct AlerterConfigInput {
    pub name: String,
    pub medium: String,
    pub chat_env: Option<String>,
    pub token_env: Option<String>,
    pub recipients_env: Option<String>
}

#[derive(Deserialize, Serialize)]
pub struct GroupConfigInput {
    pub name: String,
    pub fail_threshold: Option<u64>,
    pub tests: Vec<String>
}

#[derive(Deserialize, Serialize)]
pub struct RegionConfigInput {
    pub name: String,
    pub send_interval: Option<String>,
    pub miss_threshold: Option<u64>,
    pub kuma_url: Option<String>,
    pub groups: Vec<GroupConfigInput>
}

#[derive(Deserialize, Serialize)]
pub struct ConfigInput {
    pub alerters: Option<Vec<AlerterConfigInput>>,
    pub regions: Vec<RegionConfigInput>
}

// Internal models

#[derive(Deserialize,Serialize,Clone)]
pub struct GroupConfig {
    pub name: String,
    pub threshold_ms: u64,
    pub tests: Vec<String>
}

#[derive(Deserialize,Serialize,Clone)]
pub struct RegionConfig {
    pub name: String,
    pub interval_ms: u64,
    pub threshold_ms: u64,
    pub kuma_url: Option<String>,
    pub groups: Vec<GroupConfig>
}

#[derive(Deserialize,Serialize)]
pub struct AlertConfig {
    pub name: String,
    pub medium: String,
    pub chat_env: Option<String>,
    pub token_env: Option<String>,
    pub recipients_env: Option<String>
}

#[derive(Deserialize,Serialize)]
pub struct Config {
    pub version: String,
    pub alerters: Vec<AlertConfig>,
    pub regions: Vec<RegionConfig>
}

impl Config {

    pub async fn new(config_path: &str) -> Result<Config, Error> {

        let contents = fs::read_to_string(config_path).await.map_err(|err| Error::new("Could not read configuration file", err))?;
        let parsed_yaml: ConfigInput = serde_yaml::from_str(&contents).map_err(|err| Error::new("Could not parse YAML", err))?;

        Config::try_from(parsed_yaml).map_err(|err| Error::new("Failed to parse config", err))
    }

    pub fn export_region(&self, region_name: &str) -> Option<&RegionConfig> {

        self.regions.iter().find(|region| region.name.eq(region_name))
    }

}

impl TryFrom<ConfigInput> for Config{

    type Error = &'static str;

    fn try_from(input: ConfigInput) -> Result<Self, Self::Error> {

        let mut regions: Vec<RegionConfig> = vec![];
        for region_input in input.regions.iter() {

            let human_readable_interval = match &region_input.send_interval {
                Some(send_interval) => send_interval,
                None => "10s"
            };
            let region_interval_ms = parse_to_milliseconds(human_readable_interval)?;

            let mut groups: Vec<GroupConfig> = vec![];
            for group_input in region_input.groups.iter() {

                let group_fail_threshold = group_input.fail_threshold.unwrap_or(3);
                let group = GroupConfig {
                    name: String::from(&group_input.name),
                    threshold_ms: region_interval_ms * group_fail_threshold + 1000,
                    tests: group_input.tests.clone()
                };
                groups.push(group);
            }

            let region_miss_threshold = region_input.miss_threshold.unwrap_or(3);
            let region = RegionConfig {
                name: String::from(&region_input.name),
                interval_ms: region_interval_ms,
                // We add 1000 to let the network the network request be processed
                // after the interval multiple
                threshold_ms: region_interval_ms * region_miss_threshold + 1000,
                kuma_url: region_input.kuma_url.clone(),
                groups
            };
            regions.push(region);
        }

        let alerters = match input.alerters {
            Some(alerters) => {
                alerters.into_iter().map(|alerter_input| {

                    AlertConfig {
                        name: alerter_input.name,
                        medium: alerter_input.medium,
                        chat_env: alerter_input.chat_env,
                        token_env: alerter_input.token_env,
                        recipients_env: alerter_input.recipients_env
                    }
        
                }).collect()
            },
            None => vec![]
        };

        Ok(Config {
            // TODO Better format
            version: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            alerters,
            regions
        })
    }

}

/**
 * Parse a given time string into milliseconds. This can be used to convert a
 * string such as '20ms', '10s' or '1h' into adequate milliseconds. Without
 * suffix, the default behavior is to parse into milliseconds.
 */
pub fn parse_to_milliseconds(time_arg: &str) -> Result<u64, &'static str> {

    let len = time_arg.len();

    if time_arg.ends_with("ms") {
        let milliseconds_text = &time_arg[0..len-2];
        return match milliseconds_text.parse::<u64>() {
            Ok(ms_value) => Ok(ms_value),
            Err(_) => Err("invalid milliseconds")
        };
    }

    if time_arg.ends_with('s') {
        let seconds_text = &time_arg[0..len-1];
        return match seconds_text.parse::<u64>().map(|value| value * 1000) {
            Ok(ms_value) => Ok(ms_value),
            Err(_) => Err("invalid seconds")
        };
    }

    if time_arg.ends_with('m') {
        let seconds_text = &time_arg[0..len-1];
        return match seconds_text.parse::<u64>().map(|value| value * 1000 * 60) {
            Ok(ms_value) => Ok(ms_value),
            Err(_) => Err("invalid minutes")
        };
    }

    if time_arg.ends_with('h') {
        let hour_text = &time_arg[0..len-1];
        return match hour_text.parse::<u64>().map(|value| value * 1000 * 60 * 60) {
            Ok(ms_value) => Ok(ms_value),
            Err(_) => Err("invalid hours")
        };
    }

    match time_arg.parse::<u64>() {
        Ok(ms_value) => Ok(ms_value),
        Err(_) => Err("invalid milliseconds")
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn should_parse_milliseconds() {
        
        assert_eq!(parse_to_milliseconds("1000"), Ok(1000));
    }

    #[test]
    fn should_parse_seconds() {
        
        assert_eq!(parse_to_milliseconds("5s"), Ok(5000));
    }

    #[test]
    fn should_parse_minutes() {
        
        assert_eq!(parse_to_milliseconds("3m"), Ok(180_000));
    }

    #[test]
    fn should_parse_hours() {
        
        assert_eq!(parse_to_milliseconds("2h"), Ok(7_200_000));
    }

    #[test]
    fn should_deny_negative() {
        
        assert_eq!(parse_to_milliseconds("-45"), Err("invalid milliseconds"));
    }

    #[test]
    fn should_deny_floating_numbers() {
        
        assert_eq!(parse_to_milliseconds("3.235"), Err("invalid milliseconds"));
    }

    #[test]
    fn should_deny_invalid_characters() {
        
        assert_eq!(parse_to_milliseconds("3z"), Err("invalid milliseconds"));
    }

}
