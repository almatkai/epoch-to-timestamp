//! Windows recurring timezone transitions, evaluated in the actual calendar year.
use crate::epoch::days_from_civil;

#[derive(Clone, Copy)]
pub(super) struct Rule {
    pub month: i64,
    pub weekday: i64,
    pub occurrence: i64, // 1..4 = nth weekday, 5 = last weekday
    pub second: i64,
}

impl Rule {
    fn utc(self, year: i64, before_offset: i32) -> Result<i64, String> {
        if !(1..=12).contains(&self.month)
            || !(0..=6).contains(&self.weekday)
            || !(1..=5).contains(&self.occurrence)
            || !(0..86_400).contains(&self.second)
        {
            return Err("Invalid Windows timezone transition rule".into());
        }
        let first = days_from_civil(year, self.month, 1);
        let first_weekday = (first + 4).rem_euclid(7);
        let mut day = 1 + (self.weekday - first_weekday).rem_euclid(7) + 7 * (self.occurrence - 1);
        let next_month = if self.month == 12 {
            days_from_civil(year + 1, 1, 1)
        } else {
            days_from_civil(year, self.month + 1, 1)
        };
        if day > next_month - first {
            day -= 7;
        }
        Ok((first + day - 1) * 86_400 + self.second - i64::from(before_offset))
    }
}

pub(super) fn is_daylight(
    seconds: i64,
    year: i64,
    std_offset: i32,
    dst_offset: i32,
    daylight: Rule,
    standard: Rule,
) -> Result<bool, String> {
    let start = daylight.utc(year, std_offset)?;
    let end = standard.utc(year, dst_offset)?;
    Ok(if start < end {
        seconds >= start && seconds < end
    } else if end < start {
        seconds < end || seconds >= start
    } else {
        false
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rule(month: i64, occurrence: i64) -> Rule {
        Rule {
            month,
            occurrence,
            weekday: 0,
            second: 7200,
        }
    }
    #[test]
    fn northern_dst_exact_boundaries_and_full_date_range() {
        for year in [-271_821, -1, 0, 2024, 275_760] {
            let start = rule(3, 2).utc(year, -18000).unwrap();
            let end = rule(11, 1).utc(year, -14400).unwrap();
            for (seconds, expected) in [
                (start - 1, false),
                (start, true),
                (end - 1, true),
                (end, false),
            ] {
                assert_eq!(
                    is_daylight(seconds, year, -18000, -14400, rule(3, 2), rule(11, 1)).unwrap(),
                    expected
                );
            }
        }
    }
    #[test]
    fn southern_dst_and_last_weekday() {
        let year = 2024;
        let dst = rule(10, 1);
        let standard = rule(4, 1);
        assert!(is_daylight(
            days_from_civil(year, 1, 1) * 86400,
            year,
            36000,
            39600,
            dst,
            standard
        )
        .unwrap());
        assert!(!is_daylight(
            days_from_civil(year, 7, 1) * 86400,
            year,
            36000,
            39600,
            dst,
            standard
        )
        .unwrap());
        assert_eq!(
            rule(3, 5).utc(year, 0).unwrap(),
            days_from_civil(year, 3, 31) * 86400 + 7200
        );
        assert_eq!(
            rule(2, 5).utc(year, 0).unwrap(),
            days_from_civil(year, 2, 25) * 86400 + 7200
        );
    }
}
