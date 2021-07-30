use reqwest::Client;

use crate::common::error::Error;
use crate::server::storage::IncidentItem;

pub async fn list_incidents(base_url: &str, token: &str) -> Result<(), Error> {

    let list_api = format!("{}/api/v1/incidents", base_url);
    let authorization_header = format!("Bearer {}", token);

    let http_client = Client::new();
    let http_response = http_client.get(&list_api)
        .header("Content-Type", "application/json")
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
    
    let incidents = serde_json::from_str::<Vec<IncidentItem>>(&body).map_err(|err| Error::new("Failed to decode JSON response", err))?;

    let mut timestamp_length = 15;
    let mut message_length = 15;
    for incident in incidents.iter() {

        if incident.timestamp.len() > timestamp_length {
            timestamp_length = incident.timestamp.len();
        }

        if incident.message.len() > message_length {
            message_length = incident.message.len();
        }
    }

    println!();
    println!("| ID   | {: <h_max$} | {: <v_max$} |", "Timestamp", "Message", h_max=timestamp_length, v_max=message_length);
    println!("|------|-{:-<h_max$}-|-{:-<v_max$}-|", "", "", h_max=timestamp_length, v_max=message_length);

    for incident in incidents.iter() {
        println!("| {: <4} | {: <h_max$} | {: <v_max$} |", incident.id, incident.timestamp, incident.message, h_max=timestamp_length, v_max=message_length);
    }

    println!();

    Ok(())
}

pub async fn inspect_incident(incident_id: &str) -> Result<(), Error> {
    println!("Inspect");
    dbg!(incident_id);
    Ok(())
}
