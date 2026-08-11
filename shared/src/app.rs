#[derive(Default)]
pub struct Implore;

use crux_core::{
    App, Command,
    macros::effect,
    render::{RenderOperation, render},
};

use facet::Facet;
use serde::{Deserialize, Serialize};

impl App for Implore {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::AddPrayer {
                intention,
                details,
                tags,
            } => {
                let intention = intention.trim().to_string();
                if !intention.is_empty() {
                    let details = trim_optional(details);
                    let tags = normalize_tags(tags);
                    let id = model.next_id;
                    model.next_id += 1;
                    model.prayers.push(Prayer {
                        id,
                        intention,
                        details,
                        tags,
                    });
                }
            }
            Event::RemovePrayer { id } => {
                model.prayers.retain(|prayer| prayer.id != id);
            }
        }

        render()
    }

    fn view(&self, model: &Model) -> ViewModel {
        ViewModel {
            prayers: model.prayers.clone(),
        }
    }
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

#[derive(Facet, Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub enum Event {
    AddPrayer {
        intention: String,
        details: String,
        tags: Vec<String>,
    },
    RemovePrayer {
        id: u64,
    },
}

#[derive(Default)]
pub struct Model {
    prayers: Vec<Prayer>,
    next_id: u64,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Prayer {
    pub id: u64,
    pub intention: String,
    pub details: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Default)]
pub struct ViewModel {
    pub prayers: Vec<Prayer>,
}

#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
}

#[cfg(test)]
mod test {
    use super::*;

    fn add(
        app: &Implore,
        model: &mut Model,
        intention: &str,
        details: &str,
        tags: &[&str],
    ) {
        app.update(
            Event::AddPrayer {
                intention: intention.into(),
                details: details.into(),
                tags: tags.iter().map(|tag| (*tag).into()).collect(),
            },
            model,
        )
        .expect_only_render();
    }

    #[test]
    fn renders() {
        let app = Implore;
        let mut model = Model::default();
        add(&app, &mut model, "Mom", "", &[]);
    }

    #[test]
    fn shows_empty_list_initially() {
        let app = Implore;
        let model = Model::default();

        assert!(app.view(&model).prayers.is_empty());
    }

    #[test]
    fn adds_prayer_with_optional_fields() {
        let app = Implore;
        let mut model = Model::default();

        add(
            &app,
            &mut model,
            "Mom",
            "Surgery recovery",
            &["family", "health"],
        );

        assert_eq!(
            app.view(&model).prayers,
            vec![Prayer {
                id: 0,
                intention: "Mom".into(),
                details: Some("Surgery recovery".into()),
                tags: vec!["family".into(), "health".into()],
            }]
        );
    }

    #[test]
    fn adds_prayer_without_optional_fields() {
        let app = Implore;
        let mut model = Model::default();

        add(&app, &mut model, "Mom", "", &[]);

        assert_eq!(
            app.view(&model).prayers,
            vec![Prayer {
                id: 0,
                intention: "Mom".into(),
                details: None,
                tags: vec![],
            }]
        );
    }

    #[test]
    fn ignores_empty_or_whitespace_intention() {
        let app = Implore;
        let mut model = Model::default();

        add(&app, &mut model, "   ", "note", &["tag"]);
        add(&app, &mut model, "", "", &[]);

        assert!(app.view(&model).prayers.is_empty());
    }

    #[test]
    fn trims_fields_and_drops_empty_tags() {
        let app = Implore;
        let mut model = Model::default();

        add(
            &app,
            &mut model,
            "  Dad  ",
            "  recovery  ",
            &["  family  ", "  ", "health"],
        );

        assert_eq!(
            app.view(&model).prayers,
            vec![Prayer {
                id: 0,
                intention: "Dad".into(),
                details: Some("recovery".into()),
                tags: vec!["family".into(), "health".into()],
            }]
        );
    }

    #[test]
    fn removes_prayer_by_id() {
        let app = Implore;
        let mut model = Model::default();

        add(&app, &mut model, "Mom", "", &[]);
        add(&app, &mut model, "Dad", "", &[]);

        app.update(Event::RemovePrayer { id: 0 }, &mut model)
            .expect_only_render();

        assert_eq!(
            app.view(&model).prayers,
            vec![Prayer {
                id: 1,
                intention: "Dad".into(),
                details: None,
                tags: vec![],
            }]
        );
    }

    #[test]
    fn remove_unknown_id_is_noop() {
        let app = Implore;
        let mut model = Model::default();

        add(&app, &mut model, "Mom", "", &[]);

        app.update(Event::RemovePrayer { id: 99 }, &mut model)
            .expect_only_render();

        assert_eq!(
            app.view(&model).prayers,
            vec![Prayer {
                id: 0,
                intention: "Mom".into(),
                details: None,
                tags: vec![],
            }]
        );
    }
}
