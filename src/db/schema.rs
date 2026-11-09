pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp_utc TEXT NOT NULL,
    channel TEXT NOT NULL,
    record_id INTEGER NOT NULL,
    event_id INTEGER NOT NULL,
    provider TEXT NOT NULL,
    computer TEXT NOT NULL,
    severity TEXT NOT NULL,
    category TEXT NOT NULL,
    summary TEXT NOT NULL,
    username TEXT,
    source_ip TEXT,
    process_name TEXT,
    command_line TEXT,
    fields_json TEXT NOT NULL,
    raw_xml TEXT NOT NULL,
    UNIQUE(channel, record_id)
);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp_utc DESC);
CREATE INDEX IF NOT EXISTS idx_events_channel ON events(channel, timestamp_utc DESC);
CREATE INDEX IF NOT EXISTS idx_events_event_id ON events(event_id, timestamp_utc DESC);
CREATE INDEX IF NOT EXISTS idx_events_source_ip ON events(source_ip, timestamp_utc DESC);

CREATE TABLE IF NOT EXISTS collection_state (
    channel TEXT PRIMARY KEY,
    last_record_id INTEGER NOT NULL,
    updated_utc TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS alerts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_id TEXT NOT NULL,
    fingerprint TEXT NOT NULL UNIQUE,
    severity TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    first_seen_utc TEXT NOT NULL,
    last_seen_utc TEXT NOT NULL,
    event_count INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'open'
);
CREATE INDEX IF NOT EXISTS idx_alerts_status ON alerts(status, last_seen_utc DESC);
"#;
