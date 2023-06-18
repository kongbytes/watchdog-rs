use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, header, StatusCode},
    Json,
    response::IntoResponse,
};
use serde_json::json;

use crate::relay::model::GroupResultInput;
use crate::server::storage::{GroupState, RegionState};

use super::{config::RegionConfig, service::AppState};
use super::utils::ServerErr;
use super::storage::{RegionSummary, IncidentItem, GroupMetrics};

pub async fn handle_not_found() -> impl IntoResponse {
    ServerErr::not_found("Endpoint not found")
}

pub async fn handle_get_config(Path(region_name): Path<String>, State(state): State<Arc<AppState>>) -> Result<Json<RegionConfig>, ServerErr> {

    let config = state.config.clone();

    let exported_config = config.export_region(&region_name).cloned();

    if let Some(config) = exported_config {
        return Ok(Json(config));
    }

    let error_message = format!("Relay configuration not found for region {}", region_name);
    Err(ServerErr::not_found(error_message))
}

pub async fn handle_analytics(State(state): State<Arc<AppState>>) -> Result<Json<RegionSummary>, ServerErr> {

    let storage = state.storage.clone();

    let regions = storage.read().await.compute_analytics();

    Ok(regions.into())
}

pub async fn handle_prometheus_metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {

    // TODO Should include group states as metrics

    let storage = state.storage.clone();

    let test_metrics = storage.read().await.collect_test_metrics();
    let region_metrics = storage.read().await.collect_region_metrics();

    let formatted_tests = test_metrics.iter().map(|metric| {
    
        let labels: Vec<String> = metric.labels.iter().map(|(key, value)| format!("{}=\"{}\"", key, value)).collect();
        format!("watchdog_{}{{{}}} {}\n", metric.name, labels.join(","), metric.metric)
    
    }).collect::<String>();

    let formatted_regions = region_metrics.iter().map(|metric| {
    
        let labels: Vec<String> = metric.labels.iter().map(|(key, value)| format!("{}=\"{}\"", key, value)).collect();
        format!("watchdog_{}{{{}}} {}\n", metric.name, labels.join(","), metric.metric)
    
    }).collect::<String>();

    format!("{}\n{}\n", formatted_regions, formatted_tests)
}

// TODO Should validate body
pub async fn handle_region_update(Path(region_name): Path<String>, State(state): State<Arc<AppState>>, Json(results): Json<Vec<GroupResultInput>>) -> impl IntoResponse {

    let storage = state.storage.clone();
    let config = state.config.clone();

    // TODO Blocking RW too long
    {
        let mut write_lock = storage.write().await;

        let mut has_warning = false;
        for group in results {

            // If groups in a region do not work (failed ping test) or have warnings (ping
            // latency too high) - we set the "warning" status to a region
            if !group.working || group.has_warnings {
                has_warning = true;
            }

            let group_state = match (group.working, group.has_warnings) {
                (true, false) => GroupState::Up,
                (true, true) => GroupState::Warn,
                (false, _) => GroupState::Down
            };

            let current_state = write_lock.get_group_status(&region_name, &group.name).map(|state| state.status.clone());
        
            // If there is an ongoing incident on the group and the group is -still- not working,
            // do not refresh values (can re-trigger incidents otherwise)
            // @TODO https://github.com/orgs/kongbytes/projects/3/views/1?pane=issue&itemId=30528369
            if !group.working && matches!(current_state, Some(GroupState::Incident)) {
                continue;
            }

            let mut metrics: Vec<GroupMetrics> = vec![];
            for group_metric in group.metrics {

                metrics.push(GroupMetrics {
                    name: group_metric.name,
                    labels: group_metric.labels,
                    metric: group_metric.metric
                });
            }

            write_lock.refresh_group(&region_name, &group.name, group_state, metrics).unwrap_or_else(|err| {
                eprintln!("Could not refresh group, can cause unstable storage: {}", err);
            });
        }

        let region_status = write_lock.get_region_status(&region_name);

        if let Some(status) = region_status {

            // We already had an incident
            if let RegionState::Down = status.status {
                println!("INCIDENT RESOLVED ON REGION {}", region_name);
            }
        }

        write_lock.refresh_region(&region_name, has_warning);
    }

    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
    headers.insert(header::CONNECTION, "close".parse().unwrap());
    headers.insert("X-Watchdog-Update", config.version.clone().parse().unwrap());

    (
        StatusCode::OK,
        headers,
        Json(json!({
            "result": true
        })),
    )

}

pub async fn handle_find_incidents(State(state): State<Arc<AppState>>) -> Result<Json<Vec<IncidentItem>>, ServerErr> {

    let storage = state.storage.clone();

    let incidents = storage.read().await.find_incidents();

    Ok(incidents.into())
}

pub async fn handle_get_incident(Path(incident_id): Path<u32>, State(state): State<Arc<AppState>>) -> Result<Json<IncidentItem>, ServerErr> {

    let storage = state.storage.clone();

    let incident_result = storage.read().await.get_incident(incident_id);

    if let Some(result) = incident_result {
        return Ok(result.into())
    }

    Err(ServerErr::not_found("Could not find incident"))
}
