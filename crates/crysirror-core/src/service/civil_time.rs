// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! A sampled host local civil clock, separate from the virtual engine clock.
use crate::{state::ClientId, Result};
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
impl CivilTime {
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

#[cfg(test)]
mod tests {
    use super::*;
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
