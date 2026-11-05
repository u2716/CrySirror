// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Published timezone rules, sampled on a cancellable worker. Unlike sampling
//! transitions around today, the TZif footer states the recurring rule itself
//! (RFC9636 §3.3). No date is executed or guessed to construct this profile.
#[cfg(unix)]
use anyhow::Context;
use crysirror_core::{
    Error, Result,
    service::{
        CivilDaylightRules, CivilTimeZoneProfile, CivilTimeZoneQuery, CivilTimeZoneState,
        CivilTransition, CivilTransitionDay,
    },
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};

struct Query {
    client: ClientId,
    task: Option<Task<CivilTimeZoneProfile>>,
    state: CivilTimeZoneState,
}
impl CivilTimeZoneQuery for Query {
    fn client(&self) -> ClientId {
        self.client
    }
    fn poll(&mut self) -> Result<CivilTimeZoneState> {
        if let Some(task) = &mut self.task {
            match task.try_take() {
                Ok(None) => {}
                Ok(Some(profile)) => {
                    self.state = CivilTimeZoneState::Ready(profile);
                    self.task = None;
                }
                Err(error) => {
                    self.state = CivilTimeZoneState::Failed(error.to_string());
                    self.task = None;
                }
            }
        }
        Ok(self.state.clone())
    }
}
pub(crate) fn begin(pool: &TaskPool, client: ClientId) -> Result<Box<dyn CivilTimeZoneQuery>> {
    let task = pool
        .spawn(move |cancel| {
            cancel.check()?;
            let mut profile = platform_profile()?;
            profile.environment_tz = match std::env::var("TZ") {
                Ok(value) => (!value.is_empty()).then_some(value),
                Err(std::env::VarError::NotPresent) => None,
                Err(std::env::VarError::NotUnicode(_)) => {
                    anyhow::bail!("process TZ is not Unicode")
                }
            };
            cancel.check()?;
            Ok(profile)
        })
        .map_err(|error| Error::Io(format!("civil timezone admission: {error:#}")))?;
    Ok(Box::new(Query {
        client,
        task: Some(task),
        state: CivilTimeZoneState::Pending,
    }))
}

#[cfg(unix)]
fn platform_profile() -> anyhow::Result<CivilTimeZoneProfile> {
    use std::io::Read;
    // This is host configuration, never a client-provided game resource path.
    // Bound it before parsing both TZif data blocks or allocating a profile.
    let mut bytes = Vec::new();
    std::fs::File::open("/etc/localtime")?
        .take(1_048_577)
        .read_to_end(&mut bytes)?;
    anyhow::ensure!(
        bytes.len() <= 1_048_576,
        "host timezone metadata exceeds limit"
    );
    let first = block_end(&bytes, 0, 4)?;
    let version = *bytes
        .get(4)
        .ok_or_else(|| anyhow::anyhow!("missing TZif version"))?;
    if version == 0 {
        return fixed_profile(&bytes, 0, 4)?
            .ok_or_else(|| anyhow::anyhow!("host TZif has no published recurring policy"));
    }
    anyhow::ensure!(
        matches!(version, b'2' | b'3' | b'4'),
        "unsupported host TZif version"
    );
    let end = block_end(&bytes, first, 8)?;
    let footer = bytes
        .get(end..)
        .ok_or_else(|| anyhow::anyhow!("missing TZif footer"))?;
    anyhow::ensure!(footer.first() == Some(&b'\n'), "invalid TZif footer prefix");
    let tail = &footer[1..];
    let length = tail
        .iter()
        .position(|byte| *byte == b'\n')
        .ok_or_else(|| anyhow::anyhow!("unterminated TZif footer"))?;
    if length == 0 {
        return fixed_profile(&bytes, first, 8)?
            .ok_or_else(|| anyhow::anyhow!("host TZif does not state a recurring policy"));
    }
    anyhow::ensure!(length <= 4096, "host TZif recurring policy exceeds limit");
    let text = std::str::from_utf8(&tail[..length])?;
    parse_posix(text)
}

#[cfg(unix)]
fn fixed_profile(
    bytes: &[u8],
    offset: usize,
    width: usize,
) -> anyhow::Result<Option<CivilTimeZoneProfile>> {
    let header = &bytes[offset..offset + 44];
    let count = |at: usize| u32::from_be_bytes(header[at..at + 4].try_into().unwrap()) as usize;
    let start = offset + 44 + count(32) * (width + 1);
    let mut bias = None;
    for n in 0..count(36) {
        let at = start + n * 6;
        let east = i32::from_be_bytes(bytes[at..at + 4].try_into()?);
        if bytes[at + 4] != 0 || bias.is_some_and(|old| old != east) {
            return Ok(None);
        }
        bias = Some(east);
    }
    let standard = bias
        .context("host TZif has no fixed offset")?
        .checked_neg()
        .context("host TZif offset overflow")?;
    Ok(Some(CivilTimeZoneProfile {
        standard_bias_seconds: standard,
        daylight_bias_seconds: 0,
        daylight: None,
        environment_tz: None,
    }))
}
#[cfg(not(unix))]
fn platform_profile() -> anyhow::Result<CivilTimeZoneProfile> {
    anyhow::bail!("published civil timezone policy is unavailable on this platform")
}

#[cfg(unix)]
fn block_end(bytes: &[u8], offset: usize, width: usize) -> anyhow::Result<usize> {
    let header = bytes
        .get(
            offset
                ..offset
                    .checked_add(44)
                    .ok_or_else(|| anyhow::anyhow!("TZif offset overflow"))?,
        )
        .ok_or_else(|| anyhow::anyhow!("truncated TZif header"))?;
    anyhow::ensure!(&header[..4] == b"TZif", "invalid host timezone magic");
    let count = |start: usize| -> anyhow::Result<usize> {
        Ok(usize::try_from(u32::from_be_bytes(
            header[start..start + 4].try_into()?,
        ))?)
    };
    let ut = count(20)?;
    let standard = count(24)?;
    let leap = count(28)?;
    let times = count(32)?;
    let types = count(36)?;
    let chars = count(40)?;
    anyhow::ensure!(
        (1..=256).contains(&types),
        "invalid TZif local time type count"
    );
    let mut length = 44usize;
    for (count, size) in [
        (times, width),
        (times, 1),
        (types, 6),
        (chars, 1),
        (leap, width + 4),
        (standard, 1),
        (ut, 1),
    ] {
        length = length
            .checked_add(
                count
                    .checked_mul(size)
                    .ok_or_else(|| anyhow::anyhow!("TZif count overflow"))?,
            )
            .ok_or_else(|| anyhow::anyhow!("TZif block overflow"))?;
    }
    let end = offset
        .checked_add(length)
        .ok_or_else(|| anyhow::anyhow!("TZif offset overflow"))?;
    anyhow::ensure!(end <= bytes.len(), "truncated TZif block");
    Ok(end)
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }
    fn eat(&mut self, value: u8) -> bool {
        if self.peek() == Some(value) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn number(&mut self) -> anyhow::Result<i32> {
        let start = self.at;
        let mut value = 0i32;
        while let Some(byte @ b'0'..=b'9') = self.peek() {
            value = value
                .checked_mul(10)
                .and_then(|value| value.checked_add(i32::from(byte - b'0')))
                .ok_or_else(|| anyhow::anyhow!("timezone number overflow"))?;
            self.at += 1;
        }
        anyhow::ensure!(self.at != start, "missing timezone number");
        Ok(value)
    }
    fn name(&mut self) -> anyhow::Result<()> {
        let quoted = self.eat(b'<');
        let start = self.at;
        while let Some(byte) = self.peek() {
            if if quoted {
                byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-')
            } else {
                byte.is_ascii_alphabetic()
            } {
                self.at += 1;
            } else {
                break;
            }
        }
        anyhow::ensure!(
            self.at - start >= 3 && (!quoted || self.eat(b'>')),
            "invalid timezone abbreviation"
        );
        Ok(())
    }
    fn seconds(&mut self) -> anyhow::Result<i32> {
        let sign = if self.eat(b'-') {
            -1
        } else {
            self.eat(b'+');
            1
        };
        let hour = self.number()?;
        let minute = if self.eat(b':') { self.number()? } else { 0 };
        let second = if self.eat(b':') { self.number()? } else { 0 };
        anyhow::ensure!(
            hour <= 167 && minute <= 59 && second <= 59,
            "timezone clock outside TZif range"
        );
        Ok(sign * (hour * 3600 + minute * 60 + second))
    }
    fn transition(&mut self) -> anyhow::Result<CivilTransition> {
        let day = if self.eat(b'M') {
            let month = self.number()?;
            anyhow::ensure!(self.eat(b'.'), "missing month/week separator");
            let week = self.number()?;
            anyhow::ensure!(self.eat(b'.'), "missing week/day separator");
            let weekday = self.number()?;
            anyhow::ensure!(
                (1..=12).contains(&month) && (1..=5).contains(&week) && weekday <= 6,
                "invalid recurring timezone day"
            );
            CivilTransitionDay::MonthWeekday {
                month: month as u8,
                week: week as u8,
                weekday: weekday as u8,
            }
        } else if self.eat(b'J') {
            let day = self.number()?;
            anyhow::ensure!((1..=365).contains(&day), "invalid Julian timezone day");
            CivilTransitionDay::JulianNoLeap(day as u16)
        } else {
            let day = self.number()?;
            anyhow::ensure!(day <= 365, "invalid ordinal timezone day");
            CivilTransitionDay::Ordinal(day as u16)
        };
        let seconds = if self.eat(b'/') {
            self.seconds()?
        } else {
            7200
        };
        Ok(CivilTransition { day, seconds })
    }
}
fn parse_posix(text: &str) -> anyhow::Result<CivilTimeZoneProfile> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        at: 0,
    };
    parser.name()?;
    let standard = parser.seconds()?;
    let mut profile = CivilTimeZoneProfile {
        standard_bias_seconds: standard,
        daylight_bias_seconds: 0,
        daylight: None,
        environment_tz: None,
    };
    if parser.peek().is_none() {
        return Ok(profile);
    }
    parser.name()?;
    let daylight = if matches!(parser.peek(), Some(b'+' | b'-' | b'0'..=b'9')) {
        parser.seconds()?
    } else {
        standard - 3600
    };
    anyhow::ensure!(
        parser.eat(b','),
        "timezone daylight policy has no published start rule"
    );
    let start = parser.transition()?;
    anyhow::ensure!(
        parser.eat(b','),
        "timezone daylight policy has no published end rule"
    );
    let end = parser.transition()?;
    anyhow::ensure!(parser.peek().is_none(), "unsupported timezone rule suffix");
    profile.daylight_bias_seconds = daylight - standard;
    profile.daylight = Some(CivilDaylightRules { start, end });
    Ok(profile)
}
