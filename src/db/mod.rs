mod schema;

use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};

use crate::models::{
    ActivityPoint, AlertRecord, DashboardSummary, EventFilters, EventListResponse, EventRecord,
    NormalizedEvent, SeverityCount,
};

#[derive(Clone)]
pub struct Database {
    connection: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let connection = Connection::open(path)
            .with_context(|| format!("could not open SQLite database at {}", path.display()))?;
        connection.execute_batch(schema::SCHEMA)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self { connection: Arc::new(Mutex::new(connection)) })
    }

    pub fn last_record_id(&self, channel: &str) -> Result<Option<i64>> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT last_record_id FROM collection_state WHERE channel = ?1",
                [channel],
                |row| row.get(0),
            )
            .optional()
            .context("could not read collection checkpoint")
    }

    pub fn insert_event(&self, event: &NormalizedEvent) -> Result<bool> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        let fields_json = serde_json::to_string(&event.fields)?;
        let inserted = transaction.execute(
            "INSERT OR IGNORE INTO events (
                timestamp_utc, channel, record_id, event_id, provider, computer,
                severity, category, summary, username, source_ip, process_name,
                command_line, fields_json, raw_xml
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                event.timestamp_utc,
                event.channel,
                event.record_id,
                event.event_id,
                event.provider,
                event.computer,
                event.severity,
                event.category,
                event.summary,
                event.username,
                event.source_ip,
                event.process_name,
                event.command_line,
                fields_json,
                event.raw_xml,
            ],
        )?;

        // Keep the event and its checkpoint atomic. Also advance the checkpoint when
        // an already-stored event is encountered after a prior interrupted write.
        transaction.execute(
            "INSERT INTO collection_state(channel, last_record_id, updated_utc)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(channel) DO UPDATE SET
                last_record_id = MAX(collection_state.last_record_id, excluded.last_record_id),
                updated_utc = excluded.updated_utc",
            params![event.channel, event.record_id, Utc::now().to_rfc3339()],
        )?;
        transaction.commit()?;
        Ok(inserted > 0)
    }

    pub fn count_failed_logons(&self, source_ip: &str, since_utc: &str) -> Result<i64> {
        let connection = self.lock()?;
        Ok(connection.query_row(
            "SELECT COUNT(*) FROM events
             WHERE event_id = 4625 AND source_ip = ?1 AND timestamp_utc >= ?2",
            params![source_ip, since_utc],
            |row| row.get(0),
        )?)
    }

    pub fn upsert_alert(
        &self,
        rule_id: &str,
        fingerprint: &str,
        severity: &str,
        title: &str,
        description: &str,
        seen_utc: &str,
    ) -> Result<()> {
        let connection = self.lock()?;
        connection.execute(
            "INSERT INTO alerts (
                rule_id, fingerprint, severity, title, description,
                first_seen_utc, last_seen_utc, event_count, status
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, 1, 'open')
             ON CONFLICT(fingerprint) DO UPDATE SET
                last_seen_utc = excluded.last_seen_utc,
                event_count = alerts.event_count + 1,
                status = 'open'",
            params![rule_id, fingerprint, severity, title, description, seen_utc],
        )?;
        Ok(())
    }

    pub fn list_events(&self, filters: &EventFilters) -> Result<EventListResponse> {
        let connection = self.lock()?;
        let limit = filters.limit.unwrap_or(100).clamp(1, 250);
        let offset = filters.offset.unwrap_or(0);
        let query = filters.q.as_deref().filter(|value| !value.trim().is_empty());
        let channel = filters.channel.as_deref().filter(|value| !value.trim().is_empty());
        let severity = filters.severity.as_deref().filter(|value| !value.trim().is_empty());
        let search = query.map(|value| format!("%{}%", value.to_lowercase()));

        let where_sql = "(?1 IS NULL OR channel = ?1)
            AND (?2 IS NULL OR severity = ?2)
            AND (?3 IS NULL OR lower(
                coalesce(summary, '') || ' ' || coalesce(provider, '') || ' ' ||
                coalesce(computer, '') || ' ' || coalesce(username, '') || ' ' ||
                coalesce(source_ip, '') || ' ' || coalesce(process_name, '') || ' ' ||
                coalesce(command_line, '') || ' ' || coalesce(fields_json, '') || ' ' ||
                coalesce(cast(event_id AS TEXT), '')
            ) LIKE ?3)";
        let count_sql = format!("SELECT COUNT(*) FROM events WHERE {where_sql}");
        let total: i64 =
            connection
                .query_row(&count_sql, params![channel, severity, search], |row| row.get(0))?;

        let sql = format!(
            "SELECT id, timestamp_utc, channel, record_id, event_id, provider, computer,
                    severity, category, summary, username, source_ip, process_name,
                    command_line, fields_json, raw_xml
             FROM events WHERE {where_sql}
             ORDER BY timestamp_utc DESC, id DESC LIMIT ?4 OFFSET ?5"
        );
        let mut statement = connection.prepare(&sql)?;
        let rows =
            statement.query_map(params![channel, severity, search, limit, offset], |row| {
                let fields_json: String = row.get(14)?;
                let fields: BTreeMap<String, String> =
                    serde_json::from_str(&fields_json).unwrap_or_default();
                Ok(EventRecord {
                    id: row.get(0)?,
                    timestamp_utc: row.get(1)?,
                    channel: row.get(2)?,
                    record_id: row.get(3)?,
                    event_id: row.get(4)?,
                    provider: row.get(5)?,
                    computer: row.get(6)?,
                    severity: row.get(7)?,
                    category: row.get(8)?,
                    summary: row.get(9)?,
                    username: row.get(10)?,
                    source_ip: row.get(11)?,
                    process_name: row.get(12)?,
                    command_line: row.get(13)?,
                    fields,
                    raw_xml: row.get(15)?,
                })
            })?;
        let items = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(EventListResponse { items, total, limit, offset })
    }

    pub fn list_alerts(&self) -> Result<Vec<AlertRecord>> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT id, rule_id, fingerprint, severity, title, description,
                    first_seen_utc, last_seen_utc, event_count, status
             FROM alerts ORDER BY CASE status WHEN 'open' THEN 0 ELSE 1 END,
                    last_seen_utc DESC LIMIT 250",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(AlertRecord {
                id: row.get(0)?,
                rule_id: row.get(1)?,
                fingerprint: row.get(2)?,
                severity: row.get(3)?,
                title: row.get(4)?,
                description: row.get(5)?,
                first_seen_utc: row.get(6)?,
                last_seen_utc: row.get(7)?,
                event_count: row.get(8)?,
                status: row.get(9)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn close_alert(&self, id: i64) -> Result<bool> {
        let connection = self.lock()?;
        Ok(connection.execute(
            "UPDATE alerts SET status = 'closed' WHERE id = ?1 AND status = 'open'",
            [id],
        )? > 0)
    }

    pub fn summary(&self, hostname: &str) -> Result<DashboardSummary> {
        let connection = self.lock()?;
        let now = Utc::now();
        let since = (now - Duration::hours(24)).to_rfc3339();
        let events_total: i64 =
            connection.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        let events_24h: i64 = connection.query_row(
            "SELECT COUNT(*) FROM events WHERE timestamp_utc >= ?1",
            [&since],
            |row| row.get(0),
        )?;
        let open_alerts: i64 = connection.query_row(
            "SELECT COUNT(*) FROM alerts WHERE status = 'open'",
            [],
            |row| row.get(0),
        )?;
        let active_channels: i64 = connection.query_row(
            "SELECT COUNT(DISTINCT channel) FROM events WHERE timestamp_utc >= ?1",
            [&since],
            |row| row.get(0),
        )?;

        let mut severity_statement = connection.prepare(
            "SELECT severity, COUNT(*) FROM events WHERE timestamp_utc >= ?1 GROUP BY severity",
        )?;
        let severities = severity_statement
            .query_map([&since], |row| {
                Ok(SeverityCount { severity: row.get(0)?, count: row.get(1)? })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut activity = Vec::with_capacity(24);
        for hour_offset in (0..24).rev() {
            let start = now - Duration::hours(hour_offset);
            let end = start + Duration::hours(1);
            let start_text = start.to_rfc3339();
            let end_text = end.to_rfc3339();
            let count: i64 = connection.query_row(
                "SELECT COUNT(*) FROM events WHERE timestamp_utc >= ?1 AND timestamp_utc < ?2",
                params![start_text, end_text],
                |row| row.get(0),
            )?;
            activity.push(ActivityPoint { hour: start.format("%H:00").to_string(), count });
        }

        Ok(DashboardSummary {
            hostname: hostname.to_owned(),
            events_total,
            events_24h,
            open_alerts,
            active_channels,
            severities,
            activity,
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.connection.lock().map_err(|_| anyhow::anyhow!("database mutex poisoned"))
    }
}
