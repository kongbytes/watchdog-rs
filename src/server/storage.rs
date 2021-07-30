use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

use crate::common::error::Error;

pub type Storage = Arc<RwLock<MemoryStorage>>;

#[derive(Clone)]
pub enum RegionState {
    INITIAL,
    UP,
    WARN,
    DOWN
}

#[derive(Clone)]
pub enum GroupState {
    INITIAL,
    UP,
    DOWN,
    INCIDENT
}

#[derive(Clone)]
pub struct RegionStatus {
    pub status: RegionState,
    pub updated_at: DateTime<Utc>,
}

struct RegionMetadata {
    linked_groups: Vec<String>
}

#[derive(Clone)]
pub struct GroupStatus {
    pub status: GroupState,
    pub updated_at: DateTime<Utc>
}

pub struct IncidentRecord {
    pub id: u32,
    pub message: String,
    pub timestamp: DateTime<Utc>
}

pub struct MemoryStorage {
    region_storage: HashMap<String, RegionStatus>,
    region_metadata: HashMap<String, RegionMetadata>,
    group_storage: HashMap<String, GroupStatus>,
    incidents: Vec<IncidentRecord>,
    last_incident_id: u32
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
    pub status: String,
    pub last_update: String
}

#[derive(Deserialize,Serialize)]
pub struct IncidentItem {
    pub id: u32,
    pub message: String,
    pub timestamp: String
}

impl MemoryStorage {

    pub fn new() -> Storage {
        
        let base_cache = MemoryStorage {
            region_storage: HashMap::new(),
            region_metadata: HashMap::new(),
            group_storage: HashMap::new(),
            incidents: Vec::new(),
            last_incident_id: 0
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

    pub fn init_group(&mut self, region: &str, group: &str) {

        let group_key = format!("{}.{}", region, group);

        self.group_storage.insert(group_key, GroupStatus {
            status: GroupState::INITIAL,
            updated_at: Utc::now()
        });
    }

    pub fn get_region_status(&self, region: &str) -> Option<&RegionStatus> {
        self.region_storage.get(region)
    }

    pub fn get_group_status(&self, region: &str, group: &str) -> Option<&GroupStatus> {
        
        let group_key = format!("{}.{}", region, group);
        self.group_storage.get(&group_key)
    }

    pub fn find_incidents(&self) -> Vec<IncidentItem> {

        let mut incidents: Vec<IncidentItem> = vec![];
        for incident in &self.incidents {

            incidents.push(IncidentItem {
                id: incident.id,
                message: incident.message.clone(),
                timestamp: incident.timestamp.to_rfc3339()
            })
        }

        incidents
    }

    pub fn get_incident(&self, incident_id: u32) -> Option<IncidentItem> {
        
        self.incidents.iter()
            .find(|incident| incident.id == incident_id)
            .map(|result| IncidentItem {
                id: result.id,
                message: result.message.clone(),
                timestamp: result.timestamp.to_rfc3339()
            })
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
                status: match group_value.status {
                    GroupState::UP => "up".to_string(),
                    GroupState::DOWN => "down".to_string(),
                    GroupState::INCIDENT => "incident".to_string(),
                    GroupState::INITIAL => "initial".to_string()
                },
                last_update: group_value.updated_at.to_rfc3339()
            });
        }

        let incidents = self.find_incidents();

        RegionSummary {
            regions,
            groups,
            incidents
        }
    }

    pub fn refresh_region(&mut self, region: &str, has_warnings: bool) {

        // TODO Should also track unstable states in regions

        self.region_storage.insert(region.to_string(), RegionStatus {
            status: match has_warnings {
                true => RegionState::WARN,
                false => RegionState::UP
            },
            updated_at: Utc::now()
        });
    }

    pub fn trigger_region_incident(&mut self, region: &str) -> Result<(), Error> {

        // TODO Should track incident end

        let old_status = self.region_storage.get(region).ok_or_else(|| Error::basic(format!("Could not find region storage {}", region)))?;

        // The 'chrono UTC' type implements the 'Copy' trait and does not
        // require a clone() call, which simplifies ownership. 
        let updated_at = old_status.updated_at;
        
        self.region_storage.insert(region.to_string(), RegionStatus {
            status: RegionState::DOWN,
            updated_at
        });

        let region_metadata = self.region_metadata.get(region).ok_or_else(|| Error::basic(format!("Could not find region metadata {}", region)))?;
        for impacted_group in &region_metadata.linked_groups {

            self.group_storage.insert(format!("{}.{}", region, impacted_group), GroupStatus {
                status: GroupState::INCIDENT,
                updated_at: Utc::now()
            });
        }

        self.incidents.push(IncidentRecord {
            id: self.last_incident_id,
            message: format!("Region {} is DOWN", region),
            timestamp: Utc::now()
        });
        self.last_incident_id += 1;

        Ok(())
    }

    pub fn refresh_group(&mut self, region: &str, group: &str, status: GroupState) -> Result<(), Error> {

        let group_key = format!("{}.{}", region, group);
        let updated_at = match status {
            GroupState::DOWN => {
                let old_status = self.group_storage.get(&group_key).ok_or_else(|| Error::basic(format!("Could not find group storage {}", group_key)))?;
                old_status.updated_at
            },
            _ => Utc::now()
        };

        self.group_storage.insert(group_key, GroupStatus {
            status,
            updated_at
        });

        Ok(())
    }

    pub fn trigger_group_incident(&mut self, region: &str, group: &str) -> Result<(), Error> {

        // TODO Should track incident end

        let group_key = format!("{}.{}", region, group);
        let old_status = self.group_storage.get(&group_key).ok_or_else(|| Error::basic(format!("Could not find group storage {}", group_key)))?;

        // The 'chrono UTC' type implements the 'Copy' trait and does not
        // require a clone() call, which simplifies ownership. 
        let updated_at = old_status.updated_at;
        
        // Move to incident, this will avoid re-trigger alerts
        self.group_storage.insert(group_key, GroupStatus {
            status: GroupState::INCIDENT,
            updated_at
        });

        self.incidents.push(IncidentRecord {
            id: self.last_incident_id,
            message: format!("Group {}.{} is DOWN", region, group),
            timestamp: Utc::now()
        });
        self.last_incident_id += 1;

        Ok(())
    }

}
