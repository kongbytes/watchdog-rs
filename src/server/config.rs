use std::fs;
use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use crate::common::error::Error;

// The 'input' models below will only be used once to parse the YAML
// configuration file. This data is rather human-friendly and will not be used
// accross watchdog services (see below).

#[derive(Deserialize)]
pub struct GroupConfigInput {
    pub name: String,
    pub threshold: u64,
    pub mediums: String,
    pub tests: Vec<String>
}

#[derive(Deserialize)]
pub struct RegionConfigInput {
    pub name: String,
    pub interval: String,
    pub threshold: u64,
    pub groups: Vec<GroupConfigInput>
}

#[derive(Deserialize)]
pub struct ConfigInput {
    pub regions: Vec<RegionConfigInput>
}

// Internal models

#[derive(Deserialize,Serialize)]
pub struct GroupConfig {
    pub name: String,
    pub threshold_ms: u64,
    pub mediums: Vec<String>,
    pub tests: Vec<String>
}

#[derive(Deserialize,Serialize)]
pub struct RegionConfig {
    pub name: String,
    pub interval_ms: u64,
    pub threshold_ms: u64,
    pub groups: Vec<GroupConfig>
}

#[derive(Deserialize,Serialize)]
pub struct Config {
    pub regions: Vec<RegionConfig>
}

impl Config {

    pub fn new(config_path: &str) -> Result<Config, Error> {

        let contents = fs::read_to_string(config_path).map_err(|err| Error::new("Could not read configuration file", err))?;
        let parsed_yaml: ConfigInput = serde_yaml::from_str(&contents).map_err(|err| Error::new("Could not parse YAML", err))?;

        Config::try_from(parsed_yaml).map_err(|err| Error::new("Failed to parse config", err))
    }

    pub fn export_region(&self, region_name: &str) -> Option<&RegionConfig> {

        self.regions.iter().find(|region| region.name.eq(region_name))
    }

    pub fn has_medium(&self, medium_key: &str) -> bool {

        for region in self.regions.iter() {
            for group in region.groups.iter() {

                if group.mediums.iter().any(|medium| medium == medium_key) {
                    return true;
                }
            }
        }

        false
    }

}

impl TryFrom<ConfigInput> for Config{

    type Error = &'static str;

    fn try_from(input: ConfigInput) -> Result<Self, Self::Error> {

        let mut regions: Vec<RegionConfig> = vec![];
        for region_input in input.regions.iter() {

            let region_interval_ms = parse_to_milliseconds(&region_input.interval)?;

            let mut groups: Vec<GroupConfig> = vec![];
            for group_input in region_input.groups.iter() {

                let group = GroupConfig {
                    name: String::from(&group_input.name),
                    threshold_ms: region_interval_ms * group_input.threshold + 1000,
                    mediums: group_input.mediums.split(',')
                        .map(String::from)
                        .collect::<Vec<String>>(),
                    tests: group_input.tests.clone()
                };
                groups.push(group);
            }

            let region = RegionConfig {
                name: String::from(&region_input.name),
                interval_ms: region_interval_ms,
                // We add 1000 to let the network the network request be processed
                // after the interval multiple
                threshold_ms: region_interval_ms * region_input.threshold + 1000,
                groups
            };
            regions.push(region);
        }

        Ok(Config {
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
