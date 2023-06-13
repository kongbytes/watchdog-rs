use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::Body,
    error_handling::HandleErrorLayer,
    extract::{Path, State},
    http::{HeaderMap, header, Request, StatusCode},
    Json,
    middleware::{Next, from_fn, from_fn_with_state},
    response::IntoResponse,
    Router,
    routing::get
};
use serde_json::json;
use tokio::{signal, task, sync::RwLock};
use tokio_util::sync::CancellationToken;
use tower::{BoxError, ServiceBuilder};

use crate::common::error::Error;
use crate::relay::model::GroupResultInput;
use crate::server::config::Config;
use crate::server::storage::{MemoryStorage, Storage, GroupState, RegionState};
use crate::server::scheduler::launch_scheduler;

use super::config::RegionConfig;
use super::utils::ServerErr;
use super::storage::{RegionSummary, IncidentItem, GroupMetrics};

pub const DEFAULT_PORT: u16 = 3030; 
pub const DEFAULT_ADDRESS: &str = "127.0.0.1"; 

pub struct ServerConf {

    pub config_path: String,
    pub port: u16,
    pub address: String,
    pub token: String,

    pub telegram_token: Option<String>,
    pub telegram_chat: Option<String>

}

struct AppState {
    storage: Storage,
    config: Arc<Config>
}

pub async fn launch(server_conf: ServerConf) -> Result<(), Error> {

    let storage = MemoryStorage::new();

    let base_config = Config::new(&server_conf.config_path)?;

    if base_config.has_medium("telegram") && (server_conf.telegram_chat.is_none() || server_conf.telegram_token.is_none()) {
        let error_message = "Current configuration is using telegram medium, but missing environment variables (TELEGRAM_CHAT/TELEGRAM_TOKEN)".to_string();
        return Err(Error::basic(error_message));
    }

    let config = Arc::new(base_config);

    let app_state = Arc::new(AppState {
        storage: storage.clone(),
        config: config.clone(),
    });

    let shared_server_conf = Arc::new(server_conf);

    init_storage_regions(storage.clone(), config.clone()).await;

    let middleware = ServiceBuilder::new()
        // 3. Apply the HandleError service adapter. Since we use Tower utility layers
        // (aka middleware), an error service must be defined below to transform specific
        // errors from the middlewares into HTTP responses.
        .layer(HandleErrorLayer::new(|error: BoxError| async move {
            
            if error.is::<tower::timeout::error::Elapsed>() {
                eprintln!("Request timed-out: {}", error);
                Ok((
                    StatusCode::REQUEST_TIMEOUT,
                    "Request timed-out"
                ))
            }
            else {
                eprintln!("Found unhandled error from the middleware layers: {}", error);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Unhandled internal error"
                ))
            }
        }))
        // 2. Fail requests that take longer than 10 seconds (if the next layer takes more
        // to respond - processing is terminated and an error is returned).
        .timeout(Duration::from_secs(10))
        // 1. Perfom a basic log on the requests with the response code
        .layer(from_fn(log_request))
        .into_inner();

    let app = Router::new()
        .route(
            "/api/v1/relay/:region_name",
            get(handle_get_config)
            .put(handle_region_update)
        )
        .route(
            "/api/v1/analytics",
            get(handle_analytics)
        )
        .route(
            "/api/v1/incidents",
            get(handle_find_incidents)
        )
        .route(
            "/api/v1/incidents/:incident_id",
            get(handle_get_incident)
        )
        .route(
            "/api/v1/exporter",
            get(handle_prometheus_metrics)
        )
        .fallback(handle_not_found)
        .route_layer(from_fn_with_state(shared_server_conf.clone(), check_authorization))
        .layer(middleware)
        .with_state(app_state);

    let cancel_token = CancellationToken::new();
    let cancel_token_http = cancel_token.clone();
    let cancel_token_scheduler = cancel_token.clone();

    let api_url = format!("{}:{}", shared_server_conf.address, shared_server_conf.port);
    println!("Starting HTTP server on {}", api_url);

    let server = axum::Server::bind(&api_url.parse().unwrap())
        .serve(app.into_make_service())
        .with_graceful_shutdown(async move {
            cancel_token_http.cancelled().await;
        });

    let web_handle = task::spawn(server);

    println!();
    println!(" ✓ Watchdog monitoring API is UP (port {})", shared_server_conf.port);

    let scheduler_conf = config.clone();
    let scheduler_storage = storage.clone();
    let scheduler_handle = task::spawn(async move {
        
        println!(" ✓ Watchdog network scheduler is UP");
        println!();
        println!("You can now start region network relays");
        println!("Use the 'relay --region name' command");
        println!();
    
        launch_scheduler(cancel_token_scheduler, scheduler_conf, scheduler_storage, &shared_server_conf.clone()).await;

    });

    signal::ctrl_c().await.map_err(|err| Error::new("Could not handle graceful shutdown signal", err))?;
    cancel_token.cancel();
    println!("Received graceful shutdown signal");

    let _= web_handle.await.map_err(|err| Error::new("Could not end web task", err))?;
    scheduler_handle.await.map_err(|err| Error::new("Could not end scheduler task", err))?;

    Ok(())
}

async fn check_authorization(State(state): State<Arc<ServerConf>>, request: Request<Body>, next: Next<Body>) -> Result<impl IntoResponse, impl IntoResponse> {

    let authorization_header = request.headers().get("authorization").map(|header| header.to_str().unwrap_or_default());

    match authorization_header {
        Some(token) => {

            if token != format!("Bearer {}", state.token) {
                return Err(ServerErr::unauthorized("Invalid authentication"));
            }
            
            let response = next.run(request).await;
            Ok(response)

        }
        None => Err(ServerErr::unauthorized("Invalid authentication"))
    }
}

async fn log_request(req: Request<Body>, next: Next<Body>) -> Result<impl IntoResponse, (StatusCode, String)> {

    let uri = req.uri().clone();
    let method = req.method().clone();

    let response = next.run(req).await;

    let status = response.status();
    if status.is_success() || status.is_redirection() || status.is_informational() {
        println!("\"{} {}\" {}", method, uri, response.status().as_u16());
    } else {
        eprintln!("\"{} {}\" {}", method, uri, response.status().as_u16());
    }

    Ok(response)
}

async fn init_storage_regions(storage: Arc<RwLock<MemoryStorage>>, config: Arc<Config>) {

    let mut write_lock = storage.write().await;
            
    for region in config.regions.iter() {
        
        let mut linked_groups: Vec<String> = vec![];
        for group in region.groups.iter() {
            write_lock.init_group(&region.name, &group.name);
            linked_groups.push(group.name.to_string())
        }

        write_lock.init_region(&region.name, linked_groups);
    }
}

async fn handle_not_found() -> impl IntoResponse {
    ServerErr::not_found("Endpoint not found")
}

async fn handle_get_config(Path(region_name): Path<String>, State(state): State<Arc<AppState>>) -> Result<Json<RegionConfig>, ServerErr> {

    let config = state.config.clone();

    let exported_config = config.export_region(&region_name).cloned();

    if let Some(config) = exported_config {
        return Ok(Json(config));
    }

    let error_message = format!("Relay configuration not found for region {}", region_name);
    Err(ServerErr::not_found(error_message))
}

async fn handle_analytics(State(state): State<Arc<AppState>>) -> Result<Json<RegionSummary>, ServerErr> {

    let storage = state.storage.clone();

    let regions = storage.read().await.compute_analytics();

    Ok(regions.into())
}

async fn handle_prometheus_metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {

    // TODO Should include group states as metrics

    let storage = state.storage.clone();

    let metrics = storage.read().await.find_metrics();

    metrics.iter().map(|metric| {
    
        let labels: Vec<String> = metric.labels.iter().map(|(key, value)| format!("{}=\"{}\"", key, value)).collect();
        format!("watchdog_{}{{{}}} {}\n", metric.name, labels.join(","), metric.metric)
    
    }).collect::<String>()
}

// TODO Should validate body
async fn handle_region_update(Path(region_name): Path<String>, State(state): State<Arc<AppState>>, Json(results): Json<Vec<GroupResultInput>>) -> impl IntoResponse {

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


async fn handle_find_incidents(State(state): State<Arc<AppState>>) -> Result<Json<Vec<IncidentItem>>, ServerErr> {

    let storage = state.storage.clone();

    let incidents = storage.read().await.find_incidents();

    Ok(incidents.into())
}

async fn handle_get_incident(Path(incident_id): Path<u32>, State(state): State<Arc<AppState>>) -> Result<Json<IncidentItem>, ServerErr> {

    let storage = state.storage.clone();

    let incident_result = storage.read().await.get_incident(incident_id);

    if let Some(result) = incident_result {
        return Ok(result.into())
    }

    Err(ServerErr::not_found("Could not find incident"))
}
