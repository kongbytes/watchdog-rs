use std::thread::sleep;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::convert::Infallible;
use std::time::Duration;

use tokio::signal;
use tokio::task;
use tokio::sync::oneshot;
use warp::Filter;

use crate::server::config::Config;
use crate::server::storage::{MemoryStorage, Storage, RegionStatus, RegionState};

pub async fn launch(config_path: &str) {

    let storage = MemoryStorage::new();

    let config = Arc::new(Config::new(config_path).unwrap_or_else(|err| { 
        eprintln!("{}", err);
        std::process::exit(1);
    }));

    {
        let mut m = storage.write().await;
                
        for region in config.regions.iter() {
            
            let mut linked_groups: Vec<String> = vec![];
            for group in region.groups.iter() {
                m.init_group(&region.name, &group.name);
                linked_groups.push(group.name.to_string())
            }

            m.init_region(&region.name, linked_groups);
        }
    }

    let terminate = Arc::new(AtomicBool::new(false));

    let (tx, rx) = oneshot::channel();

    let y = config.clone();
    let find_config = warp::get()
        .and(warp::path("api"))
        .and(warp::path("v1"))
        .and(warp::path("relay"))
        .and(warp::path::param())
        .and(with_config(y))
        .and_then(handle_get_config);

    let x = config.clone();
    let z = storage.clone();
    let update_region_state = warp::put()
        .and(warp::path("api"))
        .and(warp::path("v1"))
        .and(warp::path("relay"))
        .and(warp::path::param())
        .and(warp::body::json())
        .and(with_config(x))
        .and(with_storage(z))
        .and_then(handle_region_update);

    let z1 = storage.clone();
    let get_analytics = warp::get()
        .and(warp::path("api"))
        .and(warp::path("v1"))
        .and(warp::path("analytics"))
        .and(with_storage(z1))
        .and_then(handle_analytics);

    let not_found = warp::any().map(|| "Not found");

    let routes = find_config
        .or(get_analytics)
        .or(update_region_state)
        .or(not_found);

    let (_addr, server) = warp::serve(routes)
        .bind_with_graceful_shutdown(([127, 0, 0, 1], 3030), async {
            rx.await.ok();
            println!("Received graceful shutdown signal");
        });

    let web_handle = task::spawn(server);

    let scheduler_terminate = terminate.clone();
    let scheduler_conf = config.clone();
    let scheduler_storage = storage.clone();
    let scheduler_handle = task::spawn(async move {
        
        println!("Spawning scheduler");
        loop {

            if scheduler_terminate.load(Ordering::Relaxed) {
                break;
            }
            
            for region in scheduler_conf.regions.iter() {

                // Check regions iter
                // TODO let interval = region.interval;
                let interval_seconds: i64 = 10; // TODO parse time region.interval.parse().unwrap();

                let region_status: Option<RegionStatus>;
                {
                    let ux = scheduler_storage.read().await;
                    let zzz = ux.get_region_status(&region.name).map(|status| (*status).clone());
                    region_status = zzz;
                }

                match region_status {
                    Some(status) => {

                        match status.status {
                            RegionState::DOWN => (),
                            RegionState::INITIAL => (),
                            _ => {

                                if chrono::Utc::now().signed_duration_since(status.updated_at) > chrono::Duration::seconds(interval_seconds) {
                                    println!("INCIDENT ON REGION {}", region.name);
                                    {
                                        let mut sched_store_mut = scheduler_storage.write().await;
                                        sched_store_mut.trigger_region_incident(&region.name);
                                    }
                                }

                            }
                        };

                    },
                    None => ()
                };

            }

            sleep(Duration::from_secs(1));
        }

    });
    
    signal::ctrl_c().await.expect("Should handle CTRL+C");
    
    terminate.store(true, Ordering::Relaxed);
    let _ = tx.send(());

    web_handle.await.expect("Should end web task");
    scheduler_handle.await.expect("Should end scheduler task");
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

async fn handle_region_update(region_name: String, results: Vec<crate::relay::relay::GroupResult>, _config: Arc<Config>, storage: Storage) -> Result<impl warp::Reply, Infallible> {

    // TODO Blocking RW too long
    {
        let mut w = storage.write().await;

        let mut has_warning = false;
        for group in results {

            if group.working == false {
                has_warning = true;
            }

            w.refresh_group(&region_name, &group.name, group.working)
        }

        let region_status = w.get_region_status(&region_name);

        match region_status {
            Some(status) => {

                // We already had an incident
                if let RegionState::DOWN = status.status {
                    println!("INCIDENT RESOLVED ON REGION {}", region_name);
                }

            },
            None => ()
        };

        w.refresh_region(&region_name, has_warning);
    }

    return Ok(warp::reply::json(&"{}"));
}

async fn handle_analytics(storage: Storage) -> Result<impl warp::Reply, Infallible> {

    let regions = storage.read().await.compute_analytics();

    return Ok(warp::reply::json(&regions));
}