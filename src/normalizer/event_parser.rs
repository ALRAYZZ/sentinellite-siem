use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};
use std::collections::BTreeMap;

use crate::models::NormalizedEvent;

pub fn parse_windows_event_xml(xml: &str, requested_channel: &str) -> Result<NormalizedEvent> {
    let mut reader = Reader::from_str(xml);
    // Preserve spaces in command lines and other event payloads; trim only when a Data node closes.
    reader.config_mut().trim_text(false);

    let mut stack: Vec<String> = Vec::new();
    let mut fields = BTreeMap::new();
    let mut data_name: Option<String> = None;
    let mut timestamp = None;
    let mut provider = String::from("unknown");
    let mut channel = requested_channel.to_owned();
    let mut computer = String::from("unknown");
    let mut event_id = None;
    let mut record_id = None;
    let mut level = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let tag = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                inspect_attributes(
                    &element,
                    reader.decoder(),
                    &mut provider,
                    &mut timestamp,
                    &mut data_name,
                );
                stack.push(tag);
            }
            Ok(Event::Empty(element)) => {
                let tag = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                inspect_attributes(
                    &element,
                    reader.decoder(),
                    &mut provider,
                    &mut timestamp,
                    &mut data_name,
                );
                if tag == "Data" {
                    if let Some(name) = data_name.take() {
                        fields.entry(name).or_default();
                    }
                }
            }
            Ok(Event::Text(text)) => {
                let decoded = text.decode()?.into_owned();
                let value = quick_xml::escape::unescape(&decoded)
                    .map(|text| text.into_owned())
                    .unwrap_or(decoded);
                match stack.last().map(String::as_str) {
                    Some("EventID") => event_id = value.trim().parse::<i64>().ok(),
                    Some("EventRecordID") => record_id = value.trim().parse::<i64>().ok(),
                    Some("Computer") => computer = value.trim().to_owned(),
                    Some("Channel") => channel = value.trim().to_owned(),
                    Some("Level") => level = value.trim().parse::<u8>().ok(),
                    Some("Data") => append_data_value(&mut fields, data_name.as_deref(), &value),
                    _ => {}
                }
            }
            Ok(Event::CData(text)) => {
                let value = text.decode()?.into_owned();
                if stack.last().map(String::as_str) == Some("Data") {
                    append_data_value(&mut fields, data_name.as_deref(), &value);
                }
            }
            Ok(Event::GeneralRef(reference)) => {
                if stack.last().map(String::as_str) == Some("Data") {
                    let entity_name = reference.decode()?;
                    let entity_reference = format!("&{entity_name};");
                    let expanded = quick_xml::escape::unescape(&entity_reference)?.into_owned();

                    append_data_value(&mut fields, data_name.as_deref(), &expanded);
                }
            }
            Ok(Event::End(element)) => {
                let tag = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if tag == "Data" {
                    if let Some(name) = data_name.take() {
                        let value = fields.entry(name).or_default();
                        *value = value.trim().to_owned();
                    }
                }
                let _ = stack.pop();
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(anyhow!("invalid Windows event XML: {error}")),
        }
    }

    let event_id = event_id.ok_or_else(|| anyhow!("event XML did not include EventID"))?;
    let record_id = record_id.ok_or_else(|| anyhow!("event XML did not include EventRecordID"))?;
    let timestamp_utc = timestamp
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc).to_rfc3339())
        .unwrap_or_else(|| Utc::now().to_rfc3339());

    let username =
        find_field(&fields, &["TargetUserName", "SubjectUserName", "User", "AccountName"]);
    let source_ip =
        find_field(&fields, &["IpAddress", "SourceAddress", "ClientAddress", "SourceIp"])
            .filter(|value| value != "-" && !value.is_empty());
    let process_name = find_field(&fields, &["NewProcessName", "Image", "ProcessName"]);
    let command_line = find_field(&fields, &["CommandLine", "ProcessCommandLine"]);
    let (category, summary) = classify_event(event_id, &provider);

    Ok(NormalizedEvent {
        timestamp_utc,
        channel,
        record_id,
        event_id,
        provider,
        computer,
        severity: severity_from_level(level, event_id),
        category: category.to_owned(),
        summary: summary.to_owned(),
        username,
        source_ip,
        process_name,
        command_line,
        fields,
        raw_xml: xml.to_owned(),
    })
}

fn inspect_attributes(
    element: &BytesStart<'_>,
    decoder: quick_xml::encoding::Decoder,
    provider: &mut String,
    timestamp: &mut Option<String>,
    data_name: &mut Option<String>,
) {
    let tag = String::from_utf8_lossy(element.name().as_ref()).into_owned();
    for attribute in element.attributes().flatten() {
        let key = String::from_utf8_lossy(attribute.key.as_ref());
        let value = match attribute.decode_and_unescape_value(decoder) {
            Ok(value) => value.into_owned(),
            Err(_) => continue,
        };
        match (tag.as_str(), key.as_ref()) {
            ("Provider", "Name") => *provider = value,
            ("TimeCreated", "SystemTime") => *timestamp = Some(value),
            ("Data", "Name") => *data_name = Some(value),
            _ => {}
        }
    }
}

fn append_data_value(fields: &mut BTreeMap<String, String>, name: Option<&str>, value: &str) {
    if let Some(name) = name {
        fields.entry(name.to_owned()).or_default().push_str(value);
    }
}

fn find_field(fields: &BTreeMap<String, String>, candidates: &[&str]) -> Option<String> {
    candidates.iter().find_map(|candidate| {
        fields
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(candidate))
            .map(|(_, value)| value.clone())
    })
}

fn severity_from_level(level: Option<u8>, event_id: i64) -> String {
    match event_id {
        4625 | 7045 => "warning",
        1102 => "critical",
        _ => match level {
            Some(0) | Some(4) => "info",
            Some(1) => "critical",
            Some(2) => "error",
            Some(3) => "warning",
            Some(5) => "verbose",
            _ => "unknown",
        },
    }
    .to_owned()
}

fn classify_event(event_id: i64, provider: &str) -> (&'static str, &'static str) {
    match event_id {
        4624 => ("authentication", "Successful logon"),
        4625 => ("authentication", "Failed logon"),
        4648 => ("authentication", "Logon attempted with explicit credentials"),
        4688 => ("process", "Process created"),
        4720 => ("identity", "User account created"),
        4722 => ("identity", "User account enabled"),
        4725 => ("identity", "User account disabled"),
        4726 => ("identity", "User account deleted"),
        7045 => ("service", "Windows service installed"),
        1102 => ("audit", "Security audit log cleared"),
        4719 => ("audit", "System audit policy changed"),
        1 if provider.to_ascii_lowercase().contains("sysmon") => {
            ("process", "Sysmon process creation")
        }
        3 if provider.to_ascii_lowercase().contains("sysmon") => {
            ("network", "Sysmon network connection")
        }
        11 if provider.to_ascii_lowercase().contains("sysmon") => ("file", "Sysmon file created"),
        _ => ("system", "Windows event"),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_windows_event_xml;

    #[test]
    fn parses_security_logon_event() {
        let xml = r#"<Event><System><Provider Name="Microsoft-Windows-Security-Auditing"/><EventID>4625</EventID><Level>0</Level><TimeCreated SystemTime="2026-01-01T10:00:00.000Z"/><EventRecordID>123</EventRecordID><Channel>Security</Channel><Computer>HOST01</Computer></System><EventData><Data Name="TargetUserName">alice</Data><Data Name="IpAddress">10.0.0.9</Data></EventData></Event>"#;
        let event = parse_windows_event_xml(xml, "Security").expect("XML should parse");
        assert_eq!(event.event_id, 4625);
        assert_eq!(event.record_id, 123);
        assert_eq!(event.username.as_deref(), Some("alice"));
        assert_eq!(event.source_ip.as_deref(), Some("10.0.0.9"));
        assert_eq!(event.category, "authentication");
    }

    #[test]
    fn preserves_xml_entity_references_in_field_values() {
        let xml = r#"<Event><System><EventID>1</EventID><EventRecordID>2</EventRecordID></System><EventData><Data Name="CommandLine">write A &amp; B</Data></EventData></Event>"#;
        let event = parse_windows_event_xml(xml, "Application").expect("XML should parse");
        assert_eq!(event.fields.get("CommandLine").map(String::as_str), Some("write A & B"));
    }
}
