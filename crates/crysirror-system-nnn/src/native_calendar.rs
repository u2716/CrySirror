// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original script date/holiday binding policy. Gregorian arithmetic belongs
//! to the shared Core calendar; holiday tuples are native configuration data.
use crate::{
    config::{self, Names},
    expression::{VariableProfile, Variables},
};
use anyhow::{ensure, Context, Result};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(crate) struct NativeCalendar {
    date: Option<u16>,
    year: Option<u16>,
    holidays: Option<Arc<[[i32; 3]]>>,
    use_holidays: bool,
}
impl NativeCalendar {
    pub fn configured(names: &Names, setup: &Names, profile: VariableProfile) -> Result<Self> {
        let use_year = profile == VariableProfile::Legacy1000
            || setup.integer(b"notUseYearVar")?.unwrap_or(0) == 0;
        Ok(Self {
            date: names.index(b"date"),
            year: use_year.then(|| names.index(b"year")).flatten(),
            // The original key really is spelled Saijistu, not Saijitsu.
            use_holidays: setup.integer(b"useSaijistu")?.unwrap_or(0) != 0,
            holidays: None,
        })
    }
    pub fn needs_holidays(&self) -> bool {
        self.use_holidays && self.holidays.is_none()
    }
    pub fn prepare_holidays(&mut self, table: &Names) -> Result<()> {
        ensure!(
            self.needs_holidays(),
            "NNN holiday table not requested or already prepared"
        );
        // CNameList count/3 deliberately drops trailing one or two tokens.
        let dates = table
            .tokens
            .chunks_exact(3)
            .map(|row| {
                Ok([
                    config::integer(&row[0])?,
                    config::integer(&row[1])?,
                    config::integer(&row[2])?,
                ])
            })
            .collect::<Result<Vec<_>>>()?;
        self.holidays = Some(dates.into());
        Ok(())
    }
    pub fn value(&self, name: &[u8], variables: &Variables) -> Result<i32> {
        ensure!(
            matches!(name, b"week" | b"holiday"),
            "NNN unknown calendar binding"
        );
        // GetVarData(-1) follows the original immediate-bit path and yields
        // 32767. Invalid resulting dates fail at the checked adapter boundary.
        let daymonth = variables.read_stored(self.date.unwrap_or(u16::MAX));
        let year = self.year.map_or(2000, |id| variables.read_stored(id));
        let month = u8::try_from(daymonth / 100).context("NNN script month range")?;
        let day = u8::try_from(daymonth % 100).context("NNN script day range")?;
        let week = crysirror_core::service::civil_weekday(year, month, day)
            .context("NNN script date outside validated Gregorian profile")?;
        if name == b"week" {
            return Ok(i32::from(week));
        }
        if week == 0 || week == 6 {
            return Ok(1);
        }
        if !self.use_holidays {
            return Ok(0);
        }
        let holidays = self
            .holidays
            .as_ref()
            .context("NNN native holiday table not prepared")?;
        Ok(i32::from(holidays.contains(&[
            year,
            i32::from(month),
            i32::from(day),
        ])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn vars(setup: &[u8], profile: VariableProfile) -> Variables {
        config::configured_variables(
            profile,
            &Names::parse(b"week holiday date year", false).unwrap(),
            &Names::parse(setup, false).unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn original_week_and_holiday_read_current_script_bank_and_shared_gregorian_rules() {
        let mut variables = vars(b"", VariableProfile::Legacy1000);
        // Native constructor date defaults to Jan1,2000 (Saturday).
        assert_eq!(variables.read_calculated(0).unwrap(), 6);
        assert_eq!(variables.read_calculated(1).unwrap(), 1);
        variables.write_stored(2, 103);
        assert_eq!(variables.read_calculated(0).unwrap(), 1);
        assert_eq!(variables.read_calculated(1).unwrap(), 0);
        variables.write_stored(3, 2026);
        variables.write_stored(2, 1101);
        assert_eq!(variables.read_calculated(0).unwrap(), 0);
        assert_eq!(variables.read_calculated(1).unwrap(), 1);
        variables.write_stored(3, 1900);
        variables.write_stored(2, 229);
        assert!(variables.read_calculated(0).is_err());
        variables.write_stored(3, 2000);
        assert_eq!(variables.read_calculated(0).unwrap(), 2);
        assert_eq!(variables.read_calculated(1).unwrap(), 0);
    }
    #[test]
    fn original_holiday_tuples_are_explicit_year_matches_with_trailing_token_policy() {
        let mut variables = vars(b"useSaijistu 1", VariableProfile::Legacy1000);
        variables.write_stored(2, 103); // Monday, requires the real resource.
        assert!(variables
            .read_calculated(1)
            .unwrap_err()
            .to_string()
            .contains("not prepared"));
        variables
            .prepare_native_holidays(
                &Names::parse(b"2000 1 3,2001 1 4,trailing ignored", false).unwrap(),
            )
            .unwrap();
        assert_eq!(variables.read_calculated(1).unwrap(), 1);
        variables.write_stored(2, 104);
        assert_eq!(variables.read_calculated(1).unwrap(), 0);
        variables.write_stored(3, 2001);
        assert_eq!(variables.read_calculated(1).unwrap(), 1);
        let clone = variables.clone();
        variables.write_stored(3, 2002);
        assert_eq!(variables.read_calculated(1).unwrap(), 0);
        assert_eq!(clone.read_calculated(1).unwrap(), 1);
        assert!(variables
            .prepare_native_holidays(&Names::parse(b"", false).unwrap())
            .is_err());
    }
    #[test]
    fn absent_holiday_file_is_empty_and_expanded_year_disable_does_not_mutate_storage() {
        let mut variables = vars(
            b"useSaijistu 1 notUseYearVar 1",
            VariableProfile::Expanded2200,
        );
        variables
            .prepare_native_holidays(&Names::parse(b"", false).unwrap())
            .unwrap();
        variables.write_stored(3, 2026);
        variables.write_stored(2, 103);
        assert_eq!(variables.read_calculated(0).unwrap(), 1); // fallback2000, not2026
        assert_eq!(variables.read_calculated(1).unwrap(), 0);
        assert_eq!(variables.read_stored(3), 2026);
        let mut legacy = vars(b"notUseYearVar 1", VariableProfile::Legacy1000);
        legacy.write_stored(3, 2026);
        legacy.write_stored(2, 103);
        assert_eq!(legacy.read_calculated(0).unwrap(), 6); // actual legacy always boundyear
        legacy.write_stored(2, -1);
        assert!(legacy.read_calculated(0).is_err());
    }
}
