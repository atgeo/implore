use crate::reminder::CivilDateTime;
use facet::Facet;
use serde::{Deserialize, Serialize};

/// One logged prayer, stamped with the user's local date and time.
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct PrayerLogEntry {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

impl PrayerLogEntry {
    pub const fn from_local(now: CivilDateTime) -> Self {
        Self {
            year: now.date.year,
            month: now.date.month,
            day: now.date.day,
            hour: now.hour,
            minute: now.minute,
        }
    }
}

/// Appends `entry` (append order preserved, repeats allowed).
pub fn append_entry(prayed_on: &mut Vec<PrayerLogEntry>, entry: PrayerLogEntry) {
    prayed_on.push(entry);
}

/// Removes the entry at `index`. Returns whether the list changed.
pub fn remove_entry(prayed_on: &mut Vec<PrayerLogEntry>, index: usize) -> bool {
    if index >= prayed_on.len() {
        return false;
    }
    prayed_on.remove(index);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reminder::CivilDate;

    fn entry(day: u32, hour: u32, minute: u32) -> PrayerLogEntry {
        PrayerLogEntry {
            year: 2026,
            month: 8,
            day,
            hour,
            minute,
        }
    }

    #[test]
    fn from_local_keeps_date_and_time() {
        let now = CivilDateTime {
            date: CivilDate {
                year: 2026,
                month: 8,
                day: 12,
            },
            hour: 15,
            minute: 30,
        };

        assert_eq!(PrayerLogEntry::from_local(now), entry(12, 15, 30));
    }

    #[test]
    fn append_allows_multiple_entries_for_same_day() {
        let mut prayed_on = Vec::new();
        append_entry(&mut prayed_on, entry(12, 9, 0));
        append_entry(&mut prayed_on, entry(12, 18, 15));
        assert_eq!(prayed_on, vec![entry(12, 9, 0), entry(12, 18, 15)]);
    }

    #[test]
    fn remove_entry_removes_one_item() {
        let mut prayed_on = vec![entry(10, 8, 0), entry(12, 9, 0), entry(12, 21, 0)];
        assert!(remove_entry(&mut prayed_on, 1));
        assert_eq!(prayed_on, vec![entry(10, 8, 0), entry(12, 21, 0)]);
    }

    #[test]
    fn remove_entry_out_of_bounds_is_noop() {
        let mut prayed_on = vec![entry(10, 8, 0)];
        assert!(!remove_entry(&mut prayed_on, 3));
        assert_eq!(prayed_on.len(), 1);
    }
}
