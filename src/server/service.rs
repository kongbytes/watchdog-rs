use std::sync::Arc;
use std::time::Duration;

use axum::{
    error_handling::HandleErrorLayer,
    http::StatusCode,
    middleware::{from_fn, from_fn_with_state},
    Router,
    routing::{get, post}
};
use tokio::{signal, task, sync::RwLock};
use tokio_util::sync::CancellationToken;
use tower::{BoxError, ServiceBuilder};

use crate::{common::error::Error, server::{middleware::{check_authorization, log_request}, alert::manager::AlertManager}};
use crate::server::config::Config;
use crate::server::storage::{MemoryStorage, Storage};
use crate::server::scheduler::launch_scheduler;

use super::config::ServerConf;
use super::controller::*;

pub const DEFAULT_PORT: u16 = 3030; 
pub const DEFAULT_ADDRESS: &str = "127.0.0.1"; 

pub struct AppState {
    pub storage: Storage,
    pub config: Arc<Config>,
    pub alert: Arc<AlertManager>
}

pub async fn launch(server_conf: ServerConf) -> Result<(), Error> {

    let storage = MemoryStorage::new();

    let config = Arc::new(
        Config::new(&server_conf.config_path).await?
    );

    let alert_manager = AlertManager::try_from_config(&config.alerters)?;
    let shared_alert = Arc::new(alert_manager);

    let app_state = Arc::new(AppState {
        storage: storage.clone(),
        config: config.clone(),
        alert: shared_alert.clone()
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
        .route(
            "/api/v1/alerting/test",
            post(handle_trigger_alert_test)
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
    let scheduler_alert = shared_alert.clone();
    let scheduler_handle = task::spawn(async move {
        
        println!(" ✓ Watchdog network scheduler is UP");
        println!();
        println!("You can now start region network relays");
        println!("Use the 'relay --region name' command");
        println!();
    
        launch_scheduler(cancel_token_scheduler, scheduler_conf, scheduler_storage, scheduler_alert).await;

    });

    signal::ctrl_c().await.map_err(|err| Error::new("Could not handle graceful shutdown signal", err))?;
    cancel_token.cancel();
    println!("Received graceful shutdown signal");

    let _= web_handle.await.map_err(|err| Error::new("Could not end web task", err))?;
    scheduler_handle.await.map_err(|err| Error::new("Could not end scheduler task", err))?;

    Ok(())
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

