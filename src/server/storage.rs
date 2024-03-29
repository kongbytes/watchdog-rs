use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

use crate::common::error::Error;

pub type Storage = Arc<RwLock<MemoryStorage>>;

#[derive(Clone)]
pub enum RegionState {
    Initial,
    Up,
    Warn,
    Down
}

#[derive(Clone)]
pub enum GroupState {
    Initial,
    Up,
    Warn,
    Down,
    Incident
}

#[derive(Clone)]
pub struct RegionStatus {
    pub status: RegionState,
    pub updated_at: DateTime<Utc>,
}

struct RegionMetadata {
    linked_groups: Vec<String>
}

pub struct FullMetric {
    pub name: String,
    pub labels: HashMap<String, String>,
    pub metric: f32
}

#[derive(Clone)]
pub struct GroupMetrics {
    pub name: String,
    pub labels: HashMap<String, String>,
    pub metric: f32
}

#[derive(Clone)]
pub struct GroupStatus {
    pub status: GroupState,
    pub updated_at: DateTime<Utc>,
    pub last_metrics: Vec<GroupMetrics>,
    pub last_error: Option<String>
}

pub struct IncidentRecord {
    pub id: u32,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub error_message: Option<String>,
    pub error_details: Option<String>
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
    pub timestamp: String,
    pub error_message: Option<String>,
    pub error_details: Option<String>
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
            status: RegionState::Initial,
            updated_at: Utc::now(),
        });
        self.region_metadata.insert(region.to_string(), RegionMetadata {
            linked_groups
        });
    }

    pub fn init_group(&mut self, region: &str, group: &str) {

        let group_key = format!("{}.{}", region, group);

        self.group_storage.insert(group_key, GroupStatus {
            status: GroupState::Initial,
            updated_at: Utc::now(),
            last_metrics: vec![],
            last_error: None
        });
    }

    pub fn get_region_status(&self, region: &str) -> Option<&RegionStatus> {
        self.region_storage.get(region)
    }

    pub fn get_group_status(&self, region: &str, group: &str) -> Option<&GroupStatus> {
        
        let group_key = format!("{}.{}", region, group);
        self.group_storage.get(&group_key)
    }

    pub fn collect_test_metrics(&self) -> Vec<FullMetric> {

        let mut metrics: Vec<FullMetric> = vec![];
        for (group_name, group_status) in &self.group_storage {

            for group_metric in &group_status.last_metrics {

                let mut full_labels = group_metric.labels.clone();

                let group_parts: Vec<&str> = group_name.split('.').collect();

                if let Some(region_name) = group_parts.first() {
                    full_labels.insert("region".into(), region_name.to_string());
                }

                if let Some(group_name) = group_parts.last() {
                    full_labels.insert("group".into(), group_name.to_string());
                }

                metrics.push(FullMetric {
                    name: group_metric.name.clone(),
                    labels: full_labels,
                    metric: group_metric.metric
                });
            }
        }

        metrics
    }

    pub fn collect_region_metrics(&self) -> Vec<FullMetric> {

        let mut metrics: Vec<FullMetric> = vec![];
        for (region_key, region_value) in &self.region_storage {

            metrics.push(FullMetric {
                name: "region".to_string(),
                labels: HashMap::from([
                    ("region_name".to_string(), region_key.clone())
                ]),
                metric: match region_value.status {
                    RegionState::Up => 3f32,
                    RegionState::Down => 0f32,
                    RegionState::Initial => 1f32,
                    RegionState::Warn => 2f32,
                }
            });
        }

        metrics
    }

    pub fn find_incidents(&self) -> Vec<IncidentItem> {

        let mut incidents: Vec<IncidentItem> = vec![];
        for incident in &self.incidents {

            incidents.push(IncidentItem {
                id: incident.id,
                message: incident.message.clone(),
                timestamp: incident.timestamp.to_rfc3339(),
                error_message: incident.error_message.clone(),
                error_details: incident.error_details.clone()
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
                timestamp: result.timestamp.to_rfc3339(),
                error_message: result.error_message.clone(),
                error_details: result.error_details.clone()
            })
    }

    pub fn compute_analytics(&self) -> RegionSummary {

        let mut regions: Vec<RegionSummaryItem> = vec![];
        for (region_key, region_value) in &self.region_storage {

            regions.push(RegionSummaryItem {
                name: region_key.to_string(),
                status: match region_value.status {
                    RegionState::Up => "up".to_string(),
                    RegionState::Down => "down".to_string(),
                    RegionState::Initial => "initial".to_string(),
                    RegionState::Warn => "warn".to_string()
                },
                last_update: region_value.updated_at.to_rfc3339()
            });
        }

        let mut groups: Vec<GroupSummaryItem> = vec![];
        for (group_key, group_value) in &self.group_storage {

            groups.push(GroupSummaryItem {
                name: group_key.to_string(),
                status: match group_value.status {
                    GroupState::Up => "up".to_string(),
                    GroupState::Warn => "warn".to_string(),
                    GroupState::Down => "down".to_string(),
                    GroupState::Incident => "incident".to_string(),
                    GroupState::Initial => "initial".to_string()
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
                true => RegionState::Warn,
                false => RegionState::Up
            },
            updated_at: Utc::now()
        });
    }

    pub fn trigger_region_incident(&mut self, region: &str, ms_threshold: i64) -> Result<(), Error> {

        // TODO Should track incident end

        let old_status = self.region_storage.get(region).ok_or_else(|| Error::basic(format!("Could not find region storage {}", region)))?;

        // The 'chrono UTC' type implements the 'Copy' trait and does not
        // require a clone() call, which simplifies ownership. 
        let updated_at = old_status.updated_at;
        
        self.region_storage.insert(region.to_string(), RegionStatus {
            status: RegionState::Down,
            updated_at
        });

        let region_metadata = self.region_metadata.get(region).ok_or_else(|| Error::basic(format!("Could not find region metadata {}", region)))?;
        for impacted_group in &region_metadata.linked_groups {

            self.group_storage.insert(format!("{}.{}", region, impacted_group), GroupStatus {
                status: GroupState::Incident,
                updated_at: Utc::now(),
                last_metrics: vec![],
                last_error: None
            });
        }

        self.incidents.push(IncidentRecord {
            id: self.last_incident_id,
            message: format!("Region {} is DOWN", region),
            timestamp: Utc::now(),
            error_message: Some(format!("Region relay has not sent heartbeat in time ({}ms threshold exceeded)", ms_threshold)),
            error_details: None
        });
        self.last_incident_id += 1;

        Ok(())
    }

    pub fn refresh_group(&mut self, region: &str, group: &str, status: GroupState, last_metrics: Vec<GroupMetrics>, last_error: Option<String>) -> Result<(), Error> {

        let group_key = format!("{}.{}", region, group);
        let updated_at = match status {
            GroupState::Down => {
                // A group marked as 'down' will not be updated, allowing to trigger an incident
                // after X milliseconds without update on the DOWN group
                let old_status = self.group_storage.get(&group_key).ok_or_else(|| Error::basic(format!("Could not find group storage {}", group_key)))?;
                old_status.updated_at
            },
            _ => Utc::now()
        };

        self.group_storage.insert(group_key, GroupStatus {
            status,
            updated_at,
            last_metrics,
            last_error
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

        let last_error = old_status.clone().last_error;
        
        // Move to incident, this will avoid re-trigger alerts
        self.group_storage.insert(group_key, GroupStatus {
            status: GroupState::Incident,
            updated_at,
            last_metrics: old_status.last_metrics.clone(),
            last_error: last_error.clone()
        });

        let error_message = format!("Triggered from group relay ({})", last_error.unwrap_or("-".into()));
        self.incidents.push(IncidentRecord {
            id: self.last_incident_id,
            message: format!("Group {}.{} is DOWN", region, group),
            timestamp: Utc::now(),
            error_message: Some(error_message),
            error_details: None
        });
        self.last_incident_id += 1;

        Ok(())
    }

}
