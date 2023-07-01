use reqwest::Client;

use crate::relay::model::GroupResultInput;
use crate::server::config::RegionConfig;
use crate::common::error::Error;

pub struct ServerApi {

    client: Client,
    authorization_header: String,
    config_route: String,
    update_route: String

}

impl ServerApi {

    pub fn new(base_url: &str, token: &str, region_name: &str) -> ServerApi {

        let client = Client::new();
        let authorization_header = format!("Bearer {}", token);
        
        let config_route = format!("{}/api/v1/relay/{}", base_url, region_name);
        let update_route = format!("{}/api/v1/relay/{}", base_url, region_name);

        ServerApi {
            client,
            authorization_header,
            config_route,
            update_route
        }
    }

    pub async fn fetch_region_conf(&self) -> Result<RegionConfig, Error> {

        let http_response = self.client.get(&self.config_route)
            .header("Content-Type", "application/json")
            .header("Authorization", &self.authorization_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|err| Error::new("Could not fetch configuration from server", err))?;

        if http_response.status() != 200 {
            return Err(
                Error::basic(format!("Expected status code 200, found {}", http_response.status()))
            );
        }

        let body = http_response.text()
            .await
            .map_err(|err| Error::new("Could not decode configuration from server", err))?;
        
        serde_json::from_str::<RegionConfig>(&body).map_err(|err| Error::new("Failed to decode JSON region config", err))
    }

    pub async fn update_region_state(&self, group_results: &Vec<GroupResultInput>, last_update: &str) -> Result<Option<String>, Error> {

        let json_state = serde_json::to_string(&group_results)
            .map_err(|err| Error::new("Could not parse region state to JSON", err))?;

        let response = self.client.put(&self.update_route)
            .header("Content-Type", "application/json")
            .header("Authorization", &self.authorization_header)
            .header("Accept", "application/json")
            .body(json_state)
            .send()
            .await
            .map_err(|err| Error::new("Could not update region state", err))?;

        if let Some(header_value) = response.headers().get("X-Watchdog-Update") {

            let watchdog_update = header_value.to_str().unwrap_or("unknown");

            if watchdog_update != last_update {
                return Ok(Some(watchdog_update.to_string()))
            }
        }

        Ok(None)
    }

    pub async fn trigger_kuma_update(&self, kuma_url: &str, total_groups: usize, unstable_groups: usize, last_ping: Option<f32>) -> Result<(), Error> {

        let message = if total_groups == unstable_groups {
            format!("OK {} healthy", total_groups)
        } else {
            format!("WARN {} unstable", unstable_groups)
        };

        let mut kuma_full_url = format!("{}?status=up&msg={}", kuma_url, message);

        if let Some(ping) = last_ping {
            let ping_url = format!("&ping={}", ping);
            kuma_full_url.push_str(&ping_url);
        }

        let http_response = self.client.get(kuma_full_url)
            .send()
            .await
            .map_err(|err| Error::new("Could not fetch configuration from server", err))?;

        if http_response.status() != 200 {
            return Err(
                Error::basic(format!("Expected 200 for Kuma update, found {}", http_response.status()))
            );
        }

        Ok(())
    }

}
