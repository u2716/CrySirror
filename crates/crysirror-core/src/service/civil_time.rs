// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! A sampled host local civil clock, separate from the virtual engine clock.
use crate::{Result, state::ClientId};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivilTime {
    pub client: ClientId,
    pub unix_ms: i64,
    pub utc_offset_seconds: i32,
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub millisecond: u16,
}
/// Proleptic Gregorian weekday, Sunday0 through Saturday6. Invalid dates and
/// years before1 are unavailable; this pure query reads no host/simulation time.
pub fn civil_weekday(year: i32, month: u8, day: u8) -> Option<u8> {
    if year < 1 || !(1..=12).contains(&month) {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let last = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if day == 0 || day > last {
        return None;
    }
    // January/February belong to the preceding March-based leap cycle.
    // i64 keeps the arithmetic defined for every positive i32 year.
    let year = i64::from(year) - i64::from(month < 3);
    let month_offset = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4][usize::from(month - 1)];
    Some(((year + year / 4 - year / 100 + year / 400 + month_offset + i64::from(day)) % 7) as u8)
}
/// Integer Zeller expression with signed32-bit wrapping and truncating
/// division. Unlike `civil_weekday`, this accepts arithmetic month/day fields
/// without calendar normalization and retains a negative signed remainder.
/// This pure compatibility primitive samples neither host nor virtual time.
pub fn integer_weekday_wrapping(year: i32, month: i32, day: i32) -> i32 {
    let (year, month) = if month <= 2 {
        (year.wrapping_sub(1), month.wrapping_add(12))
    } else {
        (year, month)
    };
    day.wrapping_add(year / 400)
        .wrapping_add(year)
        .wrapping_add(year / 4)
        .wrapping_sub(year / 100)
        .wrapping_add(month.wrapping_mul(13).wrapping_add(8) / 5)
        % 7
}
/// Normalize signed Gregorian month/day fields, as a civil calendar operation.
/// Month1/day1 denote January1; month0 and day0 refer to the preceding month
/// and day. This does not apply a timezone, DST, or a host CRT timestamp range.
pub fn normalize_civil_date(year: i32, month: i32, day: i32) -> Option<(i32, u8, u8)> {
    let months = i64::from(year) * 12 + i64::from(month) - 1;
    let year = months.div_euclid(12);
    let month = months.rem_euclid(12) + 1;
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let ordinal =
        era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * mp + 2) / 5 + i64::from(day) - 1;
    let era = ordinal.div_euclid(146097);
    let doe = ordinal - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    Some((i32::try_from(year).ok()?, month as u8, day as u8))
}
impl CivilTime {
    /// Convert an instant using an explicit fixed offset sampled by a host.
    /// This performs no clock/timezone lookup and does not infer historical
    /// daylight-saving rules from the supplied present-day offset.
    pub fn from_unix_ms(client: ClientId, unix_ms: i64, utc_offset_seconds: i32) -> Option<Self> {
        if utc_offset_seconds.unsigned_abs() > 86400 {
            return None;
        }
        let local = unix_ms
            .div_euclid(1000)
            .checked_add(i64::from(utc_offset_seconds))?;
        let (year, month, day) =
            normalize_civil_date(1970, 1, i32::try_from(local.div_euclid(86400) + 1).ok()?)?;
        let second = local.rem_euclid(86400);
        Some(Self {
            client,
            unix_ms,
            utc_offset_seconds,
            year,
            month,
            day,
            hour: (second / 3600) as u8,
            minute: ((second / 60) % 60) as u8,
            second: (second % 60) as u8,
            millisecond: unix_ms.rem_euclid(1000) as u16,
        })
    }
    /// Weekday of this already sampled local civil date. It performs no new
    /// clock sample and does not infer a value from unavailable/invalid fields.
    pub fn weekday(&self) -> Option<u8> {
        civil_weekday(self.year, self.month, self.day)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CivilTimeState {
    Pending,
    Ready(CivilTime),
    Failed(alloc::string::String),
}
/// An authorized worker dependency. Dropping it cancels unfinished sampling.
pub trait CivilTimeQuery: Send {
    fn client(&self) -> ClientId;
    fn poll(&mut self) -> Result<CivilTimeState>;
}

/// A recurring calendar day in a timezone's published annual rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CivilTransitionDay {
    /// Month1..12, week1..5 (5 means last), weekday Sunday0..Saturday6.
    MonthWeekday { month: u8, week: u8, weekday: u8 },
    /// Day1..365, omitting February29 from the count.
    JulianNoLeap(u16),
    /// Day0..365, counting February29 when present.
    Ordinal(u16),
}
/// Annual transition wall time; seconds may extend outside the named day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivilTransition {
    /// Calendar day before adding the transition's clock displacement.
    pub day: CivilTransitionDay,
    /// Seconds from midnight, in the wall clock immediately before transition.
    pub seconds: i32,
}
/// Published annual daylight-saving rules, independent of an instant sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivilDaylightRules {
    /// Transition into daylight time, declared on the standard clock.
    pub start: CivilTransition,
    /// Transition into standard time, declared on the daylight clock.
    pub end: CivilTransition,
}
/// Current host timezone policy captured once by an authorized worker.
/// Biases are seconds west of UTC, and daylight bias is added to standard
/// bias. Rules describe current recurring policy, not historical IANA eras.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CivilTimeZoneProfile {
    /// Standard-time seconds west of UTC.
    pub standard_bias_seconds: i32,
    /// Additional seconds west of UTC during daylight time, often -3600.
    pub daylight_bias_seconds: i32,
    /// Explicit recurring policy; None means daylight time is disabled.
    pub daylight: Option<CivilDaylightRules>,
    /// Uninterpreted process TZ override, for callers with a specific CRT's
    /// environment grammar. It does not replace the published host rules.
    pub environment_tz: Option<alloc::string::String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CivilTimeZoneState {
    Pending,
    Ready(CivilTimeZoneProfile),
    Failed(alloc::string::String),
}
/// Client-owned asynchronous timezone-policy dependency.
pub trait CivilTimeZoneQuery: Send {
    fn client(&self) -> ClientId;
    fn poll(&mut self) -> Result<CivilTimeZoneState>;
}

fn epoch_day(year: i32, month: u8, day: u8) -> Option<i64> {
    civil_weekday(year, month, day)?;
    let y = i64::from(year) - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(month) + if month > 2 { -3 } else { 9 };
    Some(
        era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * m + 2) / 5 + i64::from(day)
            - 1
            - 719468,
    )
}
fn transition_second(year: i32, rule: CivilTransition) -> Option<i64> {
    let january = epoch_day(year, 1, 1)?;
    let day = match rule.day {
        CivilTransitionDay::MonthWeekday {
            month,
            week,
            weekday,
        } => {
            if !(1..=5).contains(&week) || weekday > 6 {
                return None;
            }
            let first = epoch_day(year, month, 1)?;
            let first_weekday = civil_weekday(year, month, 1)?;
            let mut day = 1 + (weekday + 7 - first_weekday) % 7 + 7 * (week - 1);
            if civil_weekday(year, month, day).is_none() {
                day -= 7;
            }
            epoch_day(year, month, day)?
                .checked_sub(first)?
                .checked_add(first)?
        }
        CivilTransitionDay::JulianNoLeap(day) => {
            if !(1..=365).contains(&day) {
                return None;
            }
            let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
            january + i64::from(day - 1) + i64::from(leap && day >= 60)
        }
        CivilTransitionDay::Ordinal(day) => {
            if day > 365 {
                return None;
            }
            january + i64::from(day)
        }
    };
    day.checked_mul(86400)?.checked_add(i64::from(rule.seconds))
}
impl CivilTimeZoneProfile {
    fn daylight_at_standard(&self, standard_second: i64) -> Option<bool> {
        let Some(rules) = self.daylight else {
            return Some(false);
        };
        let date = normalize_civil_date(
            1970,
            1,
            i32::try_from(standard_second.div_euclid(86400) + 1).ok()?,
        )?;
        let start = transition_second(date.0, rules.start)?;
        // End is declared on the daylight clock; compare on standard clock.
        let end = transition_second(date.0, rules.end)?
            .checked_add(i64::from(self.daylight_bias_seconds))?;
        // The CRT compares ordinal first, then boundary clock. This matters
        // when both transitions land on the same date (466D03/466B0C).
        let day = standard_second.div_euclid(86400);
        let start_day = start.div_euclid(86400);
        let end_day = end.div_euclid(86400);
        Some(if start_day < end_day {
            if day < start_day || day > end_day {
                false
            } else if day == start_day {
                standard_second >= start
            } else if day == end_day {
                standard_second < end
            } else {
                true
            }
        } else if day < end_day || day > start_day {
            true
        } else if day == start_day {
            standard_second >= start
        } else if day == end_day {
            standard_second < end
        } else {
            false
        })
    }
    /// Resolve an instant using cached annual rules and the legacy CRT's
    /// standard-clock DST predicate. The first three UTC days preserve the
    /// CRT's special boundary calculation; this reads no host state.
    pub fn crt_local_time(&self, client: ClientId, unix_seconds: i64) -> Option<CivilTime> {
        let standard = unix_seconds.checked_sub(i64::from(self.standard_bias_seconds))?;
        let probe = if unix_seconds <= 259200 {
            unix_seconds
        } else {
            standard
        };
        let dst = self.daylight_at_standard(probe)?;
        let bias = i64::from(self.standard_bias_seconds)
            + if dst {
                i64::from(self.daylight_bias_seconds)
            } else {
                0
            };
        let local = unix_seconds.checked_sub(bias)?;
        let date = normalize_civil_date(1970, 1, i32::try_from(local.div_euclid(86400) + 1).ok()?)?;
        let second = local.rem_euclid(86400);
        Some(CivilTime {
            client,
            unix_ms: unix_seconds.checked_mul(1000)?,
            utc_offset_seconds: i32::try_from(-bias).ok()?,
            year: date.0,
            month: date.1,
            day: date.2,
            hour: (second / 3600) as u8,
            minute: ((second / 60) % 60) as u8,
            second: (second % 60) as u8,
            millisecond: 0,
        })
    }
    /// Standard-first midnight normalization used by mktime-style callers
    /// with tm_isdst=-1. A fold chooses standard time; gaps follow the actual
    /// two-pass offset calculation instead of choosing a guessed neighbor.
    /// Bounds apply to both localtime probes, in UTC seconds inclusive.
    pub fn resolve_crt_midnight(
        &self,
        client: ClientId,
        year: i32,
        month: u8,
        day: u8,
        utc_bounds: core::ops::RangeInclusive<i64>,
    ) -> Option<CivilTime> {
        let mut candidate = epoch_day(year, month, day)?
            .checked_mul(86400)?
            .checked_add(i64::from(self.standard_bias_seconds))?;
        if !utc_bounds.contains(&candidate) {
            return None;
        }
        let first = self.crt_local_time(client, candidate)?;
        if i64::from(first.utc_offset_seconds) != -i64::from(self.standard_bias_seconds) {
            candidate = candidate.checked_add(i64::from(self.daylight_bias_seconds))?;
            if !utc_bounds.contains(&candidate) {
                return None;
            }
        }
        self.crt_local_time(client, candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_calendar_fields_normalize_across_leap_and_century_boundaries() {
        for (input, expected) in [
            ((2024, 3, 0), (2024, 2, 29)),
            ((1900, 2, 29), (1900, 3, 1)),
            ((2000, 2, 29), (2000, 2, 29)),
            ((2024, 0, 1), (2023, 12, 1)),
            ((2024, 14, -1), (2025, 1, 30)),
            ((0, 1, 1), (0, 1, 1)),
        ] {
            assert_eq!(
                normalize_civil_date(input.0, input.1, input.2),
                Some(expected)
            );
        }
        assert_eq!(normalize_civil_date(i32::MAX, 13, 1), None);
        assert_eq!(normalize_civil_date(i32::MIN, 0, 1), None);
        let (year, month, day) = normalize_civil_date(2024, 1, i32::MAX).unwrap();
        assert!(year > 5_000_000);
        assert!(civil_weekday(year, month, day).is_some());
    }
    #[test]
    fn gregorian_dates_validate_leap_centuries_and_extreme_years() {
        for (date, expected) in [
            ((1, 1, 1), 1),
            ((1970, 1, 1), 4),
            ((2000, 1, 1), 6),
            ((2000, 2, 29), 2),
            ((2024, 2, 29), 4),
            ((2026, 11, 1), 0),
            ((9999, 12, 31), 5),
        ] {
            assert_eq!(civil_weekday(date.0, date.1, date.2), Some(expected));
        }
        for date in [
            (0, 1, 1),
            (-1, 1, 1),
            (2024, 0, 1),
            (2024, 13, 1),
            (2024, 1, 0),
            (1900, 2, 29),
            (2023, 2, 29),
            (2024, 4, 31),
        ] {
            assert_eq!(civil_weekday(date.0, date.1, date.2), None);
        }
        assert_eq!(civil_weekday(i32::MAX, 12, 31), civil_weekday(2047, 12, 31));
    }
    #[test]
    fn successive_days_and_year_boundaries_advance_once() {
        for (year, february) in [
            (1899, 28),
            (1900, 28),
            (1999, 28),
            (2000, 29),
            (2023, 28),
            (2024, 29),
        ] {
            let mut expected = civil_weekday(year, 1, 1).unwrap();
            for (month, days) in [31, february, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                .into_iter()
                .enumerate()
            {
                for day in 1..=days {
                    assert_eq!(civil_weekday(year, month as u8 + 1, day), Some(expected));
                    expected = (expected + 1) % 7;
                }
            }
            assert_eq!(civil_weekday(year + 1, 1, 1), Some(expected));
        }
    }
    #[test]
    fn retained_local_date_query_does_not_replace_it_with_unix_epoch_fields() {
        let mut value = CivilTime {
            client: ClientId(7),
            unix_ms: 0,
            utc_offset_seconds: 0,
            year: 2026,
            month: 11,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            millisecond: 0,
        };
        assert_eq!(value.weekday(), Some(0));
        assert_eq!(value.unix_ms, 0);
        value.day = 0;
        assert_eq!(value.weekday(), None);
    }
}
