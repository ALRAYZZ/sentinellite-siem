mod handlers;

use std::sync::{Arc, RwLock};

use crate::{collector::SourceStatus, db::Database};

#[derive(Clone)]
pub struct AppState {
    pub database: Database,
    pub source_statuses: Arc<RwLock<Vec<SourceStatus>>>,
    pub hostname: String,
}

pub use handlers::{alerts, close_alert, events, health, sources, summary};
