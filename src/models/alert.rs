use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AlertRecord {
    pub id: i64,
    pub rule_id: String,
    pub fingerprint: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub first_seen_utc: String,
    pub last_seen_utc: String,
    pub event_count: i64,
    pub status: String,
}
