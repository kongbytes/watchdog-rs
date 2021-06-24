use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

pub type Storage = Arc<RwLock<MemoryStorage>>;

#[derive(Clone)]
pub enum RegionState {
    INITIAL,
    UP,
    WARN,
    DOWN
}

#[derive(Clone)]
pub struct RegionStatus {
    pub status: RegionState,
    pub updated_at: DateTime<Utc>,
}

struct RegionMetadata {
    linked_groups: Vec<String>
}

pub struct GroupStatus {
    pub is_working: bool,
    pub updated_at: DateTime<Utc>
}

pub struct IncidentRecord {
    pub message: String,
    pub timestamp: DateTime<Utc>
}

pub struct MemoryStorage {
    region_storage: HashMap<String, RegionStatus>,
    region_metadata: HashMap<String, RegionMetadata>,
    group_storage: HashMap<String, GroupStatus>,
    incidents: Vec<IncidentRecord>
}

#[derive(Deserialize,Serialize)]
pub struct RegionSummary {
    pub regions: Vec<RegionSummaryItem>,
    pub groups: Vec<GroupSummaryItem>,
    pub incidents: Vec<IncidentItem>
}

#[derive(Deserialize,Serialize)]
pub struct RegionSummaryItem {
    pub name: String,
    pub status: String,
    pub last_update: String
}

#[derive(Deserialize,Serialize)]
pub struct GroupSummaryItem {
    pub name: String,
    pub is_working: bool,
    pub last_update: String
}

#[derive(Deserialize,Serialize)]
pub struct IncidentItem {
    pub message: String,
    pub timestamp: String
}

impl MemoryStorage {

    pub fn new() -> Storage {
        
        let base_cache = MemoryStorage {
            region_storage: HashMap::new(),
            region_metadata: HashMap::new(),
            group_storage: HashMap::new(),
            incidents: Vec::new()
        };
        Arc::new(RwLock::new(base_cache))
    }

    pub fn init_region(&mut self, region: &str, linked_groups: Vec<String>) {

        self.region_storage.insert(region.to_string(), RegionStatus {
            status: RegionState::INITIAL,
            updated_at: Utc::now(),
        });
        self.region_metadata.insert(region.to_string(), RegionMetadata {
            linked_groups
        });
    }

    pub fn init_group(&mut self, region: &str, group: &str) -> () {

        let group_key = format!("{}.{}", region, group);

        // TODO Should work with states and not set the instant
        self.group_storage.insert(group_key, GroupStatus {
            is_working: false,
            updated_at: Utc::now()
        });
    }

    pub fn get_region_status(&self, region: &str) -> Option<&RegionStatus> {
        self.region_storage.get(region)
    }

    pub fn compute_analytics(&self) -> RegionSummary {

        let mut regions: Vec<RegionSummaryItem> = vec![];
        for (region_key, region_value) in &self.region_storage {

            regions.push(RegionSummaryItem {
                name: region_key.to_string(),
                status: match region_value.status {
                    RegionState::UP => "up".to_string(),
                    RegionState::DOWN => "down".to_string(),
                    RegionState::INITIAL => "initial".to_string(),
                    RegionState::WARN => "warn".to_string()
                },
                last_update: region_value.updated_at.to_rfc3339()
            });
        }

        let mut groups: Vec<GroupSummaryItem> = vec![];
        for (group_key, group_value) in &self.group_storage {

            groups.push(GroupSummaryItem {
                name: group_key.to_string(),
                is_working: group_value.is_working,
                last_update: group_value.updated_at.to_rfc3339()
            });
        }

        let mut incidents: Vec<IncidentItem> = vec![];
        for incident in &self.incidents {

            incidents.push(IncidentItem {
                message: incident.message.clone(),
                timestamp: incident.timestamp.to_rfc3339()
            })
        }

        RegionSummary {
            regions,
            groups,
            incidents
        }
    }

    pub fn refresh_region(&mut self, region: &str, has_warnings: bool) -> () {

        // TODO Should also track unstable states in regions

        self.region_storage.insert(region.to_string(), RegionStatus {
            status: match has_warnings {
                true => RegionState::WARN,
                false => RegionState::UP
            },
            updated_at: Utc::now()
        });
    }

    pub fn trigger_region_incident(&mut self, region: &str) -> () {

        // TODO An incident on a region should also impact all groups
        // TODO Should track incident end

        let old_status = self.region_storage.get(region).unwrap();
        let updated_at = old_status.updated_at.clone();
        
        self.region_storage.insert(region.to_string(), RegionStatus {
            status: RegionState::DOWN,
            updated_at
        });

        /*for impacted_group in &self.region_metadata.get(region).unwrap().linked_groups {

            &self.group_storage.insert(impacted_group.to_string(), GroupStatus {
                is_working: false,
                updated_at: Utc::now()
            });
        }*/

        self.incidents.push(IncidentRecord {
            message: format!("Region {} is DOWN", region),
            timestamp: Utc::now()
        });
    }

    pub fn refresh_group(&mut self, region: &str, group: &str, is_working: bool) -> () {

        let group_key = format!("{}.{}", region, group);
        self.group_storage.insert(group_key, GroupStatus {
            is_working,
            updated_at: Utc::now()
        });
    }

}