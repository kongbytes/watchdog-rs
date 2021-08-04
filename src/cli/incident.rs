use crate::common::error::Error;
use crate::server::storage::IncidentItem;
use super::utils::api_get;

pub async fn list_incidents(base_url: &str, token: &str) -> Result<(), Error> {

    let raw_incidents: Vec<IncidentItem> = api_get(base_url, token, "api/v1/incidents").await?;

    let incidents: Vec<IncidentItem> = raw_incidents.into_iter().map(|mut incident| {

        let timestamp = chrono::DateTime::parse_from_rfc3339(&incident.timestamp).unwrap();
        let formatted_time = timestamp.format("%Y-%m-%d %H:%M:%S");

        incident.timestamp = formatted_time.to_string();

        incident

    }).collect();

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

pub async fn inspect_incident(base_url: &str, token: &str, incident_id: &str) -> Result<(), Error> {
    
    let incident_api = format!("api/v1/incidents/{}", incident_id);
    let mut incident: IncidentItem = api_get(base_url, token, &incident_api).await?;
    
    let timestamp = chrono::DateTime::parse_from_rfc3339(&incident.timestamp).unwrap();
    let formatted_time = timestamp.format("%Y-%m-%d %H:%M:%S");

    incident.timestamp = formatted_time.to_string();

    println!();
    println!("Incident ID\t{}", incident.id);
    println!("Name\t\t{}", incident.message);
    println!("Timestamp\t{}", incident.timestamp);
    println!();

    Ok(())
}
