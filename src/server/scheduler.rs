use std::thread::sleep;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::convert::TryInto;

use chrono::{Duration as ChronoDuration, Utc};

use crate::server::storage::{RegionStatus, GroupStatus, GroupState, RegionState};
use crate::server::alert::{self, TelegramOptions};
use crate::server::engine::ServerConf;
use crate::server::storage::Storage;
use crate::server::config::Config;

// TODO Should review defaults
const DEFAULT_REGION_MS: i64 = 10 * 1000;
const DEFAULT_GROUP_MS: i64 = 10 * 1000;

pub async fn launch_scheduler(terminate: Arc<AtomicBool>, conf: Arc<Config>, storage: Storage, server_conf: &ServerConf) {

    loop {

        if terminate.load(Ordering::Relaxed) {
            break;
        }
        
        for region in conf.regions.iter() {

            let region_status: Option<RegionStatus>;
            {
                let scheduler_read = storage.read().await;
                region_status = scheduler_read.get_region_status(&region.name).map(|status| (*status).clone());
            }

            if let Some(status) = region_status {

                match status.status {
                    RegionState::Down => (),
                    RegionState::Initial => (),
                    _ => {

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
                            if let (Some(telegram_token), Some(telegram_chat)) = (&server_conf.telegram_token, &server_conf.telegram_chat) {
                                
                                let options = TelegramOptions {
                                    disable_notifications: false
                                };
                                alert::alert_telegram(telegram_token, telegram_chat, &message, options).await.unwrap_or_else(|err| {
                                    eprintln!("Failed to trigger incident notification: {}", err);
                                });
                            }
                            else {
                                alert::display_warning(&message);
                            }
                        }

                    }
                };
            }

            for group in region.groups.iter() {

                let group_status: Option<GroupStatus>;
                {
                    let scheduler_read = storage.read().await;
                    group_status = scheduler_read.get_group_status(&region.name, &group.name).map(|status| (*status).clone());
                }

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
}
