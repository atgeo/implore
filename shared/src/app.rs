#[derive(Default)]
pub struct Implore;

use crux_core::{
    macros::effect,
    render::{render, RenderOperation},
    App, Command,
};
use crux_kv::{command::KeyValue, KeyValueError, KeyValueOperation};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::prayer_log::{self, PrayerLogEntry};
use crate::reminder::{self, CivilDateTime, ReminderDigest, DIGEST_HORIZON_DAYS};
use crate::{liturgical_day_for, LiturgicalDay};

const PRAYERS_KEY: &str = "prayers";
/// Max tags stored on one intention (also exposed on [`ViewModel`]).
pub const MAX_TAGS: usize = 8;
/// Max characters per tag (also exposed on [`ViewModel`]).
pub const MAX_TAG_LEN: usize = 32;
/// Max characters in an intention title (also exposed on [`ViewModel`]).
pub const MAX_INTENTION_LEN: usize = 64;
/// Max characters in optional details (also exposed on [`ViewModel`]).
pub const MAX_DETAILS_LEN: usize = 512;

/// Marketing version from the workspace package (`CARGO_PKG_VERSION`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

impl App for Implore {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Restore => KeyValue::get(PRAYERS_KEY).then_send(Event::PrayersLoaded),
            Event::PrayersLoaded(result) => {
                if let Ok(Some(bytes)) = result {
                    if let Ok(stored) = serde_json::from_slice::<StoredState>(&bytes) {
                        model.prayers = stored.prayers;
                        model.next_id = stored.next_id;
                        model.reminder_settings = stored.reminder_settings;
                    }
                }
                render()
            }
            Event::AddPrayer {
                intention,
                details,
                tags,
                cadence,
                saint_id,
                color,
            } => {
                let intention = normalize_intention(intention);
                if intention.is_empty() {
                    return render();
                }

                let details = normalize_details(details);
                let saint_id = trim_optional(saint_id);
                let tags = normalize_tags(tags);
                let id = model.next_id;
                model.next_id += 1;
                model.prayers.push(Prayer {
                    id,
                    intention,
                    details,
                    tags,
                    status: PrayerStatus::Active,
                    cadence,
                    saint_id,
                    color,
                    prayed_on: Vec::new(),
                });

                render().and(persist_prayers(model))
            }
            Event::UpdatePrayer {
                id,
                intention,
                details,
                tags,
                cadence,
                saint_id,
                color,
            } => {
                let intention = normalize_intention(intention);
                if intention.is_empty() {
                    return render();
                }

                let details = normalize_details(details);
                let saint_id = trim_optional(saint_id);
                let tags = normalize_tags(tags);
                let Some(prayer) = model.prayers.iter_mut().find(|prayer| prayer.id == id) else {
                    return render();
                };
                if prayer.intention == intention
                    && prayer.details == details
                    && prayer.tags == tags
                    && prayer.cadence == cadence
                    && prayer.saint_id == saint_id
                    && prayer.color == color
                {
                    return render();
                }

                prayer.intention = intention;
                prayer.details = details;
                prayer.tags = tags;
                prayer.cadence = cadence;
                prayer.saint_id = saint_id;
                prayer.color = color;
                render().and(persist_prayers(model))
            }
            Event::RemovePrayer { id } => {
                let before = model.prayers.len();
                model.prayers.retain(|prayer| prayer.id != id);
                if model.prayers.len() == before {
                    render()
                } else {
                    render().and(persist_prayers(model))
                }
            }
            Event::RemoveAllPrayers => {
                if model.prayers.is_empty() {
                    render()
                } else {
                    model.prayers.clear();
                    render().and(persist_prayers(model))
                }
            }
            Event::ArchivePrayer { id } => set_status(model, id, PrayerStatus::Archived),
            Event::UnarchivePrayer { id } => set_status(model, id, PrayerStatus::Active),
            Event::SetReminderSettings {
                enabled,
                hour,
                minute,
            } => {
                let hour = hour.clamp(0, 23);
                let minute = snap_minute(minute);
                if model.reminder_settings.enabled == enabled
                    && model.reminder_settings.hour == hour
                    && model.reminder_settings.minute == minute
                {
                    return render();
                }
                model.reminder_settings = ReminderSettings {
                    enabled,
                    hour,
                    minute,
                };
                render().and(persist_state(model))
            }
            Event::SyncLocalTime {
                year,
                month,
                day,
                hour,
                minute,
            } => {
                model.local_now = Some(CivilDateTime {
                    date: reminder::CivilDate {
                        year: i32::from(year),
                        month: u32::from(month),
                        day: u32::from(day),
                    },
                    hour: u32::from(hour),
                    minute: u32::from(minute),
                });
                render()
            }
            Event::LogPrayer { id } => log_prayer(model, id),
            Event::RemovePrayerLogEntry { id, index } => {
                remove_prayer_log_entry(model, id, index as usize)
            }
            Event::Persisted(_) => Command::done(),
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        let reminder_prayers: Vec<Prayer> = model
            .prayers
            .iter()
            .filter(|prayer| {
                matches!(prayer.status, PrayerStatus::Active)
                    && !matches!(prayer.cadence, IntentionCadence::Unscheduled)
            })
            .cloned()
            .collect();

        let reminder_digests = if model.reminder_settings.enabled {
            model
                .local_now
                .map(|now| {
                    reminder::plan_digests(
                        &reminder_prayers,
                        model.reminder_settings.hour,
                        model.reminder_settings.minute,
                        now,
                        DIGEST_HORIZON_DAYS,
                    )
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        ViewModel {
            version: VERSION.to_string(),
            has_prayers: !model.prayers.is_empty(),
            prayers: model
                .prayers
                .iter()
                .filter(|prayer| matches!(prayer.status, PrayerStatus::Active))
                .cloned()
                .collect(),
            archived_prayers: model
                .prayers
                .iter()
                .filter(|prayer| matches!(prayer.status, PrayerStatus::Archived))
                .cloned()
                .collect(),
            today_prayers: model
                .local_now
                .map(|now| {
                    reminder::select_today(&model.prayers, now.date)
                        .into_iter()
                        .map(|prayer| {
                            let prayed_today =
                                prayer_log::prayed_on_date(&prayer.prayed_on, now.date);
                            TodayPrayer {
                                prayer,
                                prayed_today,
                            }
                        })
                        .collect()
                })
                .unwrap_or_default(),
            local_date: model.local_now.map(|now| now.date),
            reminder_settings: model.reminder_settings,
            reminder_digests,
            liturgical_day: model.local_now.map(|now| liturgical_day_for(now.date)),
            max_tags: MAX_TAGS as u8,
            max_tag_len: MAX_TAG_LEN as u8,
            max_intention_len: MAX_INTENTION_LEN as u8,
            max_details_len: MAX_DETAILS_LEN as u16,
        }
    }
}

fn log_prayer(model: &mut Model, id: u64) -> Command<Effect, Event> {
    let Some(now) = model.local_now else {
        return render();
    };
    let Some(prayer) = model.prayers.iter_mut().find(|prayer| prayer.id == id) else {
        return render();
    };
    if prayer.status != PrayerStatus::Active {
        return render();
    }

    prayer_log::append_entry(&mut prayer.prayed_on, PrayerLogEntry::from_local(now));
    render().and(persist_state(model))
}

fn remove_prayer_log_entry(model: &mut Model, id: u64, index: usize) -> Command<Effect, Event> {
    let Some(prayer) = model.prayers.iter_mut().find(|prayer| prayer.id == id) else {
        return render();
    };

    if prayer_log::remove_entry(&mut prayer.prayed_on, index) {
        render().and(persist_state(model))
    } else {
        render()
    }
}

fn set_status(model: &mut Model, id: u64, status: PrayerStatus) -> Command<Effect, Event> {
    let Some(prayer) = model.prayers.iter_mut().find(|prayer| prayer.id == id) else {
        return render();
    };
    if prayer.status == status {
        return render();
    }
    prayer.status = status;
    render().and(persist_prayers(model))
}

fn persist_prayers(model: &Model) -> Command<Effect, Event> {
    persist_state(model)
}

fn persist_state(model: &Model) -> Command<Effect, Event> {
    let stored = StoredState {
        prayers: model.prayers.clone(),
        next_id: model.next_id,
        reminder_settings: model.reminder_settings,
    };
    let bytes = serde_json::to_vec(&stored).unwrap_or_default();
    KeyValue::set(PRAYERS_KEY, bytes).then_send(Event::Persisted)
}

fn snap_minute(minute: u8) -> u8 {
    let snapped = (minute / 15) * 15;
    snapped.min(45)
}

fn trim_optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_intention(value: String) -> String {
    value
        .trim()
        .chars()
        .take(MAX_INTENTION_LEN)
        .collect()
}

fn normalize_details(value: String) -> Option<String> {
    let trimmed = value
        .trim()
        .chars()
        .take(MAX_DETAILS_LEN)
        .collect::<String>();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .map(|tag| {
            tag.trim()
                .chars()
                .take(MAX_TAG_LEN)
                .collect::<String>()
        })
        .filter(|tag| !tag.is_empty())
        .take(MAX_TAGS)
        .collect()
}

#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub enum PrayerStatus {
    #[default]
    Active,
    Archived,
}

#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub enum IntentionCadence {
    #[default]
    Unscheduled,
    Daily,
    Weekly,
    Monthly,
}

/// Preset accent colors for intention rows (shell maps to platform colors).
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub enum IntentionColor {
    #[default]
    None,
    Sky,
    Sage,
    Sand,
    Rose,
    Slate,
    Gold,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub enum Event {
    /// Load persisted prayers from the shell key-value store.
    Restore,
    AddPrayer {
        intention: String,
        details: String,
        tags: Vec<String>,
        cadence: IntentionCadence,
        saint_id: String,
        color: IntentionColor,
    },
    UpdatePrayer {
        id: u64,
        intention: String,
        details: String,
        tags: Vec<String>,
        cadence: IntentionCadence,
        saint_id: String,
        color: IntentionColor,
    },
    RemovePrayer {
        id: u64,
    },
    /// Delete every intention (active and archived).
    RemoveAllPrayers,
    ArchivePrayer {
        id: u64,
    },
    UnarchivePrayer {
        id: u64,
    },
    SetReminderSettings {
        enabled: bool,
        hour: u8,
        minute: u8,
    },
    /// Shell reports the user's local date/time for digest planning.
    SyncLocalTime {
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
    },
    /// Append a prayer log entry for this intention (today's local date).
    LogPrayer {
        id: u64,
    },
    /// Remove one prayer log entry by index in `prayed_on`.
    RemovePrayerLogEntry {
        id: u64,
        index: u64,
    },
    /// Internal: shell finished reading the prayers blob.
    #[serde(skip)]
    #[facet(skip)]
    PrayersLoaded(#[facet(opaque)] Result<Option<Vec<u8>>, KeyValueError>),
    /// Internal: shell finished writing the prayers blob.
    #[serde(skip)]
    #[facet(skip)]
    Persisted(#[facet(opaque)] Result<Option<Vec<u8>>, KeyValueError>),
}

#[derive(Default, Clone)]
pub struct Model {
    prayers: Vec<Prayer>,
    next_id: u64,
    reminder_settings: ReminderSettings,
    local_now: Option<CivilDateTime>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct ReminderSettings {
    pub enabled: bool,
    pub hour: u8,
    pub minute: u8,
}

impl Default for ReminderSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            hour: 8,
            minute: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct StoredState {
    prayers: Vec<Prayer>,
    next_id: u64,
    reminder_settings: ReminderSettings,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Prayer {
    pub id: u64,
    pub intention: String,
    pub details: Option<String>,
    pub tags: Vec<String>,
    pub status: PrayerStatus,
    pub cadence: IntentionCadence,
    #[serde(default)]
    pub saint_id: Option<String>,
    #[serde(default)]
    pub color: IntentionColor,
    pub prayed_on: Vec<PrayerLogEntry>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct TodayPrayer {
    pub prayer: Prayer,
    pub prayed_today: bool,
}

#[derive(Facet, Serialize, Deserialize, Clone)]
pub struct ViewModel {
    /// Workspace crate version (`CARGO_PKG_VERSION`).
    pub version: String,
    /// True when any intention exists (active or archived).
    pub has_prayers: bool,
    /// Active intentions for the Intentions list.
    pub prayers: Vec<Prayer>,
    /// Archived intentions for the Archived screen.
    pub archived_prayers: Vec<Prayer>,
    /// Intentions due on the shell's local date (never-prayed first, then least-recently-prayed).
    pub today_prayers: Vec<TodayPrayer>,
    /// Shell's local calendar date; set after `SyncLocalTime`.
    pub local_date: Option<reminder::CivilDate>,
    pub reminder_settings: ReminderSettings,
    pub reminder_digests: Vec<ReminderDigest>,
    /// Temporal cycle for the shell's local date; set after `SyncLocalTime`.
    pub liturgical_day: Option<LiturgicalDay>,
    /// Max tags per intention (shell UX + [`normalize_tags`]).
    pub max_tags: u8,
    /// Max characters per tag (shell UX + [`normalize_tags`]).
    pub max_tag_len: u8,
    /// Max characters in an intention title (shell UX + [`normalize_intention`]).
    pub max_intention_len: u8,
    /// Max characters in optional details (shell UX + [`normalize_details`]).
    pub max_details_len: u16,
}

impl Default for ViewModel {
    fn default() -> Self {
        Self {
            version: VERSION.to_string(),
            has_prayers: false,
            prayers: Vec::new(),
            archived_prayers: Vec::new(),
            today_prayers: Vec::new(),
            local_date: None,
            reminder_settings: ReminderSettings::default(),
            reminder_digests: Vec::new(),
            liturgical_day: None,
            max_tags: MAX_TAGS as u8,
            max_tag_len: MAX_TAG_LEN as u8,
            max_intention_len: MAX_INTENTION_LEN as u8,
            max_details_len: MAX_DETAILS_LEN as u16,
        }
    }
}

#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    KeyValue(KeyValueOperation),
}

#[cfg(test)]
mod test {
    use super::*;

    fn prayer(
        id: u64,
        intention: &str,
        details: Option<&str>,
        tags: &[&str],
        status: PrayerStatus,
    ) -> Prayer {
        Prayer {
            id,
            intention: intention.into(),
            details: details.map(str::to_string),
            tags: tags.iter().map(|tag| (*tag).into()).collect(),
            status,
            cadence: IntentionCadence::Unscheduled,
            saint_id: None,
            color: IntentionColor::None,
            prayed_on: Vec::new(),
        }
    }

    fn add(
        app: &Implore,
        model: &mut Model,
        intention: &str,
        details: &str,
        tags: &[&str],
    ) -> Command<Effect, Event> {
        add_with_cadence(
            app,
            model,
            intention,
            details,
            tags,
            IntentionCadence::Unscheduled,
        )
    }

    fn add_with_cadence(
        app: &Implore,
        model: &mut Model,
        intention: &str,
        details: &str,
        tags: &[&str],
        cadence: IntentionCadence,
    ) -> Command<Effect, Event> {
        app.update(
            Event::AddPrayer {
                intention: intention.into(),
                details: details.into(),
                tags: tags.iter().map(|tag| (*tag).into()).collect(),
                cadence,
                saint_id: String::new(),
                color: IntentionColor::None,
            },
            model,
        )
    }

    #[test]
    fn restore_requests_get() {
        let app = Implore;
        let mut model = Model::default();

        app.update(Event::Restore, &mut model)
            .expect_key_value_with(|op| {
                assert!(matches!(
                    op,
                    KeyValueOperation::Get { key } if key == PRAYERS_KEY
                ));
            });
    }

    #[test]
    fn loads_persisted_prayers() {
        let app = Implore;
        let mut model = Model::default();

        let stored = StoredState {
            prayers: vec![prayer(3, "Mom", None, &[], PrayerStatus::Active)],
            next_id: 4,
            reminder_settings: ReminderSettings::default(),
        };
        let bytes = serde_json::to_vec(&stored).unwrap();

        app.update(Event::PrayersLoaded(Ok(Some(bytes))), &mut model)
            .expect_only_render();

        assert_eq!(model.next_id, 4);
        assert_eq!(
            app.view(&model).prayers,
            vec![prayer(3, "Mom", None, &[], PrayerStatus::Active)]
        );
    }

    #[test]
    fn loads_empty_when_missing() {
        let app = Implore;
        let mut model = Model::default();

        app.update(Event::PrayersLoaded(Ok(None)), &mut model)
            .expect_only_render();

        assert!(app.view(&model).prayers.is_empty());
        assert_eq!(model.next_id, 0);
    }

    #[test]
    fn loads_persisted_prayers_without_saint_id_field() {
        let app = Implore;
        let mut model = Model::default();

        let legacy = r#"{"prayers":[{"id":1,"intention":"Mom","details":null,"tags":[],"status":"Active","cadence":"Unscheduled","prayed_on":[]}],"next_id":2,"reminder_settings":{"enabled":false,"hour":8,"minute":0}}"#;
        app.update(
            Event::PrayersLoaded(Ok(Some(legacy.as_bytes().to_vec()))),
            &mut model,
        )
        .expect_only_render();

        assert_eq!(app.view(&model).prayers.len(), 1);
        assert_eq!(app.view(&model).prayers[0].saint_id, None);
        assert_eq!(app.view(&model).prayers[0].color, IntentionColor::None);
    }

    #[test]
    fn renders() {
        let app = Implore;
        let mut model = Model::default();
        let mut cmd = add(&app, &mut model, "Mom", "", &[]);
        assert_eq!(cmd.effects().count(), 2);
    }

    #[test]
    fn shows_empty_list_initially() {
        let app = Implore;
        let model = Model::default();

        assert!(app.view(&model).prayers.is_empty());
        assert_eq!(app.view(&model).version, VERSION);
    }

    #[test]
    fn adds_prayer_with_optional_fields() {
        let app = Implore;
        let mut model = Model::default();

        let mut cmd = add(
            &app,
            &mut model,
            "Mom",
            "Surgery recovery",
            &["family", "health"],
        );
        assert_eq!(cmd.effects().count(), 2);

        assert_eq!(
            app.view(&model).prayers,
            vec![prayer(
                0,
                "Mom",
                Some("Surgery recovery"),
                &["family", "health"],
                PrayerStatus::Active,
            )]
        );
    }

    #[test]
    fn adds_prayer_without_optional_fields() {
        let app = Implore;
        let mut model = Model::default();

        let mut cmd = add(&app, &mut model, "Mom", "", &[]);
        assert_eq!(cmd.effects().count(), 2);

        assert_eq!(
            app.view(&model).prayers,
            vec![prayer(0, "Mom", None, &[], PrayerStatus::Active)]
        );
    }

    #[test]
    fn ignores_empty_or_whitespace_intention() {
        let app = Implore;
        let mut model = Model::default();

        add(&app, &mut model, "   ", "note", &["tag"]).expect_only_render();
        add(&app, &mut model, "", "", &[]).expect_only_render();

        assert!(app.view(&model).prayers.is_empty());
    }

    #[test]
    fn trims_fields_and_drops_empty_tags() {
        let app = Implore;
        let mut model = Model::default();

        let mut cmd = add(
            &app,
            &mut model,
            "  Dad  ",
            "  recovery  ",
            &["  family  ", "  ", "health"],
        );
        assert_eq!(cmd.effects().count(), 2);

        assert_eq!(
            app.view(&model).prayers,
            vec![prayer(
                0,
                "Dad",
                Some("recovery"),
                &["family", "health"],
                PrayerStatus::Active,
            )]
        );
    }

    #[test]
    fn caps_tag_count_and_length() {
        let app = Implore;
        let mut model = Model::default();
        let long = "a".repeat(40);
        let tags = [
            long.as_str(),
            "t1",
            "t2",
            "t3",
            "t4",
            "t5",
            "t6",
            "t7",
            "t8",
            "t9",
            "t10",
            "t11",
        ];

        let _ = add(&app, &mut model, "Mom", "", &tags);

        let saved = &app.view(&model).prayers[0].tags;
        assert_eq!(saved.len(), MAX_TAGS);
        assert_eq!(saved[0].chars().count(), MAX_TAG_LEN);
        assert_eq!(saved[1], "t1");
        assert_eq!(saved.last().unwrap(), "t7");

        let view = app.view(&model);
        assert_eq!(view.max_tags as usize, MAX_TAGS);
        assert_eq!(view.max_tag_len as usize, MAX_TAG_LEN);
    }

    #[test]
    fn caps_intention_and_details_length() {
        let app = Implore;
        let mut model = Model::default();
        let long_intention = "i".repeat(MAX_INTENTION_LEN + 20);
        let long_details = "d".repeat(MAX_DETAILS_LEN + 100);

        let _ = add(
            &app,
            &mut model,
            format!("  {long_intention}  ").as_str(),
            long_details.as_str(),
            &[],
        );

        let saved = &app.view(&model).prayers[0];
        assert_eq!(saved.intention.chars().count(), MAX_INTENTION_LEN);
        assert_eq!(saved.details.as_ref().unwrap().chars().count(), MAX_DETAILS_LEN);

        let view = app.view(&model);
        assert_eq!(view.max_intention_len as usize, MAX_INTENTION_LEN);
        assert_eq!(view.max_details_len as usize, MAX_DETAILS_LEN);
    }

    #[test]
    fn removes_prayer_by_id() {
        let app = Implore;
        let mut model = Model::default();

        let _ = add(&app, &mut model, "Mom", "", &[]);
        let _ = add(&app, &mut model, "Dad", "", &[]);

        let mut cmd = app.update(Event::RemovePrayer { id: 0 }, &mut model);
        assert_eq!(cmd.effects().count(), 2);

        assert_eq!(
            app.view(&model).prayers,
            vec![prayer(1, "Dad", None, &[], PrayerStatus::Active)]
        );
    }

    #[test]
    fn remove_unknown_id_is_noop() {
        let app = Implore;
        let mut model = Model::default();

        let _ = add(&app, &mut model, "Mom", "", &[]);

        app.update(Event::RemovePrayer { id: 99 }, &mut model)
            .expect_only_render();

        assert_eq!(
            app.view(&model).prayers,
            vec![prayer(0, "Mom", None, &[], PrayerStatus::Active)]
        );
    }

    #[test]
    fn removes_all_prayers_and_persists() {
        let app = Implore;
        let mut model = Model::default();

        let _ = add(&app, &mut model, "Mom", "", &[]);
        let _ = add(&app, &mut model, "Dad", "", &[]);
        let _ = app.update(Event::ArchivePrayer { id: 0 }, &mut model);

        assert!(app.view(&model).has_prayers);

        app.update(Event::RemoveAllPrayers, &mut model)
            .expect_render()
            .expect_key_value_with(|op| {
                let KeyValueOperation::Set { value, .. } = op else {
                    panic!("expected KeyValue set effect");
                };
                let stored: StoredState = serde_json::from_slice(value).unwrap();
                assert!(stored.prayers.is_empty());
                assert_eq!(stored.next_id, 2);
            });

        let view = app.view(&model);
        assert!(!view.has_prayers);
        assert!(view.prayers.is_empty());
        assert_eq!(model.next_id, 2);
    }

    #[test]
    fn remove_all_prayers_when_empty_is_noop() {
        let app = Implore;
        let mut model = Model::default();

        app.update(Event::RemoveAllPrayers, &mut model)
            .expect_only_render();
        assert!(!app.view(&model).has_prayers);
    }

    #[test]
    fn archives_prayer_and_hides_from_active_list() {
        let app = Implore;
        let mut model = Model::default();

        let _ = add(&app, &mut model, "Mom", "", &[]);
        let _ = add(&app, &mut model, "Dad", "", &[]);

        let mut cmd = app.update(Event::ArchivePrayer { id: 0 }, &mut model);
        assert_eq!(cmd.effects().count(), 2);

        assert_eq!(
            app.view(&model).prayers,
            vec![prayer(1, "Dad", None, &[], PrayerStatus::Active)]
        );
        assert_eq!(
            app.view(&model).archived_prayers,
            vec![prayer(0, "Mom", None, &[], PrayerStatus::Archived)]
        );
        assert_eq!(model.prayers[0].status, PrayerStatus::Archived);
    }

    #[test]
    fn active_and_archived_lists_are_separate() {
        let app = Implore;
        let mut model = Model::default();

        let _ = add(&app, &mut model, "Mom", "", &[]);
        let _ = add(&app, &mut model, "Dad", "", &[]);
        let _ = app.update(Event::ArchivePrayer { id: 0 }, &mut model);

        let view = app.view(&model);
        assert_eq!(
            view.prayers,
            vec![prayer(1, "Dad", None, &[], PrayerStatus::Active)]
        );
        assert_eq!(
            view.archived_prayers,
            vec![prayer(0, "Mom", None, &[], PrayerStatus::Archived)]
        );
    }

    #[test]
    fn unarchives_prayer() {
        let app = Implore;
        let mut model = Model::default();

        let _ = add(&app, &mut model, "Mom", "", &[]);
        let _ = app.update(Event::ArchivePrayer { id: 0 }, &mut model);
        assert!(app.view(&model).prayers.is_empty());

        let mut cmd = app.update(Event::UnarchivePrayer { id: 0 }, &mut model);
        assert_eq!(cmd.effects().count(), 2);

        assert_eq!(
            app.view(&model).prayers,
            vec![prayer(0, "Mom", None, &[], PrayerStatus::Active)]
        );
    }

    #[test]
    fn persists_json_blob_with_next_id() {
        let app = Implore;
        let mut model = Model::default();

        add(&app, &mut model, "Mom", "", &["family"])
            .expect_render()
            .expect_key_value_with(|op| {
                let KeyValueOperation::Set { key, value } = op else {
                    panic!("expected KeyValue set effect");
                };
                assert_eq!(key, PRAYERS_KEY);

                let stored: StoredState = serde_json::from_slice(value).unwrap();
                assert_eq!(stored.next_id, 1);
                assert_eq!(stored.prayers[0].intention, "Mom");
                assert_eq!(stored.prayers[0].tags, vec!["family".to_string()]);
                assert_eq!(stored.prayers[0].status, PrayerStatus::Active);
                assert_eq!(stored.prayers[0].cadence, IntentionCadence::Unscheduled);
                assert!(!stored.reminder_settings.enabled);
            });
    }

    #[test]
    fn set_reminder_settings_persists_and_plans_digests() {
        let app = Implore;
        let mut model = Model::default();

        let _ = add_with_cadence(&app, &mut model, "Mom", "", &[], IntentionCadence::Daily);
        let _ = app.update(
            Event::SyncLocalTime {
                year: 2026,
                month: 8,
                day: 12,
                hour: 7,
                minute: 0,
            },
            &mut model,
        );
        let _ = app.update(
            Event::SetReminderSettings {
                enabled: true,
                hour: 8,
                minute: 0,
            },
            &mut model,
        );

        let view = app.view(&model);
        assert!(view.reminder_settings.enabled);
        assert!(!view.reminder_digests.is_empty());
        assert_eq!(view.reminder_digests[0].intentions, vec!["Mom".to_string()]);
    }

    #[test]
    fn reminder_digests_omit_intentions_already_prayed_that_day() {
        let app = Implore;
        let mut model = Model::default();

        let _ = add_with_cadence(&app, &mut model, "Mom", "", &[], IntentionCadence::Daily);
        let _ = add_with_cadence(&app, &mut model, "Dad", "", &[], IntentionCadence::Daily);
        let _ = app.update(
            Event::SyncLocalTime {
                year: 2026,
                month: 8,
                day: 12,
                hour: 7,
                minute: 0,
            },
            &mut model,
        );
        let _ = app.update(
            Event::SetReminderSettings {
                enabled: true,
                hour: 8,
                minute: 0,
            },
            &mut model,
        );
        let _ = app.update(Event::LogPrayer { id: 0 }, &mut model);

        let view = app.view(&model);
        assert_eq!(view.reminder_digests[0].day, 12);
        assert_eq!(view.reminder_digests[0].intentions, vec!["Dad".to_string()]);
        assert!(view
            .reminder_digests
            .iter()
            .skip(1)
            .all(|digest| digest.intentions == ["Mom", "Dad"]));
    }

    #[test]
    fn reminder_digests_skip_unscheduled_and_archived() {
        let app = Implore;
        let mut model = Model::default();

        let _ = add_with_cadence(&app, &mut model, "Mom", "", &[], IntentionCadence::Daily);
        let _ = add_with_cadence(
            &app,
            &mut model,
            "Dad",
            "",
            &[],
            IntentionCadence::Unscheduled,
        );
        let _ = add_with_cadence(
            &app,
            &mut model,
            "Parish",
            "",
            &[],
            IntentionCadence::Weekly,
        );
        let _ = app.update(Event::ArchivePrayer { id: 2 }, &mut model);
        let _ = app.update(
            Event::SyncLocalTime {
                year: 2026,
                month: 8,
                day: 16,
                hour: 7,
                minute: 0,
            },
            &mut model,
        );
        let _ = app.update(
            Event::SetReminderSettings {
                enabled: true,
                hour: 8,
                minute: 0,
            },
            &mut model,
        );

        let view = app.view(&model);
        assert_eq!(view.prayers.len(), 2);
        assert_eq!(view.archived_prayers.len(), 1);
        assert!(view
            .reminder_digests
            .iter()
            .all(|digest| digest.intentions == ["Mom"]));
    }

    #[test]
    fn adds_and_persists_daily_and_monthly_cadence() {
        let app = Implore;
        let mut model = Model::default();

        add_with_cadence(&app, &mut model, "Mom", "", &[], IntentionCadence::Daily)
            .expect_render()
            .expect_key_value_with(|op| {
                let KeyValueOperation::Set { value, .. } = op else {
                    panic!("expected KeyValue set effect");
                };
                let stored: StoredState = serde_json::from_slice(value).unwrap();
                assert_eq!(stored.prayers[0].cadence, IntentionCadence::Daily);
            });

        assert_eq!(app.view(&model).prayers[0].cadence, IntentionCadence::Daily);

        add_with_cadence(&app, &mut model, "Dad", "", &[], IntentionCadence::Monthly)
            .expect_render()
            .expect_key_value_with(|op| {
                let KeyValueOperation::Set { value, .. } = op else {
                    panic!("expected KeyValue set effect");
                };
                let stored: StoredState = serde_json::from_slice(value).unwrap();
                assert_eq!(stored.prayers[1].cadence, IntentionCadence::Monthly);
            });
    }

    #[test]
    fn updates_prayer_fields_and_cadence() {
        let app = Implore;
        let mut model = Model::default();
        let _ = add(&app, &mut model, "Mom", "", &["family"]);

        app.update(
            Event::UpdatePrayer {
                id: 0,
                intention: "  Dad  ".into(),
                details: "  recovery  ".into(),
                tags: vec!["  health  ".into(), "  ".into()],
                cadence: IntentionCadence::Weekly,
                saint_id: "st-joseph".into(),
                color: IntentionColor::Sage,
            },
            &mut model,
        )
        .expect_render()
        .expect_key_value_with(|op| {
            let KeyValueOperation::Set { value, .. } = op else {
                panic!("expected KeyValue set effect");
            };
            let stored: StoredState = serde_json::from_slice(value).unwrap();
            assert_eq!(stored.prayers[0].intention, "Dad");
            assert_eq!(stored.prayers[0].details.as_deref(), Some("recovery"));
            assert_eq!(stored.prayers[0].tags, vec!["health".to_string()]);
            assert_eq!(stored.prayers[0].cadence, IntentionCadence::Weekly);
            assert_eq!(stored.prayers[0].status, PrayerStatus::Active);
            assert_eq!(stored.prayers[0].saint_id.as_deref(), Some("st-joseph"));
            assert_eq!(stored.prayers[0].color, IntentionColor::Sage);
        });

        assert_eq!(
            app.view(&model).prayers[0],
            Prayer {
                id: 0,
                intention: "Dad".into(),
                details: Some("recovery".into()),
                tags: vec!["health".into()],
                status: PrayerStatus::Active,
                cadence: IntentionCadence::Weekly,
                saint_id: Some("st-joseph".into()),
                color: IntentionColor::Sage,
                prayed_on: Vec::new(),
            }
        );
    }

    #[test]
    fn log_prayer_persists_and_allows_multiple_entries_same_day() {
        let app = Implore;
        let mut model = Model::default();

        let _ = add(&app, &mut model, "Mom", "", &[]);
        let _ = app.update(
            Event::SyncLocalTime {
                year: 2026,
                month: 8,
                day: 12,
                hour: 9,
                minute: 0,
            },
            &mut model,
        );

        app.update(Event::LogPrayer { id: 0 }, &mut model)
            .expect_render()
            .expect_key_value_with(|op| {
                let KeyValueOperation::Set { value, .. } = op else {
                    panic!("expected KeyValue set effect");
                };
                let stored: StoredState = serde_json::from_slice(value).unwrap();
                assert_eq!(stored.prayers[0].prayed_on.len(), 1);
                assert_eq!(
                    stored.prayers[0].prayed_on[0],
                    PrayerLogEntry {
                        year: 2026,
                        month: 8,
                        day: 12,
                        hour: 9,
                        minute: 0,
                    }
                );
            });

        app.update(Event::LogPrayer { id: 0 }, &mut model)
            .expect_render()
            .expect_key_value_with(|op| {
                let KeyValueOperation::Set { value, .. } = op else {
                    panic!("expected KeyValue set effect");
                };
                let stored: StoredState = serde_json::from_slice(value).unwrap();
                assert_eq!(stored.prayers[0].prayed_on.len(), 2);
            });

        assert_eq!(app.view(&model).prayers[0].prayed_on.len(), 2);
    }

    #[test]
    fn remove_prayer_log_entry_removes_one() {
        let app = Implore;
        let mut model = Model::default();

        let _ = add(&app, &mut model, "Mom", "", &[]);
        let _ = app.update(
            Event::SyncLocalTime {
                year: 2026,
                month: 8,
                day: 12,
                hour: 9,
                minute: 0,
            },
            &mut model,
        );
        let _ = app.update(Event::LogPrayer { id: 0 }, &mut model);
        let _ = app.update(Event::LogPrayer { id: 0 }, &mut model);
        assert_eq!(app.view(&model).prayers[0].prayed_on.len(), 2);

        app.update(Event::RemovePrayerLogEntry { id: 0, index: 0 }, &mut model)
            .expect_render()
            .expect_key_value_with(|op| {
                let KeyValueOperation::Set { value, .. } = op else {
                    panic!("expected KeyValue set effect");
                };
                let stored: StoredState = serde_json::from_slice(value).unwrap();
                assert_eq!(stored.prayers[0].prayed_on.len(), 1);
            });
    }

    #[test]
    fn log_prayer_ignored_for_archived_or_without_local_time() {
        let app = Implore;
        let mut model = Model::default();

        let _ = add(&app, &mut model, "Mom", "", &[]);
        let _ = app.update(Event::ArchivePrayer { id: 0 }, &mut model);

        app.update(Event::LogPrayer { id: 0 }, &mut model)
            .expect_only_render();

        let _ = app.update(Event::UnarchivePrayer { id: 0 }, &mut model);
        app.update(Event::LogPrayer { id: 0 }, &mut model)
            .expect_only_render();
        assert!(app.view(&model).prayers[0].prayed_on.is_empty());
    }

    #[test]
    fn update_empty_intention_or_unknown_id_is_noop() {
        let app = Implore;
        let mut model = Model::default();
        let _ = add(&app, &mut model, "Mom", "", &[]);

        app.update(
            Event::UpdatePrayer {
                id: 0,
                intention: "   ".into(),
                details: "note".into(),
                tags: vec![],
                cadence: IntentionCadence::Daily,
                saint_id: String::new(),
                color: IntentionColor::Sky,
            },
            &mut model,
        )
        .expect_only_render();

        app.update(
            Event::UpdatePrayer {
                id: 99,
                intention: "Dad".into(),
                details: String::new(),
                tags: vec![],
                cadence: IntentionCadence::Daily,
                saint_id: String::new(),
                color: IntentionColor::Sky,
            },
            &mut model,
        )
        .expect_only_render();

        assert_eq!(
            app.view(&model).prayers,
            vec![prayer(0, "Mom", None, &[], PrayerStatus::Active)]
        );
    }

    #[test]
    fn liturgical_day_follows_synced_local_date() {
        let app = Implore;
        let mut model = Model::default();
        assert_eq!(app.view(&model).liturgical_day, None);

        app.update(
            Event::SyncLocalTime {
                year: 2026,
                month: 8,
                day: 13,
                hour: 8,
                minute: 0,
            },
            &mut model,
        )
        .expect_only_render();

        assert_eq!(
            app.view(&model).liturgical_day,
            Some(LiturgicalDay::OrdinaryTime {
                week: 19,
                weekday: 5
            })
        );
    }

    #[test]
    fn today_prayers_empty_until_local_time_then_due_only() {
        let app = Implore;
        let mut model = Model::default();

        let _ = add_with_cadence(&app, &mut model, "Mom", "", &[], IntentionCadence::Daily);
        let _ = add_with_cadence(
            &app,
            &mut model,
            "Parish",
            "",
            &[],
            IntentionCadence::Weekly,
        );
        let _ = add(&app, &mut model, "Dad", "", &[]);

        assert!(app.view(&model).today_prayers.is_empty());

        app.update(
            Event::SyncLocalTime {
                year: 2026,
                month: 8,
                day: 12,
                hour: 8,
                minute: 0,
            },
            &mut model,
        )
        .expect_only_render();

        let today = app.view(&model).today_prayers;
        assert_eq!(today.len(), 1);
        assert_eq!(today[0].prayer.intention, "Mom");
        assert!(!today[0].prayed_today);
        assert_eq!(
            app.view(&model).local_date,
            Some(reminder::CivilDate {
                year: 2026,
                month: 8,
                day: 12
            })
        );
    }

    #[test]
    fn today_prayers_marks_prayed_today_from_local_date() {
        let app = Implore;
        let mut model = Model::default();

        let _ = add_with_cadence(&app, &mut model, "Mom", "", &[], IntentionCadence::Daily);

        app.update(
            Event::SyncLocalTime {
                year: 2026,
                month: 8,
                day: 12,
                hour: 8,
                minute: 0,
            },
            &mut model,
        )
        .expect_only_render();

        app.update(Event::LogPrayer { id: 0 }, &mut model)
            .expect_render()
            .expect_key_value_with(|op| {
                let KeyValueOperation::Set { value, .. } = op else {
                    panic!("expected KeyValue set effect");
                };
                let stored: StoredState = serde_json::from_slice(value).unwrap();
                assert_eq!(stored.prayers[0].prayed_on.len(), 1);
            });

        let today = app.view(&model).today_prayers;
        assert_eq!(today.len(), 1);
        assert!(today[0].prayed_today);
    }
}
