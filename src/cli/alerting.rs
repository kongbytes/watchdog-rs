use serde::Deserialize;

use crate::common::error::Error;
use super::utils::api_post;

#[derive(Deserialize)]
struct AlertTestResponse {
    alerts_sent: bool,
    error: Option<String>
}

pub async fn test_alerting(base_url: &str, token: &str) -> Result<(), Error> {
    
    let test_response: AlertTestResponse = api_post(base_url, token, "api/v1/alerting/test").await?;

    if test_response.alerts_sent {
        println!("Test alerts sent to all mediums");
    }
    else {
        let error_message = test_response.error.unwrap_or("no details".into());
        println!("Error while sending test alerts to all mediums ({})", error_message);
    }

    Ok(())
}
