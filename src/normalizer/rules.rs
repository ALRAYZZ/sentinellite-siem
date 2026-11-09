use anyhow::Result;
use chrono::{Duration, Utc};

use crate::{db::Database, models::NormalizedEvent};

/// Small, transparent starter rules. These are heuristics for an MVP, not a substitute
/// for a tuned production detection program.
pub fn evaluate(database: &Database, event: &NormalizedEvent) -> Result<()> {
    let seen = &event.timestamp_utc;

    let is_process_creation = event.event_id == 4688
        || (event.event_id == 1 && event.provider.to_ascii_lowercase().contains("sysmon"));
    if is_process_creation {
        let command = event.command_line.as_deref().unwrap_or("");
        let process = event.process_name.as_deref().unwrap_or("");
        let lower_command = command.to_ascii_lowercase();
        let lower_process = process.to_ascii_lowercase();
        if (lower_process.contains("powershell") || lower_process.contains("pwsh"))
            && (lower_command.contains("-enc") || lower_command.contains("-encodedcommand"))
        {
            database.upsert_alert(
                "powershell_encoded_command",
                &format!("powershell_encoded_command:{}:{}", event.channel, event.record_id),
                "high",
                "PowerShell encoded command detected",
                &format!("An encoded PowerShell command line was observed on {}.", event.computer),
                seen,
            )?;
        }
    }

    match event.event_id {
        4720 => {
            let account = event.username.as_deref().unwrap_or("unknown account");
            database.upsert_alert(
                "windows_new_account",
                &format!("windows_new_account:{}:{}", event.channel, event.record_id),
                "medium",
                "Windows user account created",
                &format!(
                    "A user account creation event was recorded for {account} on {}.",
                    event.computer
                ),
                seen,
            )?;
        }
        1102 => {
            database.upsert_alert(
                "security_log_cleared",
                &format!("security_log_cleared:{}:{}", event.channel, event.record_id),
                "critical",
                "Windows Security log cleared",
                &format!("A Security log-clearing event was recorded on {}.", event.computer),
                seen,
            )?;
        }
        7045 => {
            let service =
                event.fields.get("ServiceName").map(String::as_str).unwrap_or("unknown service");
            database.upsert_alert(
                "windows_service_installed",
                &format!("windows_service_installed:{}:{}", event.channel, event.record_id),
                "medium",
                "Windows service installed",
                &format!("Service '{service}' was installed on {}.", event.computer),
                seen,
            )?;
        }
        4625 => {
            if let Some(source_ip) = event.source_ip.as_deref() {
                if source_ip != "127.0.0.1" && source_ip != "::1" {
                    let parsed = chrono::DateTime::parse_from_rfc3339(seen)
                        .map(|value| value.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    let since = (parsed - Duration::minutes(5)).to_rfc3339();
                    let failures = database.count_failed_logons(source_ip, &since)?;
                    if failures >= 5 {
                        let bucket = parsed.timestamp() / 300;
                        database.upsert_alert(
                            "repeated_failed_logons",
                            &format!("repeated_failed_logons:{source_ip}:{bucket}"),
                            "high",
                            "Repeated failed logons from one IP",
                            &format!("At least {failures} failed logons from {source_ip} were recorded in a five-minute window."),
                            seen,
                        )?;
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}
