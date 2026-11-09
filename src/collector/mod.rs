mod windows;

use chrono::Utc;
use serde::Serialize;
use std::sync::{Arc, RwLock};
use tokio::time::{sleep, Duration};
use tracing::{debug, warn};

use crate::db::Database;

pub const CHANNELS: &[&str] =
    &["Security", "System", "Application", "Microsoft-Windows-Sysmon/Operational"];

#[derive(Debug, Clone, Serialize)]
pub struct SourceStatus {
    pub channel: String,
    pub status: String,
    pub last_checked_utc: Option<String>,
    pub events_last_poll: u64,
    pub error: Option<String>,
}

impl SourceStatus {
    pub fn starting(channel: &str) -> Self {
        Self {
            channel: channel.to_owned(),
            status: "starting".to_owned(),
            last_checked_utc: None,
            events_last_poll: 0,
            error: None,
        }
    }
}

pub fn start(database: Database, statuses: Arc<RwLock<Vec<SourceStatus>>>) {
    tokio::spawn(async move {
        loop {
            for channel in CHANNELS {
                let db = database.clone();
                let channel_name = (*channel).to_owned();
                let result = tokio::task::spawn_blocking(move || {
                    windows::collect_channel(&db, &channel_name)
                })
                .await;

                let (count, error) = match result {
                    Ok(Ok(count)) => (count, None),
                    Ok(Err(error)) => (0, Some(error.to_string())),
                    Err(error) => (0, Some(format!("collector task failed: {error}"))),
                };

                if let Some(message) = error.as_deref() {
                    debug!(channel = *channel, error = message, "event channel poll failed");
                }

                if let Ok(mut sources) = statuses.write() {
                    if let Some(source) =
                        sources.iter_mut().find(|source| source.channel == *channel)
                    {
                        source.last_checked_utc = Some(Utc::now().to_rfc3339());
                        source.events_last_poll = count as u64;
                        source.error = error.clone();
                        source.status = if error.is_none() {
                            "active".to_owned()
                        } else {
                            "unavailable".to_owned()
                        };
                    }
                }
            }
            sleep(Duration::from_secs(5)).await;
        }
    });
}

pub fn snapshot(statuses: &Arc<RwLock<Vec<SourceStatus>>>) -> Vec<SourceStatus> {
    statuses.read().map(|sources| sources.clone()).unwrap_or_default()
}
