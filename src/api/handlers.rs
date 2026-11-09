use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;

use crate::{
    api::AppState,
    collector,
    models::{AlertRecord, DashboardSummary, EventFilters, EventListResponse},
};

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    status: &'static str,
    app: &'static str,
    platform: &'static str,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        app: "SentinelLite",
        platform: if cfg!(windows) { "windows" } else { "unsupported" },
    })
}

pub async fn summary(
    State(state): State<AppState>,
) -> Result<Json<DashboardSummary>, (StatusCode, String)> {
    state.database.summary(&state.hostname).map(Json).map_err(internal_error)
}

pub async fn events(
    State(state): State<AppState>,
    Query(filters): Query<EventFilters>,
) -> Result<Json<EventListResponse>, (StatusCode, String)> {
    state.database.list_events(&filters).map(Json).map_err(internal_error)
}

pub async fn alerts(
    State(state): State<AppState>,
) -> Result<Json<Vec<AlertRecord>>, (StatusCode, String)> {
    state.database.list_alerts().map(Json).map_err(internal_error)
}

pub async fn close_alert(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, String)> {
    match state.database.close_alert(id).map_err(internal_error)? {
        true => Ok(StatusCode::NO_CONTENT),
        false => Err((StatusCode::NOT_FOUND, "Alert not found or already closed".to_owned())),
    }
}

pub async fn sources(State(state): State<AppState>) -> Json<Vec<collector::SourceStatus>> {
    Json(collector::snapshot(&state.source_statuses))
}

fn internal_error(error: anyhow::Error) -> (StatusCode, String) {
    tracing::error!(error = %error, "API request failed");
    (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_owned())
}
