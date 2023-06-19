use std::sync::Arc;
use std::convert::TryInto;

use tokio::time::{sleep, Duration};
use chrono::{Duration as ChronoDuration, Utc};
use tokio_util::sync::CancellationToken;

use crate::server::storage::{RegionStatus, GroupStatus, GroupState, RegionState};
use crate::server::storage::Storage;
use crate::server::config::Config;

use super::alert::manager::AlertManager;
use super::config::{RegionConfig, GroupConfig};

// TODO Should review defaults
const DEFAULT_REGION_MS: i64 = 10 * 1000;
const DEFAULT_GROUP_MS: i64 = 10 * 1000;

pub async fn launch_scheduler(cancel_token: CancellationToken, conf: Arc<Config>, storage: Storage, manager: Arc<AlertManager>) {

    loop {
        
        for region in conf.regions.iter() {

            let region_status: Option<RegionStatus>;
            {
                let scheduler_read = storage.read().await;
                region_status = scheduler_read.get_region_status(&region.name).map(|status| (*status).clone());
            }

            trigger_region_incident(region, region_status, storage.clone(), manager.clone()).await;

            for group in region.groups.iter() {

                let group_status: Option<GroupStatus>;
                {
                    let scheduler_read = storage.read().await;
                    group_status = scheduler_read.get_group_status(&region.name, &group.name).map(|status| (*status).clone());
                }

                trigger_group_incident(region, group, group_status, storage.clone(), manager.clone()).await;
            }
        }

        let mut cancel_loop = false;

        tokio::select! {
            _ = cancel_token.cancelled() => {
                cancel_loop = true;
            }
            _ = sleep(Duration::from_secs(1)) => {
                // Sleep went well... on to the next tests
            }
        };

        if cancel_loop {
            break;
        }
    }
}


async fn trigger_region_incident(region: &RegionConfig, region_status: Option<RegionStatus>, storage: Storage, manager: Arc<AlertManager>) {

    if let Some(status) = region_status {

        match status.status {
            RegionState::Down | RegionState::Initial => (),
            RegionState::Up | RegionState::Warn => {

                let region_ms: i64 = region.threshold_ms.try_into().unwrap_or(DEFAULT_REGION_MS);
                if Utc::now().signed_duration_since(status.updated_at) > ChronoDuration::milliseconds(region_ms) {
                    
                    println!("INCIDENT ON REGION {}", region.name);
                    {
                        let mut sched_store_mut = storage.write().await;
                        sched_store_mut.trigger_region_incident(&region.name).unwrap_or_else(|err| {
                            eprintln!("Failed to trigger incident in storage: {}", err);
                            eprintln!("This error will be ignored but can cause unstable storage");
                        });
                    }

                    let message = format!("Network DOWN on region {}", &region.name);
                    manager.alert(None, &message).await.unwrap_or_else(|err| {
                        eprintln!("Error while triggering alert: {}", err);
                    });
                }

            }
        };
    }
}

async fn trigger_group_incident(region: &RegionConfig, group: &GroupConfig, group_status: Option<GroupStatus>, storage: Storage, manager: Arc<AlertManager>) {

    if let Some(status) = group_status {

        match status.status {
            GroupState::Up | GroupState::Initial | GroupState::Warn | GroupState::Incident => (),
            GroupState::Down => {

                let group_ms: i64 = group.threshold_ms.try_into().unwrap_or(DEFAULT_GROUP_MS);
                if Utc::now().signed_duration_since(status.updated_at) > ChronoDuration::milliseconds(group_ms) {
                    
                    println!("INCIDENT ON GROUP {}.{}", region.name, group.name);
                    {
                        // TODO Should trigger incident in logs
                        let mut sched_store_mut = storage.write().await;
                        sched_store_mut.trigger_group_incident(&region.name, &group.name).unwrap_or_else(|err| {
                            eprintln!("Failed to trigger incident in storage: {}", err);
                            eprintln!("This error will be ignored but can cause unstable storage");
                        });
                    }

                    let message = format!("Network DOWN on group {}.{}", &region.name, &group.name);
                    manager.alert(None, &message).await.unwrap_or_else(|err| {
                        eprintln!("Error while triggering alert: {}", err);
                    });
                }

            }
        };
    }
}
