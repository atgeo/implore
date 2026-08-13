//! Liturgical calendar helpers for the Dioceses of the United States
//! (General Roman Calendar with US movable-feast transfers).
//!
//! Ordinary Time weeks are Sunday–Saturday. Week 1 weekdays follow the Baptism
//! of the Lord; later weekday cycles follow that week's Sunday. After Pentecost
//! the numbering is chosen so Christ the King is always the 34th Sunday.

use std::cmp::Ordering;

use crate::reminder::{add_days, weekday, CivilDate};
use facet::Facet;
use serde::{Deserialize, Serialize};

/// Temporal cycle day (US calendar). Weekday is 1 = Sunday … 7 = Saturday,
/// matching [`weekday`]. Saint feasts are a separate lookup.
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum LiturgicalDay {
    Advent { week: u32, weekday: u32 },
    ChristmasDay,
    HolyFamily,
    ChristmasSeason { weekday: u32 },
    Epiphany,
    BaptismOfTheLord,
    OrdinaryTime { week: u32, weekday: u32 },
    AshWednesday,
    AfterAshWednesday { weekday: u32 },
    Lent { week: u32, weekday: u32 },
    PalmSunday,
    HolyWeek { weekday: u32 },
    HolyThursday,
    GoodFriday,
    HolySaturday,
    EasterSunday,
    Easter { week: u32, weekday: u32 },
    Pentecost,
    TrinitySunday,
    CorpusChristi,
    SacredHeart,
    ChristTheKing,
}

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

pub fn ash_wednesday(year: i32) -> CivilDate {
    easter_offset(year, -46)
}

pub fn pentecost(year: i32) -> CivilDate {
    easter_offset(year, 49)
}

/// Epiphany in the United States: Sunday from 2 through 8 January.
pub fn epiphany(year: i32) -> CivilDate {
    sunday_on_or_after(CivilDate {
        year,
        month: 1,
        day: 2,
    })
}

/// Baptism of the Lord (US): Sunday after Epiphany, or the following Monday
/// when Epiphany falls on 7 or 8 January.
pub fn baptism_of_the_lord(year: i32) -> CivilDate {
    let epiphany = epiphany(year);
    if epiphany.day >= 7 {
        add_days(epiphany, 1)
    } else {
        add_days(epiphany, 7)
    }
}

/// First Sunday of Advent: Sunday from 27 November through 3 December.
pub fn first_sunday_of_advent(year: i32) -> CivilDate {
    sunday_on_or_after(CivilDate {
        year,
        month: 11,
        day: 27,
    })
}

pub fn christ_the_king(year: i32) -> CivilDate {
    subtract_days(first_sunday_of_advent(year), 7)
}

/// Whether `date` falls in Ordinary Time (US calendar).
pub fn is_ordinary_time(date: CivilDate) -> bool {
    let year = date.year;
    let baptism = baptism_of_the_lord(year);
    let ash = ash_wednesday(year);
    let pentecost = pentecost(year);
    let advent = first_sunday_of_advent(year);

    (cmp_date(date, baptism) == Ordering::Greater && cmp_date(date, ash) == Ordering::Less)
        || (cmp_date(date, pentecost) == Ordering::Greater
            && cmp_date(date, advent) == Ordering::Less)
}

/// Ordinary Time week number (1–34) for a civil date, if the day is in Ordinary Time.
pub fn ordinary_time_week(date: CivilDate) -> Option<u32> {
    if !is_ordinary_time(date) {
        return None;
    }

    let sunday = sunday_on_or_before(date);
    let year = date.year;
    if cmp_date(sunday, ash_wednesday(year)) == Ordering::Less {
        let week1 = sunday_from_start(year, 1);
        Some(1 + days_between(week1, sunday) / 7)
    } else {
        Some(34 - days_between(sunday, christ_the_king(year)) / 7)
    }
}

/// Sunday of Ordinary Time week `n` when that Sunday itself falls in Ordinary Time.
///
/// There is no 1st Sunday in Ordinary Time (Baptism of the Lord occupies that
/// slot). Some later Sundays are omitted when they fall in Lent or Easter.
pub fn sunday_of_ordinary_time_week(year: i32, week: u32) -> Option<CivilDate> {
    ordinary_time_weekday(year, week, 1)
}

/// Temporal cycle identity of `date` (US calendar).
///
/// The shell formats this (for example Ordinary Time week 19, Thursday →
/// “Thursday of the 19th Week in Ordinary Time”). Saint feasts are not included.
pub fn liturgical_day_for(date: CivilDate) -> LiturgicalDay {
    let year = date.year;
    let wd = weekday(date);

    if date.month == 12 && date.day == 25 {
        return LiturgicalDay::ChristmasDay;
    }

    let easter = western_easter(year);
    if date == easter {
        return LiturgicalDay::EasterSunday;
    }

    let pentecost = pentecost(year);
    if date == pentecost {
        return LiturgicalDay::Pentecost;
    }
    if date == add_days(pentecost, 7) {
        return LiturgicalDay::TrinitySunday;
    }
    if date == add_days(pentecost, 14) {
        return LiturgicalDay::CorpusChristi;
    }
    if date == add_days(pentecost, 19) {
        return LiturgicalDay::SacredHeart;
    }

    if date == ash_wednesday(year) {
        return LiturgicalDay::AshWednesday;
    }
    if date == epiphany(year) {
        return LiturgicalDay::Epiphany;
    }
    if date == baptism_of_the_lord(year) {
        return LiturgicalDay::BaptismOfTheLord;
    }
    if date == christ_the_king(year) {
        return LiturgicalDay::ChristTheKing;
    }
    if date == holy_family(year) {
        return LiturgicalDay::HolyFamily;
    }

    let palm = easter_offset(year, -7);
    if date == palm {
        return LiturgicalDay::PalmSunday;
    }
    if date == easter_offset(year, -3) {
        return LiturgicalDay::HolyThursday;
    }
    if date == easter_offset(year, -2) {
        return LiturgicalDay::GoodFriday;
    }
    if date == easter_offset(year, -1) {
        return LiturgicalDay::HolySaturday;
    }

    if cmp_date(date, palm) == Ordering::Greater
        && cmp_date(date, easter_offset(year, -3)) == Ordering::Less
    {
        return LiturgicalDay::HolyWeek { weekday: wd };
    }

    let ash = ash_wednesday(year);
    let lent1 = easter_offset(year, -42);
    if cmp_date(date, ash) == Ordering::Greater && cmp_date(date, lent1) == Ordering::Less {
        return LiturgicalDay::AfterAshWednesday { weekday: wd };
    }
    if cmp_date(date, lent1) != Ordering::Less && cmp_date(date, palm) == Ordering::Less {
        let sunday = sunday_on_or_before(date);
        let week = 1 + days_between(lent1, sunday) / 7;
        return LiturgicalDay::Lent { week, weekday: wd };
    }

    if cmp_date(date, easter) == Ordering::Greater && cmp_date(date, pentecost) == Ordering::Less {
        let sunday = sunday_on_or_before(date);
        let week = 1 + days_between(easter, sunday) / 7;
        return LiturgicalDay::Easter { week, weekday: wd };
    }

    let advent = first_sunday_of_advent(year);
    let in_advent = cmp_date(date, advent) != Ordering::Less
        && (date.month == 11 || (date.month == 12 && date.day <= 24));
    if in_advent {
        let sunday = sunday_on_or_before(date);
        let week = 1 + days_between(advent, sunday) / 7;
        return LiturgicalDay::Advent { week, weekday: wd };
    }

    let baptism = baptism_of_the_lord(year);
    if date.month == 12 && date.day > 25
        || date.month == 1 && cmp_date(date, baptism) == Ordering::Less
    {
        return LiturgicalDay::ChristmasSeason { weekday: wd };
    }

    if let Some(week) = ordinary_time_week(date) {
        return LiturgicalDay::OrdinaryTime { week, weekday: wd };
    }

    LiturgicalDay::ChristmasSeason { weekday: wd }
}

/// Civil date of a weekday in Ordinary Time week `n`.
///
/// `weekday` matches [`weekday`]: 1 = Sunday … 5 = Thursday … 7 = Saturday.
/// Weekdays of week `n` follow that week's Sunday (week 1 follows the Baptism
/// of the Lord). Returns `None` when that day is not in Ordinary Time — for
/// example Sunday of a week swallowed by Lent, Easter, or Pentecost.
pub fn ordinary_time_weekday(year: i32, week: u32, weekday: u32) -> Option<CivilDate> {
    if !(1..=7).contains(&weekday) {
        return None;
    }
    let sunday = notional_sunday_of_week(year, week)?;
    let date = if weekday == 1 {
        sunday
    } else {
        add_days(sunday, weekday - 1)
    };
    is_ordinary_time(date).then_some(date)
}

fn notional_sunday_of_week(year: i32, week: u32) -> Option<CivilDate> {
    if !(1..=34).contains(&week) {
        return None;
    }

    let from_start = sunday_from_start(year, week);
    if cmp_date(from_start, ash_wednesday(year)) == Ordering::Less {
        return Some(from_start);
    }

    let from_end = sunday_from_end(year, week);
    if cmp_date(from_end, pentecost(year)) != Ordering::Less
        && cmp_date(from_end, first_sunday_of_advent(year)) == Ordering::Less
    {
        return Some(from_end);
    }

    None
}

fn sunday_from_start(year: i32, week: u32) -> CivilDate {
    let second = next_sunday(baptism_of_the_lord(year));
    if week >= 2 {
        add_days(second, (week - 2) * 7)
    } else {
        subtract_days(second, 7)
    }
}

fn sunday_from_end(year: i32, week: u32) -> CivilDate {
    subtract_days(christ_the_king(year), (34 - week) * 7)
}

fn holy_family(year: i32) -> CivilDate {
    let christmas = CivilDate {
        year,
        month: 12,
        day: 25,
    };
    if weekday(christmas) == 1 {
        CivilDate {
            year,
            month: 12,
            day: 30,
        }
    } else {
        next_sunday(christmas)
    }
}

fn next_sunday(date: CivilDate) -> CivilDate {
    let wd = weekday(date);
    if wd == 1 {
        add_days(date, 7)
    } else {
        add_days(date, 8 - wd)
    }
}

fn sunday_on_or_after(date: CivilDate) -> CivilDate {
    let wd = weekday(date);
    if wd == 1 {
        date
    } else {
        add_days(date, 8 - wd)
    }
}

fn sunday_on_or_before(date: CivilDate) -> CivilDate {
    subtract_days(date, weekday(date) - 1)
}

fn cmp_date(a: CivilDate, b: CivilDate) -> Ordering {
    (a.year, a.month, a.day).cmp(&(b.year, b.month, b.day))
}

fn days_between(from: CivilDate, to: CivilDate) -> u32 {
    u32::try_from((julian_day(to) - julian_day(from)).max(0)).unwrap_or(0)
}

fn julian_day(date: CivilDate) -> i32 {
    let y = date.year;
    let m = i32::try_from(date.month).unwrap_or(1);
    let d = i32::try_from(date.day).unwrap_or(1);
    let a = (14 - m) / 12;
    let y = y + 4800 - a;
    let m = m + 12 * a - 3;
    d + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045
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

    #[test]
    fn us_epiphany_and_baptism_known_years() {
        assert_eq!(epiphany(2024), date(2024, 1, 7));
        assert_eq!(baptism_of_the_lord(2024), date(2024, 1, 8));
        assert_eq!(epiphany(2025), date(2025, 1, 5));
        assert_eq!(baptism_of_the_lord(2025), date(2025, 1, 12));
        assert_eq!(epiphany(2026), date(2026, 1, 4));
        assert_eq!(baptism_of_the_lord(2026), date(2026, 1, 11));
    }

    #[test]
    fn first_sunday_of_advent_known_years() {
        assert_eq!(first_sunday_of_advent(2024), date(2024, 12, 1));
        assert_eq!(first_sunday_of_advent(2025), date(2025, 11, 30));
        assert_eq!(first_sunday_of_advent(2026), date(2026, 11, 29));
        assert_eq!(first_sunday_of_advent(2027), date(2027, 11, 28));
    }

    #[test]
    fn ordinary_time_sundays_2026() {
        assert_eq!(sunday_of_ordinary_time_week(2026, 1), None);
        assert_eq!(
            sunday_of_ordinary_time_week(2026, 2),
            Some(date(2026, 1, 18))
        );
        assert_eq!(
            sunday_of_ordinary_time_week(2026, 6),
            Some(date(2026, 2, 15))
        );
        assert_eq!(sunday_of_ordinary_time_week(2026, 7), None);
        assert_eq!(sunday_of_ordinary_time_week(2026, 8), None);
        assert_eq!(
            sunday_of_ordinary_time_week(2026, 11),
            Some(date(2026, 6, 14))
        );
        assert_eq!(
            sunday_of_ordinary_time_week(2026, 18),
            Some(date(2026, 8, 2))
        );
        assert_eq!(
            sunday_of_ordinary_time_week(2026, 19),
            Some(date(2026, 8, 9))
        );
        assert_eq!(
            sunday_of_ordinary_time_week(2026, 34),
            Some(date(2026, 11, 22))
        );
    }

    #[test]
    fn thursday_of_19th_week_in_ordinary_time() {
        assert_eq!(ordinary_time_weekday(2025, 19, 5), Some(date(2025, 8, 14)));
        assert_eq!(ordinary_time_weekday(2026, 19, 5), Some(date(2026, 8, 13)));
        assert_eq!(ordinary_time_weekday(2027, 19, 5), Some(date(2027, 8, 12)));
    }

    #[test]
    fn weekdays_resume_after_pentecost_2026() {
        assert_eq!(ordinary_time_weekday(2026, 8, 1), None);
        assert_eq!(ordinary_time_weekday(2026, 8, 2), Some(date(2026, 5, 25)));
        assert_eq!(ordinary_time_weekday(2026, 8, 5), Some(date(2026, 5, 28)));
        assert_eq!(ordinary_time_weekday(2026, 7, 5), None);
    }

    #[test]
    fn ordinary_time_week_round_trip() {
        let thursday = ordinary_time_weekday(2026, 19, 5).expect("week 19 Thursday");
        assert_eq!(ordinary_time_week(thursday), Some(19));
        assert_eq!(weekday(thursday), 5);
        assert!(is_ordinary_time(thursday));
        assert!(!is_ordinary_time(ash_wednesday(2026)));
        assert!(!is_ordinary_time(pentecost(2026)));
        assert!(!is_ordinary_time(baptism_of_the_lord(2026)));
    }

    #[test]
    fn skipped_week_when_easter_is_early_2027() {
        assert_eq!(
            sunday_of_ordinary_time_week(2027, 5),
            Some(date(2027, 2, 7))
        );
        assert_eq!(sunday_of_ordinary_time_week(2027, 6), None);
        assert_eq!(sunday_of_ordinary_time_week(2027, 7), None);
        assert_eq!(ordinary_time_weekday(2027, 7, 2), Some(date(2027, 5, 17)));
    }

    #[test]
    fn liturgical_day_for_thursday_of_19th_week() {
        assert_eq!(
            liturgical_day_for(date(2026, 8, 13)),
            LiturgicalDay::OrdinaryTime {
                week: 19,
                weekday: 5
            }
        );
        assert_eq!(
            liturgical_day_for(date(2025, 8, 14)),
            LiturgicalDay::OrdinaryTime {
                week: 19,
                weekday: 5
            }
        );
    }

    #[test]
    fn liturgical_day_for_named_days_2026() {
        assert_eq!(
            liturgical_day_for(date(2026, 4, 5)),
            LiturgicalDay::EasterSunday
        );
        assert_eq!(
            liturgical_day_for(date(2026, 5, 24)),
            LiturgicalDay::Pentecost
        );
        assert_eq!(
            liturgical_day_for(date(2026, 2, 18)),
            LiturgicalDay::AshWednesday
        );
        assert_eq!(
            liturgical_day_for(date(2026, 1, 4)),
            LiturgicalDay::Epiphany
        );
        assert_eq!(
            liturgical_day_for(date(2026, 1, 11)),
            LiturgicalDay::BaptismOfTheLord
        );
        assert_eq!(
            liturgical_day_for(date(2026, 5, 31)),
            LiturgicalDay::TrinitySunday
        );
        assert_eq!(
            liturgical_day_for(date(2026, 6, 7)),
            LiturgicalDay::CorpusChristi
        );
        assert_eq!(
            liturgical_day_for(date(2026, 3, 29)),
            LiturgicalDay::PalmSunday
        );
        assert_eq!(
            liturgical_day_for(date(2026, 11, 22)),
            LiturgicalDay::ChristTheKing
        );
        assert_eq!(
            liturgical_day_for(date(2026, 12, 25)),
            LiturgicalDay::ChristmasDay
        );
    }

    #[test]
    fn liturgical_day_for_seasons_2026() {
        assert_eq!(
            liturgical_day_for(date(2026, 2, 19)),
            LiturgicalDay::AfterAshWednesday { weekday: 5 }
        );
        assert_eq!(
            liturgical_day_for(date(2026, 2, 25)),
            LiturgicalDay::Lent {
                week: 1,
                weekday: 4
            }
        );
        assert_eq!(
            liturgical_day_for(date(2026, 4, 6)),
            LiturgicalDay::Easter {
                week: 1,
                weekday: 2
            }
        );
        assert_eq!(
            liturgical_day_for(date(2026, 11, 29)),
            LiturgicalDay::Advent {
                week: 1,
                weekday: 1
            }
        );
        assert_eq!(
            liturgical_day_for(date(2026, 5, 25)),
            LiturgicalDay::OrdinaryTime {
                week: 8,
                weekday: 2
            }
        );
    }
}
