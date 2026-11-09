mod api;
mod collector;
mod db;
mod models;
mod normalizer;

use anyhow::{Context, Result};
use axum::{routing::get, Router};
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tokio::net::TcpListener;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info;

use crate::{api::AppState, collector::SourceStatus, db::Database};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let data_dir = app_data_dir()?;
    std::fs::create_dir_all(&data_dir)
        .with_context(|| format!("could not create data directory {}", data_dir.display()))?;
    let database = Database::open(&data_dir.join("sentinellite.db"))?;

    let source_statuses = Arc::new(RwLock::new(
        collector::CHANNELS.iter().map(|channel| SourceStatus::starting(*channel)).collect(),
    ));

    collector::start(database.clone(), Arc::clone(&source_statuses));

    let state = AppState {
        database,
        source_statuses,
        hostname: std::env::var("COMPUTERNAME").unwrap_or_else(|_| "localhost".to_owned()),
    };

    let frontend_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("frontend/dist");
    let index_file = frontend_dir.join("index.html");
    let static_files = ServeDir::new(&frontend_dir).not_found_service(ServeFile::new(index_file));

    let app = Router::new()
        .route("/api/health", get(api::health))
        .route("/api/summary", get(api::summary))
        .route("/api/events", get(api::events))
        .route("/api/alerts", get(api::alerts))
        .route("/api/alerts/{id}/close", axum::routing::post(api::close_alert))
        .route("/api/sources", get(api::sources))
        .fallback_service(static_files)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Deliberately loopback-only: this MVP has no remote-agent authentication yet.
    let address = SocketAddr::from(([127, 0, 0, 1], 8787));
    let listener = TcpListener::bind(address).await?;
    info!("SentinelLite listening at http://{}", address);
    info!("SQLite data directory: {}", data_dir.display());

    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("Shutdown signal received");
}

fn app_data_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        let root = std::env::var_os("LOCALAPPDATA")
            .context("LOCALAPPDATA is not set; cannot choose a safe data directory")?;
        Ok(PathBuf::from(root).join("SentinelLite"))
    }

    #[cfg(not(windows))]
    {
        let root = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
            .unwrap_or_else(|| PathBuf::from("./data"));
        Ok(root.join("sentinellite"))
    }
}
