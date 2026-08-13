//! Liturgical calendar helpers (Western / General Roman Calendar).

use crate::reminder::{add_days, weekday, CivilDate};

/// Western Easter Sunday (Gregorian calendar), per the Anonymous Gregorian algorithm.
pub fn western_easter(year: i32) -> CivilDate {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = u32::try_from((h + l - 7 * m + 114) / 31).unwrap_or(3);
    let day = u32::try_from((h + l - 7 * m + 114) % 31 + 1).unwrap_or(1);

    CivilDate { year, month, day }
}

/// Offset from Easter Sunday by a signed number of days (negative = before Easter).
pub fn easter_offset(year: i32, days: i32) -> CivilDate {
    let easter = western_easter(year);
    if days >= 0 {
        add_days(easter, u32::try_from(days).unwrap_or(0))
    } else {
        subtract_days(easter, u32::try_from(-days).unwrap_or(0))
    }
}

fn subtract_days(date: CivilDate, days: u32) -> CivilDate {
    let mut y = date.year;
    let mut m = date.month;
    let mut d = i32::try_from(date.day).unwrap_or(1) - i32::try_from(days).unwrap_or(0);

    while d < 1 {
        m -= 1;
        if m < 1 {
            m = 12;
            y -= 1;
        }
        d += i32::try_from(days_in_month(y, m)).unwrap_or(28);
    }

    CivilDate {
        year: y,
        month: m,
        day: u32::try_from(d).unwrap_or(1),
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> CivilDate {
        CivilDate { year, month, day }
    }

    #[test]
    fn western_easter_known_years() {
        assert_eq!(western_easter(2025), date(2025, 4, 20));
        assert_eq!(western_easter(2026), date(2026, 4, 5));
        assert_eq!(western_easter(2027), date(2027, 3, 28));
    }

    #[test]
    fn western_easter_is_always_sunday() {
        for year in 1900..=2100 {
            let easter = western_easter(year);
            assert_eq!(
                weekday(easter),
                1,
                "Easter {year} should fall on a Sunday, got {:?}",
                easter
            );
        }
    }

    #[test]
    fn easter_offset_pentecost_is_49_days_after() {
        for year in [2025, 2026, 2027] {
            assert_eq!(
                easter_offset(year, 49),
                add_days(western_easter(year), 49),
                "Pentecost offset for {year}"
            );
        }
    }

    #[test]
    fn easter_offset_ash_wednesday_is_46_days_before() {
        for year in [2025, 2026, 2027] {
            assert_eq!(
                easter_offset(year, -46),
                subtract_days(western_easter(year), 46),
                "Ash Wednesday offset for {year}"
            );
        }
    }
}
