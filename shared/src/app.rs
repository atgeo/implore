#[derive(Default)]
pub struct Implore;

use crux_core::{
    App, Command,
    macros::effect,
    render::{RenderOperation, render},
};
use crux_kv::{KeyValueError, KeyValueOperation, command::KeyValue};
use facet::Facet;
use serde::{Deserialize, Serialize};

const PRAYERS_KEY: &str = "prayers";

impl App for Implore {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Restore => KeyValue::get(PRAYERS_KEY).then_send(Event::PrayersLoaded),
            Event::PrayersLoaded(result) => {
                if let Ok(Some(bytes)) = result
                    && let Ok(stored) = serde_json::from_slice::<StoredPrayers>(&bytes)
                {
                    model.prayers = stored.prayers;
                    model.next_id = stored.next_id;
                }
                render()
            }
            Event::AddPrayer {
                intention,
                details,
                tags,
            } => {
                let intention = intention.trim().to_string();
                if intention.is_empty() {
                    return render();
                }

                let details = trim_optional(details);
                let tags = normalize_tags(tags);
                let id = model.next_id;
                model.next_id += 1;
                model.prayers.push(Prayer {
                    id,
                    intention,
                    details,
                    tags,
                    status: PrayerStatus::Active,
                });

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
            Event::ArchivePrayer { id } => set_status(model, id, PrayerStatus::Archived),
            Event::UnarchivePrayer { id } => set_status(model, id, PrayerStatus::Active),
            Event::SetFilter { filter } => {
                model.filter = filter;
                render()
            }
            Event::Persisted(_) => Command::done(),
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        ViewModel {
            filter: model.filter,
            prayers: model
                .prayers
                .iter()
                .filter(|prayer| model.filter.matches(prayer.status))
                .cloned()
                .collect(),
        }
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
    let stored = StoredPrayers {
        prayers: model.prayers.clone(),
        next_id: model.next_id,
    };
    let bytes = serde_json::to_vec(&stored).unwrap_or_default();
    KeyValue::set(PRAYERS_KEY, bytes).then_send(Event::Persisted)
}

fn trim_optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
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
pub enum IntentionFilter {
    #[default]
    Active,
    Archived,
    All,
}

impl IntentionFilter {
    const fn matches(self, status: PrayerStatus) -> bool {
        match self {
            Self::All => true,
            Self::Active => matches!(status, PrayerStatus::Active),
            Self::Archived => matches!(status, PrayerStatus::Archived),
        }
    }
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
    },
    RemovePrayer {
        id: u64,
    },
    ArchivePrayer {
        id: u64,
    },
    UnarchivePrayer {
        id: u64,
    },
    SetFilter {
        filter: IntentionFilter,
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

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Model {
    prayers: Vec<Prayer>,
    next_id: u64,
    filter: IntentionFilter,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct StoredPrayers {
    prayers: Vec<Prayer>,
    next_id: u64,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Prayer {
    pub id: u64,
    pub intention: String,
    pub details: Option<String>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub status: PrayerStatus,
}

#[derive(Facet, Serialize, Deserialize, Clone, Default)]
pub struct ViewModel {
    pub filter: IntentionFilter,
    pub prayers: Vec<Prayer>,
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
        }
    }

    fn add(
        app: &Implore,
        model: &mut Model,
        intention: &str,
        details: &str,
        tags: &[&str],
    ) -> Command<Effect, Event> {
        app.update(
            Event::AddPrayer {
                intention: intention.into(),
                details: details.into(),
                tags: tags.iter().map(|tag| (*tag).into()).collect(),
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

        let stored = StoredPrayers {
            prayers: vec![prayer(3, "Mom", None, &[], PrayerStatus::Active)],
            next_id: 4,
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
    fn loads_missing_status_as_active() {
        let app = Implore;
        let mut model = Model::default();

        let bytes = br#"{"prayers":[{"id":1,"intention":"Mom","details":null,"tags":[]}],"next_id":2}"#;

        app.update(Event::PrayersLoaded(Ok(Some(bytes.to_vec()))), &mut model)
            .expect_only_render();

        assert_eq!(model.prayers[0].status, PrayerStatus::Active);
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
        assert_eq!(app.view(&model).filter, IntentionFilter::Active);
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
    fn archives_prayer_and_hides_from_active_filter() {
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
        assert_eq!(model.prayers[0].status, PrayerStatus::Archived);
    }

    #[test]
    fn filter_archived_and_all() {
        let app = Implore;
        let mut model = Model::default();

        let _ = add(&app, &mut model, "Mom", "", &[]);
        let _ = add(&app, &mut model, "Dad", "", &[]);
        let _ = app.update(Event::ArchivePrayer { id: 0 }, &mut model);

        app.update(
            Event::SetFilter {
                filter: IntentionFilter::Archived,
            },
            &mut model,
        )
        .expect_only_render();

        assert_eq!(
            app.view(&model).prayers,
            vec![prayer(0, "Mom", None, &[], PrayerStatus::Archived)]
        );

        app.update(
            Event::SetFilter {
                filter: IntentionFilter::All,
            },
            &mut model,
        )
        .expect_only_render();

        assert_eq!(app.view(&model).prayers.len(), 2);
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

                let stored: StoredPrayers = serde_json::from_slice(value).unwrap();
                assert_eq!(stored.next_id, 1);
                assert_eq!(stored.prayers[0].intention, "Mom");
                assert_eq!(stored.prayers[0].tags, vec!["family".to_string()]);
                assert_eq!(stored.prayers[0].status, PrayerStatus::Active);
            });
    }
}
