# SentinelLite — Windows SIEM MVP

SentinelLite is a **local-first Windows event monitoring MVP** written primarily in Rust. It collects local Windows Event Log records, extracts structured fields from the event XML, stores both normalized and raw events in SQLite, runs a few transparent starter detections, and presents the results in a React dashboard.

This is a development starting point, not a production SIEM or a substitute for an EDR platform.

## What is included

- Native Windows Event Log access using `wevtapi.dll` through the `windows-sys` crate (`EvtQuery`, `EvtNext`, `EvtRender`, `EvtClose`).
- Five-second polling of `Security`, `System`, `Application`, and `Microsoft-Windows-Sysmon/Operational` channels.
- Initial backfill capped at 250 recent events per channel.
- Per-channel `EventRecordID` checkpoints and duplicate protection using a unique `(channel, record_id)` constraint.
- XML parsing and normalization into common fields: timestamp, provider, channel, event ID, computer, severity, category, username, source IP, process name, command line, and named event data.
- Retention of the complete original event XML for later investigation.
- Local SQLite storage using WAL mode and indexes for common event queries.
- Starter detections for repeated failed logons, new user accounts, service installation, and encoded PowerShell process command lines.
- React + TypeScript dashboard with overview, event explorer, event detail drawer, detections, and source health.
- API bound to `127.0.0.1:8787` only. No remote listener or agent enrollment is enabled in this MVP.

## Architecture

```text
Windows Event Log APIs
        |
        v
Rust collector (five-second polling)
        |
        v
XML parser + event normalizer
        |
        +------> raw XML + normalized fields
        |                     |
        v                     v
Starter detection rules    SQLite
        |                     |
        +----------+----------+
                   |
                   v
             Axum local API
                   |
                   v
          React / TypeScript UI
             127.0.0.1:8787
```

The code uses Rust composition and small modules rather than inheritance-heavy OOP. Platform-specific unsafe Win32 calls are isolated in `src/collector/windows.rs`; the parser, rules, database and API are separate modules.

## Requirements

On a Windows 10/11 development machine:

1. Rust stable with the MSVC toolchain (`rustup` + Visual Studio C++ Build Tools).
2. Node.js 20 or newer and npm.
3. An account permitted to read the desired event channels. The Security channel commonly requires an elevated terminal or membership in the appropriate Windows event-log reader group.
4. Sysmon installed and configured to collect events if you want Sysmon telemetry. SentinelLite does not install or configure Sysmon.

## Run the MVP

Open PowerShell in the project root. The build script installs frontend dependencies, builds the dashboard, formats the Rust source, runs tests and Clippy, and produces a release binary:

```powershell
.\scripts\build.ps1
.\scripts\run.ps1
```

To run manually instead, build the frontend with `cd frontend; npm install; npm run build`, return to the project root, and run `cargo run --release`.

Then open:

```text
http://127.0.0.1:8787
```

For Security channel access, run PowerShell as Administrator. The app only reads event channels; it does not clear, modify, or export them.

To work on the frontend with Vite hot reload, use two PowerShell windows:

```powershell
# Window 1: Rust API / collector
cargo run
```

```powershell
# Window 2: React UI
cd frontend
npm install
npm run dev
```

Open `http://127.0.0.1:5173`. The Vite dev server proxies `/api` calls to the local Rust server.

## Data location

On Windows, the database is stored at:

```text
%LOCALAPPDATA%\SentinelLite\sentinellite.db
```

The database stores event data, raw XML and alert records. Do not upload it publicly: Windows logs can contain usernames, machine names, IP addresses, command lines and other sensitive operational information.

## Starter detection rules

| Rule | Logic | Severity |
|---|---|---|
| Repeated failed logons | At least five Event ID 4625 failures from one non-loopback source IP within a rolling five-minute window | High |
| New user account | Event ID 4720 | Medium |
| Security log cleared | Event ID 1102 | Critical |
| Service installed | Event ID 7045 | Medium |
| Encoded PowerShell | Event ID 4688; PowerShell process with `-enc` or `-encodedcommand` in the command line | High |

These rules are deliberately simple heuristics. They will miss activity, may generate false positives, and need tuning before operational use. Alert closure is currently manual and alerts are not automatically correlated into incidents.

## Important MVP limitations

- **Polling, not push subscriptions:** the collector polls every five seconds. A future iteration should use `EvtSubscribe` and/or a durable bookmark-based subscription.
- **Single machine:** only the local Windows machine is collected. There is no remote agent fleet, enrollment or central ingestion server yet.
- **Initial backfill:** first run reads up to 250 recent records per channel. Subsequent polling uses the per-channel `EventRecordID` checkpoint. A cleared/recreated event channel may require checkpoint reset; robust channel-reset detection is not implemented yet.
- **Message formatting:** the parser extracts the structured XML fields and preserves raw XML. It does not yet resolve publisher resource DLL strings with `EvtFormatMessage`; the dashboard uses concise event summaries plus extracted fields.
- **No production hardening:** no user authentication is needed for a loopback-only prototype, but do not change the listener to a public/network interface without adding authentication, authorization, TLS, rate limits and secure agent enrollment.
- **Permissions and availability:** missing Sysmon, disabled channels or insufficient access are surfaced as unavailable source statuses rather than silently treated as healthy.
- **No installer/service wrapper:** run it interactively for now. Windows Service installation, signed binaries, upgrades and retention management are future work.

## API endpoints

- `GET /api/health`
- `GET /api/summary`
- `GET /api/events?q=&channel=&severity=&limit=&offset=`
- `GET /api/alerts`
- `POST /api/alerts/{id}/close`
- `GET /api/sources`

All endpoints are intended for local UI use and are only reachable through the loopback listener.

## Engineering checks

From the project root:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Frontend:

```powershell
cd frontend
npm run build
```

## License

MIT. See [LICENSE](LICENSE).
