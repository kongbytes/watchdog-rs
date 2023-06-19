use reqwest::Client;
use serde::de::DeserializeOwned;
use chrono::DateTime;

use crate::common::error::Error;

pub async fn api_get<T>(base_url: &str, token: &str, route: &str) -> Result<T, Error> where T: DeserializeOwned {

    let get_api = format!("{}/{}", base_url, route);
    let authorization_header = format!("Bearer {}", token);

    let http_client = Client::new();
    let http_response = http_client.get(&get_api)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", &authorization_header)
        .send()
        .await
        .map_err(|err| Error::new("An unknown network error triggered", err))?;

    let http_status = &http_response.status();
    if http_status.is_client_error() || http_status.is_server_error() {
        let status_err = Error::basic(format!("Expected HTTP response code OK, but received {}", http_status));
        return Err(status_err);
    }

    let body = http_response.text()
        .await
        .map_err(|err| Error::new("Could not decode response from server", err))?;
    
    let json_response = serde_json::from_str::<T>(&body).map_err(|err| Error::new("Failed to decode JSON response", err))?;
    
    Ok(json_response)
}

pub async fn api_post<T>(base_url: &str, token: &str, route: &str) -> Result<T, Error> where T: DeserializeOwned {

    let post_api = format!("{}/{}", base_url, route);
    let authorization_header = format!("Bearer {}", token);

    let http_client = Client::new();
    let http_response = http_client.post(&post_api)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", &authorization_header)
        .send()
        .await
        .map_err(|err| Error::new("An unknown network error triggered", err))?;

    let http_status = &http_response.status();
    if http_status.is_client_error() || http_status.is_server_error() {
        let status_err = Error::basic(format!("Expected HTTP response code OK, but received {}", http_status));
        return Err(status_err);
    }

    let body = http_response.text()
        .await
        .map_err(|err| Error::new("Could not decode response from server", err))?;
    
    let json_response = serde_json::from_str::<T>(&body).map_err(|err| Error::new("Failed to decode JSON response", err))?;
    
    Ok(json_response)
}

pub fn format_timestamp(timestamp: &str) -> String {

    match DateTime::parse_from_rfc3339(timestamp) {
        Ok(datetime) => datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
        Err(_) => "invalid timestamp".to_string()
    }
}
