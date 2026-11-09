use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SeverityCount {
    pub severity: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivityPoint {
    pub hour: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardSummary {
    pub hostname: String,
    pub events_total: i64,
    pub events_24h: i64,
    pub open_alerts: i64,
    pub active_channels: i64,
    pub severities: Vec<SeverityCount>,
    pub activity: Vec<ActivityPoint>,
}
