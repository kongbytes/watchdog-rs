use crate::common::error::Error;
use crate::server::storage::IncidentItem;
use super::utils::{api_get, format_timestamp};

fn get_error_message(error_message: &Option<String>) -> String {

    match error_message {
        Some(message) => message.to_string(),
        None => "no_message".to_string()
    }
}

pub async fn list_incidents(base_url: &str, token: &str) -> Result<(), Error> {

    let raw_incidents: Vec<IncidentItem> = api_get(base_url, token, "api/v1/incidents").await?;

    let incidents: Vec<IncidentItem> = raw_incidents.into_iter().map(|mut incident| {
        incident.timestamp = format_timestamp(&incident.timestamp);
        incident
    }).collect();

    let mut timestamp_length = 15;
    let mut message_length = 15;
    let mut error_length = 15;
    for incident in incidents.iter() {

        if incident.timestamp.len() > timestamp_length {
            timestamp_length = incident.timestamp.len();
        }

        if incident.message.len() > message_length {
            message_length = incident.message.len();
        }

        if let Some(error_message) = incident.error_message.as_ref() {
            if error_message.len() > error_length {
                error_length = error_message.len();
            }
        }
    }

    println!();
    println!("| ID   | {: <h_max$} | {: <v_max$} | {: <e_max$} |", "Timestamp", "Message", "Details", h_max=timestamp_length, v_max=message_length, e_max=error_length);
    println!("|------|-{:-<h_max$}-|-{:-<v_max$}-|-{:-<e_max$}-|", "", "", "", h_max=timestamp_length, v_max=message_length, e_max=error_length);

    for incident in incidents.iter() {
    
        let error_message = get_error_message(&incident.error_message);
        println!("| {: <4} | {: <h_max$} | {: <v_max$} | {: <e_max$} |", incident.id, incident.timestamp, incident.message, error_message, h_max=timestamp_length, v_max=message_length, e_max=error_length);
    }
    println!();

    Ok(())
}

pub async fn inspect_incident(base_url: &str, token: &str, incident_id: &str) -> Result<(), Error> {
    
    let incident_api = format!("api/v1/incidents/{}", incident_id);
    let mut incident: IncidentItem = api_get(base_url, token, &incident_api).await?;
    
    incident.timestamp = format_timestamp(&incident.timestamp);

    println!();
    println!("Incident ID\t{}", incident.id);
    println!("Timestamp\t{}", incident.timestamp);
    println!("Message\t\t{}", incident.message);
    println!("Details\t\t{}", get_error_message(&incident.error_message));
    println!();

    Ok(())
}
