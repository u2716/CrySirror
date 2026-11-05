// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! OS/timezone reads happen on a bounded shared worker, never a VM/owner tick.
use chrono::{Datelike, TimeZone, Timelike};
use crysirror_core::{
    Error, Result,
    service::{CivilTime, CivilTimeQuery, CivilTimeState},
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};
#[path = "civil_timezone.rs"]
mod timezone;
pub(crate) use timezone::begin as begin_timezone;
struct Query {
    client: ClientId,
    task: Option<Task<CivilTime>>,
    state: CivilTimeState,
}
impl CivilTimeQuery for Query {
    fn client(&self) -> ClientId {
        self.client
    }
    fn poll(&mut self) -> Result<CivilTimeState> {
        if let Some(task) = &mut self.task {
            match task.try_take() {
                Ok(None) => {}
                Ok(Some(value)) => {
                    self.state = CivilTimeState::Ready(value);
                    self.task = None;
                }
                Err(e) => {
                    self.state = CivilTimeState::Failed(e.to_string());
                    self.task = None;
                }
            }
        }
        Ok(self.state.clone())
    }
}
pub(crate) fn begin(pool: &TaskPool, client: ClientId) -> Result<Box<dyn CivilTimeQuery>> {
    let task = pool
        .spawn(move |cancel| {
            cancel.check()?;
            let t = chrono::Local::now();
            let result = CivilTime {
                client,
                unix_ms: t.timestamp_millis(),
                utc_offset_seconds: t.offset().local_minus_utc(),
                year: t.year(),
                month: t.month() as u8,
                day: t.day() as u8,
                hour: t.hour() as u8,
                minute: t.minute() as u8,
                second: t.second() as u8,
                millisecond: t.timestamp_subsec_millis() as u16,
            };
            cancel.check()?;
            Ok(result)
        })
        .map_err(|e| Error::Io(format!("civil clock admission: {e:#}")))?;
    Ok(Box::new(Query {
        client,
        task: Some(task),
        state: CivilTimeState::Pending,
    }))
}

/// A requested local date needs the timezone rule for that date, not the
/// current wall-clock offset. Date discontinuities are returned to the caller
/// rather than silently adopting one host CRT's normalization convention.
pub(crate) fn begin_date(
    pool: &TaskPool,
    client: ClientId,
    year: i32,
    month: u8,
    day: u8,
) -> Result<Box<dyn CivilTimeQuery>> {
    let task = pool
        .spawn(move |cancel| {
            cancel.check()?;
            let date = chrono::NaiveDate::from_ymd_opt(year, u32::from(month), u32::from(day))
                .and_then(|date| date.and_hms_opt(0, 0, 0))
                .ok_or_else(|| anyhow::anyhow!("invalid normalized local civil date"))?;
            let t = match chrono::Local.from_local_datetime(&date) {
                chrono::LocalResult::Single(time) => time,
                chrono::LocalResult::Ambiguous(_, _) => {
                    anyhow::bail!("ambiguous local midnight requires an explicit transition policy")
                }
                chrono::LocalResult::None => anyhow::bail!(
                    "nonexistent local midnight requires an explicit transition policy"
                ),
            };
            let result = CivilTime {
                client,
                unix_ms: t.timestamp_millis(),
                utc_offset_seconds: t.offset().local_minus_utc(),
                year: t.year(),
                month: t.month() as u8,
                day: t.day() as u8,
                hour: t.hour() as u8,
                minute: t.minute() as u8,
                second: t.second() as u8,
                millisecond: t.timestamp_subsec_millis() as u16,
            };
            cancel.check()?;
            Ok(result)
        })
        .map_err(|e| Error::Io(format!("local civil date admission: {e:#}")))?;
    Ok(Box::new(Query {
        client,
        task: Some(task),
        state: CivilTimeState::Pending,
    }))
}
