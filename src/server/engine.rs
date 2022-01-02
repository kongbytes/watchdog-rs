use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::convert::Infallible;

use tokio::{signal, task, sync::oneshot, sync::RwLock};
use warp::{Filter, Rejection, Reply, http::Response, http::StatusCode, reply};
use warp::reject::{MissingHeader, MethodNotAllowed, Reject};
use serde::Serialize;

use crate::common::error::Error;
use crate::server::config::Config;
use crate::server::storage::{MemoryStorage, Storage, GroupState, RegionState};
use crate::relay::instance::GroupResult;
use crate::server::scheduler::launch_scheduler;

pub const DEFAULT_PORT: u16 = 3030; 

pub struct ServerConf {

    pub config_path: String,
    pub port: u16,
    pub token: String,

    pub telegram_token: Option<String>,
    pub telegram_chat: Option<String>

}

#[derive(Serialize)]
pub struct ServerErr {
    
    pub status: u16,
    pub message: String

}

#[derive(Debug)]
struct InvalidToken;
impl Reject for InvalidToken {}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "API route not found";
    } else if err.find::<InvalidToken>().is_some() {
        code = StatusCode::FORBIDDEN;
        message = "API token is invalid";
    } else if let Some(missing_header) = err.find::<MissingHeader>() {
        if missing_header.name() == "authorization" {
            code = StatusCode::UNAUTHORIZED;
            message = "Missing API token in request"
        } else {
            code = StatusCode::BAD_REQUEST;
            message = "Missing header in request";
        }
    }
    else if err.find::<MethodNotAllowed>().is_some() {
        code = StatusCode::NOT_FOUND;
        message = "API route not found";
    }
    else {
        eprintln!("[ERR] Got an unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Server side error, please retry later";
    }

    let json = warp::reply::json(&ServerErr {
        status: code.as_u16(),
        message: message.into(),
    });
    Ok(warp::reply::with_status(json, code))
}

fn check_authorization(token: &str) -> impl Filter<Extract = (), Error = warp::Rejection> + Clone {

    let owned_token = token.to_string();
    warp::header::<String>("authorization")
        .map(move |auth_header: String| (owned_token.to_string(), auth_header))
        .and_then(|(conf_token, auth_header): (String, String)| async move {

            if auth_header != format!("Bearer {}", conf_token) {
                return Err(warp::reject::custom(InvalidToken));
            }

            Ok(())
    
        })
        .untuple_one()
}

pub async fn launch(server_conf: ServerConf) -> Result<(), Error> {

    let storage = MemoryStorage::new();

    let base_config = Config::new(&server_conf.config_path)?;

    if base_config.has_medium("telegram") && (server_conf.telegram_chat.is_none() || server_conf.telegram_token.is_none()) {
        let error_message = "Current configuration is using telegram medium, but missing environment variables".to_string();
        return Err(Error::basic(error_message));
    }

    let config = Arc::new(base_config);

    init_storage_regions(storage.clone(), config.clone()).await;

    let base_path = warp::path::end()
        .map(|| Response::builder()
            .status(404)
            .header("Content-Type", "text/html")
            .header("Cache-Control", "no-cache")
            .header("Connection", "close")
            .body("")
            .expect("Could not build base path response"));

    let find_config = warp::get()
        .and(warp::path!("api" / "v1" / "relay" / String))
        .and(check_authorization(&server_conf.token))
        .and(with_config(config.clone()))
        .and_then(handle_get_config);

    let update_region_state = warp::put()
        .and(warp::path!("api" / "v1" / "relay" / String))
        .and(check_authorization(&server_conf.token))
        .and(warp::body::json())
        .and(with_config(config.clone()))
        .and(with_storage(storage.clone()))
        .and_then(handle_region_update);

    let get_analytics = warp::get()
        .and(warp::path!("api" / "v1" / "analytics"))
        .and(check_authorization(&server_conf.token))
        .and(with_storage(storage.clone()))
        .and_then(handle_analytics);

    let find_incidents = warp::get()
        .and(warp::path!("api" / "v1" / "incidents"))
        .and(check_authorization(&server_conf.token))
        .and(with_storage(storage.clone()))
        .and_then(handle_find_incidents);

    let get_incident = warp::get()
        .and(warp::path!("api" / "v1" / "incidents" / u32))
        .and(check_authorization(&server_conf.token))
        .and(with_storage(storage.clone()))
        .and_then(handle_get_incident);

    let routes = base_path
        .or(find_config)
        .or(get_analytics)
        .or(update_region_state)
        .or(find_incidents)
        .or(get_incident)
        .recover(handle_rejection);

    let (server_tx, server_rx) = oneshot::channel();

    let (_address, server) = warp::serve(routes)
        .bind_with_graceful_shutdown(([0, 0, 0, 0], server_conf.port), async {
            server_rx.await.ok();
            println!("Received graceful shutdown signal");
        });

    let web_handle = task::spawn(server);

    println!();
    println!(" ✓ Watchdog monitoring API is UP");

    let terminate_sheduler = Arc::new(AtomicBool::new(false));

    let scheduler_terminate = terminate_sheduler.clone();
    let scheduler_conf = config.clone();
    let scheduler_storage = storage.clone();
    let scheduler_handle = task::spawn(async move {
        
        println!(" ✓ Watchdog network scheduler is UP");
        println!();
        println!("You can now start region network relays");
        println!("Use the 'relay --region name' command");
        println!();
    
        launch_scheduler(scheduler_terminate, scheduler_conf, scheduler_storage, &server_conf).await;

    });

    signal::ctrl_c().await.map_err(|err| Error::new("Could not handle graceful shutdown signal", err))?;
    
    terminate_sheduler.store(true, Ordering::Relaxed);
    let _ = server_tx.send(());

    web_handle.await.map_err(|err| Error::new("Could not end web task", err))?;
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

fn with_config(config: Arc<Config>) -> impl Filter<Extract = (Arc<Config>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || config.clone())
}

fn with_storage(storage: Storage) -> impl Filter<Extract = (Storage,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || storage.clone())
}

async fn handle_get_config(region_name: String, config: Arc<Config>) -> Result<impl warp::Reply, Infallible> {

    match config.export_region(&region_name) {
        Some(exported_config) => Ok(warp::reply::json(exported_config)),
        None => Ok(warp::reply::json(&"{}"))
    }   
}

async fn handle_region_update(region_name: String, results: Vec<GroupResult>, config: Arc<Config>, storage: Storage) -> Result<impl warp::Reply, Infallible> {

    // TODO Blocking RW too long
    {
        let mut write_lock = storage.write().await;

        let mut has_warning = false;
        for group in results {

            if !group.working {
                has_warning = true;
            }

            let state = match group.working {
                true => GroupState::Up,
                false => GroupState::Down
            };

            let current_status = write_lock.get_group_status(&region_name, &group.name).map(|state| state.status.clone());
        
            // If there is an incident on the group and the group is -still- not working,
            // do not override values (can re-trigger incidents otherwise)
            if matches!(current_status, Some(GroupState::Incident)) && !group.working {
                continue;
            }

            write_lock.refresh_group(&region_name, &group.name, state).unwrap_or_else(|err| {
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

    let response = Response::builder()
        .status(200)
        .header("Cache-Control", "no-cache")
        .header("Connection", "close")
        .header("X-Watchdog-Update", &config.version)
        .body("{}")
        .unwrap();

    Ok(response)
}

async fn handle_analytics(storage: Storage) -> Result<impl warp::Reply, Infallible> {

    let regions = storage.read().await.compute_analytics();

    Ok(warp::reply::json(&regions))
}

async fn handle_find_incidents(storage: Storage) -> Result<impl warp::Reply, Infallible> {

    let incidents = storage.read().await.find_incidents();

    Ok(warp::reply::json(&incidents))
}

async fn handle_get_incident(incident_id: u32, storage: Storage) -> Result<impl warp::Reply, Infallible> {

    let incident_result = storage.read().await.get_incident(incident_id);

    match incident_result {
        Some(incident) => {

            let json_response = reply::json(&incident);
            Ok(reply::with_status(json_response, StatusCode::OK))
        },
        None => {

            let error_body = ServerErr {
                status: 404,
                message: "Could not find incident".to_string()
            };
            let json_response = reply::json(&error_body);
            Ok(reply::with_status(json_response, StatusCode::NOT_FOUND))
        }
    }
}
