use std::thread::sleep;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::convert::{TryInto, Infallible};

use tokio::{signal, task, sync::oneshot, sync::RwLock};
use warp::Filter;
use chrono::{Duration as ChronoDuration, Utc};

use crate::common::error::ServerError;
use crate::server::config::Config;
use crate::server::storage::{MemoryStorage, Storage, RegionStatus, GroupStatus, GroupState, RegionState};
use crate::relay::instance::GroupResult;
use crate::server::alert::{self, TelegramOptions};

// TODO Should review defaults
const DEFAULT_REGION_MS: i64 = 10 * 1000;
const DEFAULT_GROUP_MS: i64 = 10 * 1000;

pub struct ServerConf {

    pub config_path: String,
    pub port: u16,

    pub telegram_token: Option<String>,
    pub telegram_chat: Option<String>

}

pub async fn launch(server_conf: ServerConf) -> Result<(), ServerError> {

    let storage = MemoryStorage::new();

    let base_config = Config::new(&server_conf.config_path)?;
    let config = Arc::new(base_config);

    init_storage_regions(storage.clone(), config.clone()).await;

    let config_relay_get = config.clone();
    let find_config = warp::get()
        .and(warp::path("api"))
        .and(warp::path("v1"))
        .and(warp::path("relay"))
        .and(warp::path::param())
        .and(with_config(config_relay_get))
        .and_then(handle_get_config);

    let config_relay_put = config.clone();
    let storage_relay_put = storage.clone();
    let update_region_state = warp::put()
        .and(warp::path("api"))
        .and(warp::path("v1"))
        .and(warp::path("relay"))
        .and(warp::path::param())
        .and(warp::body::json())
        .and(with_config(config_relay_put))
        .and(with_storage(storage_relay_put))
        .and_then(handle_region_update);

    let config_analytics = storage.clone();
    let get_analytics = warp::get()
        .and(warp::path("api"))
        .and(warp::path("v1"))
        .and(warp::path("analytics"))
        .and(with_storage(config_analytics))
        .and_then(handle_analytics);

    let not_found = warp::any().map(|| "Not found");

    let routes = find_config
        .or(get_analytics)
        .or(update_region_state)
        .or(not_found);

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
    
        loop {

            if scheduler_terminate.load(Ordering::Relaxed) {
                break;
            }
            
            for region in scheduler_conf.regions.iter() {

                let region_status: Option<RegionStatus>;
                {
                    let scheduler_read = scheduler_storage.read().await;
                    region_status = scheduler_read.get_region_status(&region.name).map(|status| (*status).clone());
                }

                if let Some(status) = region_status {

                    match status.status {
                        RegionState::DOWN => (),
                        RegionState::INITIAL => (),
                        _ => {

                            let region_ms: i64 = region.threshold_ms.try_into().unwrap_or(DEFAULT_REGION_MS);
                            if Utc::now().signed_duration_since(status.updated_at) > ChronoDuration::milliseconds(region_ms) {
                                
                                println!("INCIDENT ON REGION {}", region.name);
                                {
                                    let mut sched_store_mut = scheduler_storage.write().await;
                                    sched_store_mut.trigger_region_incident(&region.name).unwrap_or_else(|err| {
                                        eprintln!("Failed to trigger incident in storage: {}", err);
                                        eprintln!("This error will be ignored but can cause unstable storage");
                                    });
                                }

                                // TODO What if wrong telegram conf ?
                                if let (Some(telegram_token), Some(telegram_chat)) = (&server_conf.telegram_token, &server_conf.telegram_chat) {
                                    let message = format!("Network DOWN on region {}", &region.name);
                                    let options = TelegramOptions {
                                        disable_notifications: false
                                    };
                                    alert::alert_telegram(telegram_token, telegram_chat, &message, options).await.unwrap_or_else(|err| {
                                        eprintln!("Failed to trigger incident notification: {}", err);
                                    });
                                }
                            }

                        }
                    };
                }

                for group in region.groups.iter() {

                    let group_status: Option<GroupStatus>;
                    {
                        let scheduler_read = scheduler_storage.read().await;
                        group_status = scheduler_read.get_group_status(&region.name, &group.name).map(|status| (*status).clone());
                    }

                    if let Some(status) = group_status {

                        match status.status {
                            GroupState::UP | GroupState::INITIAL | GroupState::INCIDENT => (),
                            GroupState::DOWN => {
    
                                let group_ms: i64 = group.threshold_ms.try_into().unwrap_or(DEFAULT_GROUP_MS);
                                if Utc::now().signed_duration_since(status.updated_at) > ChronoDuration::milliseconds(group_ms) {
                                    
                                    println!("INCIDENT ON GROUP {}.{}", region.name, group.name);
                                    {
                                        // TODO Should trigger incident in logs
                                        let mut sched_store_mut = scheduler_storage.write().await;
                                        sched_store_mut.trigger_group_incident(&region.name, &group.name).unwrap_or_else(|err| {
                                            eprintln!("Failed to trigger incident in storage: {}", err);
                                            eprintln!("This error will be ignored but can cause unstable storage");
                                        });
                                    }
    
                                    // TODO What if wrong telegram conf ?
                                    if let (Some(telegram_token), Some(telegram_chat)) = (&server_conf.telegram_token, &server_conf.telegram_chat) {
                                        let message = format!("Network DOWN on group {}.{}", &region.name, &group.name);
                                        let options = TelegramOptions {
                                            disable_notifications: false
                                        };
                                        alert::alert_telegram(telegram_token, telegram_chat, &message, options).await.unwrap_or_else(|err| {
                                            eprintln!("Failed to trigger incident notification: {}", err);
                                        });
                                    }
                                }
    
                            }
                        };
                    }
                }
            }

            sleep(Duration::from_secs(1));
        }

    });

    signal::ctrl_c().await.map_err(|err| ServerError::new("Could not handle graceful shutdown signal", err))?;
    
    terminate_sheduler.store(true, Ordering::Relaxed);
    let _ = server_tx.send(());

    web_handle.await.map_err(|err| ServerError::new("Could not end web task", err))?;
    scheduler_handle.await.map_err(|err| ServerError::new("Could not end scheduler task", err))?;

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

async fn handle_region_update(region_name: String, results: Vec<GroupResult>, _config: Arc<Config>, storage: Storage) -> Result<impl warp::Reply, Infallible> {

    // TODO Blocking RW too long
    {
        let mut write_lock = storage.write().await;

        let mut has_warning = false;
        for group in results {

            if !group.working {
                has_warning = true;
            }

            let state = match group.working {
                true => GroupState::UP,
                false => GroupState::DOWN
            };

            write_lock.refresh_group(&region_name, &group.name, state).unwrap_or_else(|err| {
                eprintln!("Could not refresh group, can cause unstable storage: {}", err);
            });
        }

        let region_status = write_lock.get_region_status(&region_name);

        if let Some(status) = region_status {

            // We already had an incident
            if let RegionState::DOWN = status.status {
                println!("INCIDENT RESOLVED ON REGION {}", region_name);
            }
        }

        write_lock.refresh_region(&region_name, has_warning);
    }

    return Ok(warp::reply::json(&"{}"));
}

async fn handle_analytics(storage: Storage) -> Result<impl warp::Reply, Infallible> {

    let regions = storage.read().await.compute_analytics();

    return Ok(warp::reply::json(&regions));
}