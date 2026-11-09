use anyhow::{anyhow, Context, Result};

use crate::{
    db::Database,
    models::NormalizedEvent,
    normalizer::{parse_windows_event_xml, rules},
};

pub fn collect_channel(database: &Database, channel: &str) -> Result<usize> {
    let cursor = database.last_record_id(channel)?;
    let events = platform::fetch_events(channel, cursor, 250)?;
    let mut inserted = 0;

    for event in events {
        if database.insert_event(&event)? {
            rules::evaluate(database, &event)?;
            inserted += 1;
        }
    }

    Ok(inserted)
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::{ffi::c_void, io, os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::{
        Foundation::{GetLastError, ERROR_NO_MORE_ITEMS},
        System::EventLog::{
            EvtClose, EvtNext, EvtQuery, EvtQueryChannelPath, EvtQueryForwardDirection,
            EvtQueryReverseDirection, EvtRender, EvtRenderEventXml,
        },
    };

    struct EventHandle(isize);
    impl Drop for EventHandle {
        fn drop(&mut self) {
            if self.0 != 0 {
                unsafe {
                    EvtClose(self.0);
                }
            }
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        std::ffi::OsStr::new(value).encode_wide().chain(Some(0)).collect()
    }

    pub fn fetch_events(
        channel: &str,
        cursor: Option<i64>,
        limit: usize,
    ) -> Result<Vec<NormalizedEvent>> {
        let query = match cursor {
            Some(record_id) => format!("*[System[EventRecordID > {record_id}]]"),
            None => "*".to_owned(),
        };
        let channel_wide = wide(channel);
        let query_wide = wide(&query);
        let direction =
            if cursor.is_some() { EvtQueryForwardDirection } else { EvtQueryReverseDirection };
        let query_handle = unsafe {
            EvtQuery(0, channel_wide.as_ptr(), query_wide.as_ptr(), EvtQueryChannelPath | direction)
        };
        if query_handle == 0 {
            let error = unsafe { GetLastError() };
            return Err(io::Error::from_raw_os_error(error as i32))
                .with_context(|| format!("EvtQuery failed for channel '{channel}'"));
        }
        let query_handle = EventHandle(query_handle);

        let mut collected = Vec::new();
        let mut done = false;
        while !done && collected.len() < limit {
            let mut handles = [0_isize; 32];
            let mut returned = 0_u32;
            let success = unsafe {
                EvtNext(
                    query_handle.0,
                    handles.len() as u32,
                    handles.as_mut_ptr(),
                    0,
                    0,
                    &mut returned,
                )
            };

            if success == 0 {
                let error = unsafe { GetLastError() };
                if error == ERROR_NO_MORE_ITEMS {
                    break;
                }
                return Err(io::Error::from_raw_os_error(error as i32))
                    .with_context(|| format!("EvtNext failed for channel '{channel}'"));
            }

            if returned == 0 {
                break;
            }

            let event_handles = handles
                .iter()
                .take(returned as usize)
                .copied()
                .map(EventHandle)
                .collect::<Vec<_>>();
            for event_handle in event_handles {
                if collected.len() < limit {
                    let xml = render_event(event_handle.0)
                        .with_context(|| format!("could not render event from '{channel}'"))?;
                    let event = parse_windows_event_xml(&xml, channel)?;
                    collected.push(event);
                }
                // EventHandle closes this handle; unprocessed handles are also closed when
                // the owned iterator is dropped after the limit is reached.
            }
            done = collected.len() >= limit;
        }

        // First startup query is reverse-ordered so we get the newest N records efficiently.
        // Store in chronological order to make detection evaluation predictable.
        if cursor.is_none() {
            collected.reverse();
        }
        Ok(collected)
    }

    fn render_event(event_handle: isize) -> Result<String> {
        let mut required_bytes = 0_u32;
        let mut property_count = 0_u32;
        let first_call = unsafe {
            EvtRender(
                0,
                event_handle,
                EvtRenderEventXml,
                0,
                ptr::null_mut(),
                &mut required_bytes,
                &mut property_count,
            )
        };
        if first_call != 0 || required_bytes == 0 {
            return Err(anyhow!("EvtRender did not return a required buffer size"));
        }

        let mut buffer = vec![0_u16; (required_bytes as usize / 2) + 1];
        let success = unsafe {
            EvtRender(
                0,
                event_handle,
                EvtRenderEventXml,
                required_bytes,
                buffer.as_mut_ptr().cast::<c_void>(),
                &mut required_bytes,
                &mut property_count,
            )
        };
        if success == 0 {
            let error = unsafe { GetLastError() };
            return Err(io::Error::from_raw_os_error(error as i32)).context("EvtRender failed");
        }
        let length = (required_bytes as usize / 2).min(buffer.len());
        Ok(String::from_utf16_lossy(&buffer[..length]).trim_end_matches('\0').to_owned())
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;

    pub fn fetch_events(
        _channel: &str,
        _cursor: Option<i64>,
        _limit: usize,
    ) -> Result<Vec<NormalizedEvent>> {
        Err(anyhow!("Windows Event Log collection is available only when running on Windows"))
    }
}
