use crate::{IntentionCadence, Prayer, PrayerStatus};
use facet::Facet;
use serde::{Deserialize, Serialize};

pub const DIGEST_HORIZON_DAYS: u32 = 14;

/// Calendar date in the user's local timezone (weekday 1 = Sunday, matching `Calendar`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivilDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivilDateTime {
    pub date: CivilDate,
    pub hour: u32,
    pub minute: u32,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct ReminderDigest {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub intentions: Vec<String>,
}

/// Daily every day; weekly on Sunday; monthly on the 1st. Active + scheduled only.
pub fn is_due(prayer: &Prayer, date: CivilDate) -> bool {
    if prayer.status != PrayerStatus::Active {
        return false;
    }
    match prayer.cadence {
        IntentionCadence::Unscheduled => false,
        IntentionCadence::Daily => true,
        IntentionCadence::Weekly => weekday(date) == 1,
        IntentionCadence::Monthly => date.day == 1,
    }
}

pub fn plan_digests(
    prayers: &[Prayer],
    hour: u8,
    minute: u8,
    now: CivilDateTime,
    horizon_days: u32,
) -> Vec<ReminderDigest> {
    if prayers.is_empty() || horizon_days == 0 {
        return Vec::new();
    }

    let mut digests = Vec::new();
    let mut date = now.date;

    for day_offset in 0..horizon_days {
        if day_offset > 0 {
            date = add_days(date, 1);
        }

        let fire = CivilDateTime {
            date,
            hour: u32::from(hour),
            minute: u32::from(minute),
        };
        if !is_future(fire, now) {
            continue;
        }

        let intentions: Vec<String> = prayers
            .iter()
            .filter(|prayer| is_due(prayer, date))
            .map(|prayer| prayer.intention.clone())
            .collect();

        if intentions.is_empty() {
            continue;
        }

        digests.push(ReminderDigest {
            year: u16::try_from(date.year).unwrap_or(i16::MAX as u16),
            month: date.month as u8,
            day: date.day as u8,
            hour,
            minute,
            intentions,
        });
    }

    digests
}

fn is_future(fire: CivilDateTime, now: CivilDateTime) -> bool {
    if fire.date.year != now.date.year {
        return fire.date.year > now.date.year;
    }
    if fire.date.month != now.date.month {
        return fire.date.month > now.date.month;
    }
    if fire.date.day != now.date.day {
        return fire.date.day > now.date.day;
    }
    if fire.hour != now.hour {
        return fire.hour > now.hour;
    }
    fire.minute > now.minute
}

/// Sunday = 1 … Saturday = 7 (matches `Calendar.component(.weekday)`).
pub fn weekday(date: CivilDate) -> u32 {
    let y = if date.month < 3 {
        date.year - 1
    } else {
        date.year
    };
    let m = if date.month < 3 {
        date.month + 12
    } else {
        date.month
    };
    let k = y % 100;
    let j = y / 100;
    let h = (i32::try_from(date.day).unwrap_or(0)
        + (13 * (i32::try_from(m).unwrap_or(0) + 1)) / 5
        + k
        + k / 4
        + j / 4
        + 5 * j)
        % 7;
    u32::try_from((h + 6) % 7 + 1).unwrap_or(1)
}

pub fn add_days(date: CivilDate, days: u32) -> CivilDate {
    let mut y = date.year;
    let mut m = date.month;
    let mut d = date.day + days;

    while d > days_in_month(y, m) {
        d -= days_in_month(y, m);
        m += 1;
        if m > 12 {
            m = 1;
            y += 1;
        }
    }

    CivilDate { year: y, month: m, day: d }
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
    use crate::Prayer;

    fn scheduled_prayer(intention: &str, cadence: IntentionCadence) -> Prayer {
        Prayer {
            id: 0,
            intention: intention.into(),
            details: None,
            tags: vec![],
            status: PrayerStatus::Active,
            cadence,
        }
    }

    fn dt(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> CivilDateTime {
        CivilDateTime {
            date: CivilDate { year, month, day },
            hour,
            minute,
        }
    }

    #[test]
    fn weekday_matches_sunday_and_monday() {
        // 2026-08-12 is a Wednesday
        assert_eq!(weekday(CivilDate { year: 2026, month: 8, day: 12 }), 4);
        // 2026-08-16 is a Sunday
        assert_eq!(weekday(CivilDate { year: 2026, month: 8, day: 16 }), 1);
    }

    #[test]
    fn daily_is_due_every_day() {
        let prayer = scheduled_prayer("Mom", IntentionCadence::Daily);
        let date = CivilDate {
            year: 2026,
            month: 8,
            day: 12,
        };
        assert!(is_due(&prayer, date));
    }

    #[test]
    fn weekly_is_due_on_sunday_only() {
        let prayer = scheduled_prayer("Parish", IntentionCadence::Weekly);
        assert!(!is_due(
            &prayer,
            CivilDate {
                year: 2026,
                month: 8,
                day: 12
            }
        ));
        assert!(is_due(
            &prayer,
            CivilDate {
                year: 2026,
                month: 8,
                day: 16
            }
        ));
    }

    #[test]
    fn monthly_is_due_on_first_only() {
        let prayer = scheduled_prayer("Souls", IntentionCadence::Monthly);
        assert!(!is_due(
            &prayer,
            CivilDate {
                year: 2026,
                month: 8,
                day: 12
            }
        ));
        assert!(is_due(
            &prayer,
            CivilDate {
                year: 2026,
                month: 9,
                day: 1
            }
        ));
    }

    #[test]
    fn archived_and_unscheduled_are_never_due() {
        let mut prayer = scheduled_prayer("Mom", IntentionCadence::Daily);
        prayer.status = PrayerStatus::Archived;
        assert!(!is_due(&prayer, CivilDate { year: 2026, month: 8, day: 12 }));

        let unscheduled = scheduled_prayer("Dad", IntentionCadence::Unscheduled);
        assert!(!is_due(
            &unscheduled,
            CivilDate {
                year: 2026,
                month: 8,
                day: 12
            }
        ));
    }

    #[test]
    fn plan_skips_past_times_and_empty_days() {
        let prayers = vec![scheduled_prayer("Mom", IntentionCadence::Daily)];
        let now = dt(2026, 8, 12, 9, 0);
        let digests = plan_digests(&prayers, 8, 0, now, 3);
        assert_eq!(digests.len(), 2);
        assert_eq!(digests[0].day, 13);
        assert_eq!(digests[0].intentions, vec!["Mom".to_string()]);
    }

    #[test]
    fn plan_groups_multiple_intentions_same_day() {
        let prayers = vec![
            scheduled_prayer("Mom", IntentionCadence::Daily),
            scheduled_prayer("Dad", IntentionCadence::Daily),
        ];
        let now = dt(2026, 8, 12, 7, 0);
        let digests = plan_digests(&prayers, 8, 0, now, 1);
        assert_eq!(digests.len(), 1);
        assert_eq!(digests[0].intentions.len(), 2);
    }

    #[test]
    fn plan_weekly_only_on_sundays_in_horizon() {
        let prayers = vec![scheduled_prayer("Parish", IntentionCadence::Weekly)];
        let now = dt(2026, 8, 12, 7, 0); // Wed
        let digests = plan_digests(&prayers, 8, 0, now, 14);
        assert_eq!(digests.len(), 2);
        assert_eq!(digests[0].day, 16);
        assert_eq!(digests[1].day, 23);
    }
}
