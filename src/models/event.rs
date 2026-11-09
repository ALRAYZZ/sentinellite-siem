use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedEvent {
    pub timestamp_utc: String,
    pub channel: String,
    pub record_id: i64,
    pub event_id: i64,
    pub provider: String,
    pub computer: String,
    pub severity: String,
    pub category: String,
    pub summary: String,
    pub username: Option<String>,
    pub source_ip: Option<String>,
    pub process_name: Option<String>,
    pub command_line: Option<String>,
    pub fields: BTreeMap<String, String>,
    pub raw_xml: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub id: i64,
    pub timestamp_utc: String,
    pub channel: String,
    pub record_id: i64,
    pub event_id: i64,
    pub provider: String,
    pub computer: String,
    pub severity: String,
    pub category: String,
    pub summary: String,
    pub username: Option<String>,
    pub source_ip: Option<String>,
    pub process_name: Option<String>,
    pub command_line: Option<String>,
    pub fields: BTreeMap<String, String>,
    pub raw_xml: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EventFilters {
    pub q: Option<String>,
    pub channel: Option<String>,
    pub severity: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventListResponse {
    pub items: Vec<EventRecord>,
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
}
