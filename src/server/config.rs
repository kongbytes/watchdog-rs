use std::fs;

use serde::{Deserialize, Serialize};
use serde_yaml;

use crate::error::ServerError;

#[derive(Deserialize,Serialize)]
pub struct GroupConfig {
    pub name: String,
    pub threshold: String,
    pub mediums: String,
    pub tests: Vec<String>
}

#[derive(Deserialize,Serialize)]
pub struct RegionConfig {
    pub name: String,
    pub interval: String,
    pub threshold: String,
    pub groups: Vec<GroupConfig>
}

#[derive(Deserialize,Serialize)]
pub struct Config {
    pub regions: Vec<RegionConfig>
}

impl Config {

    pub fn new(config_path: &str) -> Result<Config, ServerError> {

        let contents = fs::read_to_string(config_path).map_err(|err| ServerError {
            message: format!("Could not read configuration file - {}", err)
        })?;
        let parsed_yaml = serde_yaml::from_str(&contents).map_err(|err| ServerError {
            message: format!("Could not parse YAML - {}", err)
        })?;

        Ok(parsed_yaml)
    }

    pub fn export_region(&self, region_name: &str) -> Option<&RegionConfig> {

        self.regions.iter().find(|region| region.name.eq(region_name))
    }

}
