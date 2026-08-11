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
            Event::AddPrayer { text } => {
                let text = text.trim().to_string();
                if !text.is_empty() {
                    let id = model.next_id;
                    model.next_id += 1;
                    model.prayers.push(Prayer { id, text });
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

#[derive(Facet, Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub enum Event {
    AddPrayer { text: String },
    RemovePrayer { id: u64 },
}

#[derive(Default)]
pub struct Model {
    prayers: Vec<Prayer>,
    next_id: u64,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Prayer {
    pub id: u64,
    pub text: String,
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

    #[test]
    fn renders() {
        let app = Implore;
        let mut model = Model::default();

        app.update(
            Event::AddPrayer {
                text: "Mom".into(),
            },
            &mut model,
        )
        .expect_only_render();
    }

    #[test]
    fn shows_empty_list_initially() {
        let app = Implore;
        let model = Model::default();

        assert!(app.view(&model).prayers.is_empty());
    }

    #[test]
    fn adds_prayer() {
        let app = Implore;
        let mut model = Model::default();

        app.update(
            Event::AddPrayer {
                text: "Mom".into(),
            },
            &mut model,
        )
        .expect_only_render();

        assert_eq!(
            app.view(&model).prayers,
            vec![Prayer {
                id: 0,
                text: "Mom".into(),
            }]
        );
    }

    #[test]
    fn ignores_empty_or_whitespace_prayer() {
        let app = Implore;
        let mut model = Model::default();

        app.update(
            Event::AddPrayer {
                text: "   ".into(),
            },
            &mut model,
        )
        .expect_only_render();
        app.update(
            Event::AddPrayer {
                text: String::new(),
            },
            &mut model,
        )
        .expect_only_render();

        assert!(app.view(&model).prayers.is_empty());
    }

    #[test]
    fn trims_prayer_text() {
        let app = Implore;
        let mut model = Model::default();

        app.update(
            Event::AddPrayer {
                text: "  Dad  ".into(),
            },
            &mut model,
        )
        .expect_only_render();

        assert_eq!(
            app.view(&model).prayers,
            vec![Prayer {
                id: 0,
                text: "Dad".into(),
            }]
        );
    }

    #[test]
    fn removes_prayer_by_id() {
        let app = Implore;
        let mut model = Model::default();

        let _ = app.update(
            Event::AddPrayer {
                text: "Mom".into(),
            },
            &mut model,
        );
        let _ = app.update(
            Event::AddPrayer {
                text: "Dad".into(),
            },
            &mut model,
        );

        app.update(Event::RemovePrayer { id: 0 }, &mut model)
            .expect_only_render();

        assert_eq!(
            app.view(&model).prayers,
            vec![Prayer {
                id: 1,
                text: "Dad".into(),
            }]
        );
    }

    #[test]
    fn remove_unknown_id_is_noop() {
        let app = Implore;
        let mut model = Model::default();

        let _ = app.update(
            Event::AddPrayer {
                text: "Mom".into(),
            },
            &mut model,
        );

        app.update(Event::RemovePrayer { id: 99 }, &mut model)
            .expect_only_render();

        assert_eq!(
            app.view(&model).prayers,
            vec![Prayer {
                id: 0,
                text: "Mom".into(),
            }]
        );
    }
}
