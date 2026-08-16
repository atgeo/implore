#[derive(Default)]
pub struct Implore;

use crux_core::{
    macros::effect,
    render::{render, RenderOperation},
    App, Command,
};
use crux_http::command::Http;
use crux_http::protocol::HttpRequest;
use crux_kv::{command::KeyValue, KeyValueError, KeyValueOperation};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::account::{
    auth_error_message, auth_url, email_looks_valid, normalize_email, session_expired_message,
    AccountOperation, AccountStatus, AuthRequest, AuthResponse, Session, API_BASE_URL,
    MIN_PASSWORD_LEN, SESSION_KEY,
};
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
            Event::Restore => KeyValue::get(PRAYERS_KEY)
                .then_send(Event::PrayersLoaded)
                .and(KeyValue::get(SESSION_KEY).then_send(Event::SessionLoaded)),
            Event::PrayersLoaded(result) => {
                if let Ok(Some(bytes)) = result {
                    if let Ok(stored) = serde_json::from_slice::<StoredState>(&bytes) {
                        model.prayers = stored.prayers;
                        model.next_id = stored.next_id;
                        model.reminder_settings = stored.reminder_settings;
                        model.updated_at = stored.updated_at;
                    }
                }
                render()
            }
            Event::SessionLoaded(result) => {
                if let Ok(Some(bytes)) = result {
                    if let Ok(session) = serde_json::from_slice::<Session>(&bytes) {
                        model.last_synced_at = session.last_synced_at;
                        model.session = Some(session);
                        model.account_status = AccountStatus::SignedIn;
                        model.account_operation = AccountOperation::Idle;
                        model.account_error = None;
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
                novena_start,
            } => {
                let intention = normalize_intention(intention);
                if intention.is_empty() {
                    return render();
                }

                let details = normalize_details(details);
                let saint_id = trim_optional(saint_id);
                let tags = normalize_tags(tags);
                let novena_start = resolve_novena_start(cadence, novena_start);
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
                    novena_start,
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
                novena_start,
            } => {
                let intention = normalize_intention(intention);
                if intention.is_empty() {
                    return render();
                }

                let details = normalize_details(details);
                let saint_id = trim_optional(saint_id);
                let tags = normalize_tags(tags);
                let novena_start = resolve_novena_start(cadence, novena_start);
                let Some(prayer) = model.prayers.iter_mut().find(|prayer| prayer.id == id) else {
                    return render();
                };
                if prayer.intention == intention
                    && prayer.details == details
                    && prayer.tags == tags
                    && prayer.cadence == cadence
                    && prayer.saint_id == saint_id
                    && prayer.color == color
                    && prayer.novena_start == novena_start
                {
                    return render();
                }

                prayer.intention = intention;
                prayer.details = details;
                prayer.tags = tags;
                prayer.cadence = cadence;
                prayer.saint_id = saint_id;
                prayer.color = color;
                prayer.novena_start = novena_start;
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
                render().and(persist_after_mutation(model))
            }
            Event::SyncLocalTime {
                year,
                month,
                day,
                hour,
                minute,
                unix_seconds,
            } => {
                let today = reminder::CivilDate {
                    year: i32::from(year),
                    month: u32::from(month),
                    day: u32::from(day),
                };
                model.local_now = Some(CivilDateTime {
                    date: today,
                    hour: u32::from(hour),
                    minute: u32::from(minute),
                });
                model.unix_seconds = Some(unix_seconds);
                model.calendar_date = Some(match model.calendar_date {
                    Some(selected) => reminder::clamp_calendar_date(selected, today),
                    None => today,
                });
                render()
            }
            Event::SelectCalendarDate { year, month, day } => {
                let date = reminder::CivilDate {
                    year: i32::from(year),
                    month: u32::from(month),
                    day: u32::from(day),
                };
                model.calendar_date = Some(match model.local_now {
                    Some(now) => reminder::clamp_calendar_date(date, now.date),
                    None => date,
                });
                render()
            }
            Event::LogPrayer { id } => log_prayer(model, id),
            Event::RemovePrayerLogEntry { id, index } => {
                remove_prayer_log_entry(model, id, index as usize)
            }
            Event::SignUp { email, password } => begin_auth(model, email, password, true),
            Event::SignIn { email, password } => begin_auth(model, email, password, false),
            Event::SignOut => sign_out(model),
            Event::DismissAccountError => dismiss_account_error(model),
            Event::SyncRequested => begin_sync(model),
            Event::AuthCompleted { email, result } => auth_completed(model, email, result),
            Event::SyncGetCompleted(result) => sync_get_completed(model, result),
            Event::SyncPutCompleted(result) => sync_put_completed(model, result),
            Event::SessionPersisted(_) => Command::done(),
            Event::SignOutCompleted(_) => Command::done(),
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
                .map(|now| day_prayers(&model.prayers, now.date))
                .unwrap_or_default(),
            local_date: model.local_now.map(|now| now.date),
            reminder_settings: model.reminder_settings,
            reminder_digests,
            liturgical_day: model.local_now.map(|now| liturgical_day_for(now.date)),
            calendar_date: model.calendar_date,
            calendar_min_date: model
                .local_now
                .map(|now| reminder::calendar_range(now.date).0),
            calendar_max_date: model
                .local_now
                .map(|now| reminder::calendar_range(now.date).1),
            calendar_liturgical_day: model.calendar_date.map(liturgical_day_for),
            calendar_prayers: model
                .calendar_date
                .map(|date| day_prayers(&model.prayers, date))
                .unwrap_or_default(),
            max_tags: MAX_TAGS as u8,
            max_tag_len: MAX_TAG_LEN as u8,
            max_intention_len: MAX_INTENTION_LEN as u8,
            max_details_len: MAX_DETAILS_LEN as u16,
            account_status: model.account_status.clone(),
            signed_in_email: model
                .session
                .as_ref()
                .map(|session| session.email.clone())
                .unwrap_or_default(),
            last_synced_at: model.last_synced_at,
            account_error: model.account_error.clone().unwrap_or_default(),
            account_operation: model.account_operation.clone(),
            api_base_url: API_BASE_URL.to_string(),
        }
    }
}

fn day_prayers(prayers: &[Prayer], date: reminder::CivilDate) -> Vec<TodayPrayer> {
    reminder::select_today(prayers, date)
        .into_iter()
        .map(|prayer| {
            let prayed_today = prayer_log::prayed_on_date(&prayer.prayed_on, date);
            TodayPrayer {
                prayer,
                prayed_today,
            }
        })
        .collect()
}

fn is_account_busy(model: &Model) -> bool {
    matches!(
        model.account_status,
        AccountStatus::SigningIn | AccountStatus::Syncing
    )
}

fn begin_auth(
    model: &mut Model,
    email: String,
    password: String,
    sign_up: bool,
) -> Command<Effect, Event> {
    if is_account_busy(model) || model.session.is_some() {
        return Command::done();
    }

    let email = normalize_email(email);
    let password = password.trim().to_string();
    model.account_operation = if sign_up {
        AccountOperation::SignUp
    } else {
        AccountOperation::SignIn
    };

    if email.is_empty() || password.is_empty() {
        model.account_status = AccountStatus::Error;
        model.account_error = Some("Email and password are required".into());
        return render();
    }
    if !email_looks_valid(&email) {
        model.account_status = AccountStatus::Error;
        model.account_error = Some("Enter a valid email address".into());
        return render();
    }
    if sign_up && password.len() < MIN_PASSWORD_LEN {
        model.account_status = AccountStatus::Error;
        model.account_error = Some(format!(
            "Password must be at least {MIN_PASSWORD_LEN} characters"
        ));
        return render();
    }

    model.account_status = AccountStatus::SigningIn;
    model.account_error = None;

    let path = if sign_up {
        "/auth/sign-up"
    } else {
        "/auth/sign-in"
    };
    let body = AuthRequest {
        email: email.clone(),
        password,
    };

    render().and(
        Http::post(auth_url(path))
            .body_json(&body)
            .expect("auth body")
            .expect_json::<AuthResponse>()
            .build()
            .then_send(move |result| Event::AuthCompleted { email, result }),
    )
}

fn auth_completed(
    model: &mut Model,
    email: String,
    result: crux_http::Result<crux_http::Response<AuthResponse>>,
) -> Command<Effect, Event> {
    if model.account_status != AccountStatus::SigningIn {
        return Command::done();
    }

    match result {
        Ok(mut response) => {
            let Some(body) = response.take_body() else {
                model.account_status = AccountStatus::Error;
                model.account_error = Some("Could not complete sign-in. Try again.".into());
                return render();
            };
            if body.token.is_empty() || body.user_id.is_empty() {
                model.account_status = AccountStatus::Error;
                model.account_error = Some("Could not complete sign-in. Try again.".into());
                return render();
            }
            let session = Session {
                user_id: body.user_id,
                token: body.token,
                email,
                last_synced_at: None,
            };
            model.session = Some(session.clone());
            model.last_synced_at = None;
            model.account_status = AccountStatus::SignedIn;
            model.account_operation = AccountOperation::Idle;
            model.account_error = None;
            render().and(persist_session(&session))
        }
        Err(error) => {
            model.session = None;
            model.account_status = AccountStatus::Error;
            model.account_error = Some(auth_error_message(&error));
            render()
        }
    }
}

fn sign_out(model: &mut Model) -> Command<Effect, Event> {
    let token = model.session.as_ref().map(|session| session.token.clone());
    model.session = None;
    model.account_status = AccountStatus::SignedOut;
    model.account_operation = AccountOperation::Idle;
    model.account_error = None;
    model.last_synced_at = None;

    let clear = render().and(KeyValue::delete(SESSION_KEY).then_send(Event::SessionPersisted));
    match token {
        Some(token) => clear.and(
            Http::post(auth_url("/auth/sign-out"))
                .header("Authorization", format!("Bearer {token}"))
                .build()
                .then_send(Event::SignOutCompleted),
        ),
        None => clear,
    }
}

fn dismiss_account_error(model: &mut Model) -> Command<Effect, Event> {
    model.account_error = None;
    model.account_operation = AccountOperation::Idle;
    if model.account_status == AccountStatus::Error {
        model.account_status = if model.session.is_some() {
            AccountStatus::SignedIn
        } else {
            AccountStatus::SignedOut
        };
    }
    render()
}

fn begin_sync(model: &mut Model) -> Command<Effect, Event> {
    if model.account_status == AccountStatus::Syncing {
        return Command::done();
    }
    if is_account_busy(model) {
        return Command::done();
    }

    let Some(session) = model.session.clone() else {
        model.account_status = AccountStatus::Error;
        model.account_operation = AccountOperation::Sync;
        model.account_error = Some("Sign in to sync".into());
        return render();
    };

    model.account_status = AccountStatus::Syncing;
    model.account_operation = AccountOperation::Sync;
    model.account_error = None;

    render().and(
        Http::get(auth_url("/sync"))
            .header("Authorization", format!("Bearer {}", session.token))
            .expect_json::<StoredState>()
            .build()
            .then_send(Event::SyncGetCompleted),
    )
}

fn sync_get_completed(
    model: &mut Model,
    result: crux_http::Result<crux_http::Response<StoredState>>,
) -> Command<Effect, Event> {
    if model.session.is_none() || model.account_status != AccountStatus::Syncing {
        return Command::done();
    }

    match result {
        Ok(mut response) => {
            let Some(remote) = response.take_body() else {
                return push_sync(model);
            };
            if remote.updated_at > model.updated_at {
                apply_stored_state(model, remote);
                finish_sync_success(model)
            } else {
                push_sync(model)
            }
        }
        Err(error) if error.code() == Some(404) => push_sync(model),
        Err(error) if error.code() == Some(401) => expire_session(model),
        Err(error) => {
            model.account_status = AccountStatus::Error;
            model.account_error = Some(auth_error_message(&error));
            render()
        }
    }
}

fn push_sync(model: &mut Model) -> Command<Effect, Event> {
    if model.account_status != AccountStatus::Syncing {
        return Command::done();
    }
    let Some(session) = model.session.clone() else {
        return Command::done();
    };

    touch_updated_at(model);
    let body = stored_state_from_model(model);

    Http::put(auth_url("/sync"))
        .header("Authorization", format!("Bearer {}", session.token))
        .body_json(&body)
        .expect("sync body")
        .expect_json::<StoredState>()
        .build()
        .then_send(Event::SyncPutCompleted)
}

fn sync_put_completed(
    model: &mut Model,
    result: crux_http::Result<crux_http::Response<StoredState>>,
) -> Command<Effect, Event> {
    if model.session.is_none() || model.account_status != AccountStatus::Syncing {
        return Command::done();
    }

    match result {
        Ok(mut response) => {
            if let Some(remote) = response.take_body() {
                model.updated_at = remote.updated_at.max(model.updated_at);
            }
            finish_sync_success(model)
        }
        Err(error) if error.code() == Some(401) => expire_session(model),
        Err(error) => {
            model.account_status = AccountStatus::Error;
            model.account_error = Some(auth_error_message(&error));
            render()
        }
    }
}

fn finish_sync_success(model: &mut Model) -> Command<Effect, Event> {
    model.last_synced_at = model.unix_seconds;
    if let Some(session) = model.session.as_mut() {
        session.last_synced_at = model.last_synced_at;
    }
    model.account_status = AccountStatus::SignedIn;
    model.account_operation = AccountOperation::Idle;
    model.account_error = None;
    let Some(session) = model.session.clone() else {
        return render().and(persist_state(model));
    };
    render()
        .and(persist_state(model))
        .and(persist_session(&session))
}

fn expire_session(model: &mut Model) -> Command<Effect, Event> {
    model.session = None;
    model.last_synced_at = None;
    model.account_status = AccountStatus::Error;
    model.account_operation = AccountOperation::SignIn;
    model.account_error = Some(session_expired_message());
    render().and(KeyValue::delete(SESSION_KEY).then_send(Event::SessionPersisted))
}

fn apply_stored_state(model: &mut Model, stored: StoredState) {
    model.prayers = stored.prayers;
    model.next_id = stored.next_id;
    model.reminder_settings = stored.reminder_settings;
    model.updated_at = stored.updated_at;
}

fn stored_state_from_model(model: &Model) -> StoredState {
    StoredState {
        prayers: model.prayers.clone(),
        next_id: model.next_id,
        reminder_settings: model.reminder_settings,
        updated_at: model.updated_at,
    }
}

fn touch_updated_at(model: &mut Model) {
    if let Some(unix) = model.unix_seconds {
        if unix >= model.updated_at {
            model.updated_at = unix;
        } else {
            model.updated_at += 1;
        }
    } else {
        model.updated_at = model.updated_at.saturating_add(1);
    }
}

fn persist_session(session: &Session) -> Command<Effect, Event> {
    let bytes = serde_json::to_vec(session).unwrap_or_default();
    KeyValue::set(SESSION_KEY, bytes).then_send(Event::SessionPersisted)
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
    render().and(persist_after_mutation(model))
}

fn remove_prayer_log_entry(model: &mut Model, id: u64, index: usize) -> Command<Effect, Event> {
    let Some(prayer) = model.prayers.iter_mut().find(|prayer| prayer.id == id) else {
        return render();
    };

    if prayer_log::remove_entry(&mut prayer.prayed_on, index) {
        render().and(persist_after_mutation(model))
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

fn persist_prayers(model: &mut Model) -> Command<Effect, Event> {
    persist_after_mutation(model)
}

fn persist_state(model: &Model) -> Command<Effect, Event> {
    let bytes = serde_json::to_vec(&stored_state_from_model(model)).unwrap_or_default();
    KeyValue::set(PRAYERS_KEY, bytes).then_send(Event::Persisted)
}

/// Persist after a local mutation, bumping [`Model::updated_at`] for LWW sync.
fn persist_after_mutation(model: &mut Model) -> Command<Effect, Event> {
    touch_updated_at(model);
    persist_state(model)
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

fn resolve_novena_start(
    cadence: IntentionCadence,
    start: Option<reminder::CivilDate>,
) -> Option<reminder::CivilDate> {
    match cadence {
        IntentionCadence::Novena => start,
        _ => None,
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
    Novena,
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
        novena_start: Option<reminder::CivilDate>,
    },
    UpdatePrayer {
        id: u64,
        intention: String,
        details: String,
        tags: Vec<String>,
        cadence: IntentionCadence,
        saint_id: String,
        color: IntentionColor,
        novena_start: Option<reminder::CivilDate>,
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
        /// Unix seconds (UTC) from the shell clock; used for sync LWW.
        unix_seconds: u64,
    },
    /// Selected day on the in-app calendar (±1 year from local today).
    SelectCalendarDate {
        year: u16,
        month: u8,
        day: u8,
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
    /// Create an account on the sync API.
    SignUp {
        email: String,
        password: String,
    },
    /// Sign in to an existing account.
    SignIn {
        email: String,
        password: String,
    },
    /// Clear the local session (does not delete intentions).
    SignOut,
    /// Clear a stale account error (leaving Create Account, etc.).
    DismissAccountError,
    /// Pull/push the sync document (LWW on `updated_at`).
    SyncRequested,
    /// Internal: shell finished reading the prayers blob.
    #[serde(skip)]
    #[facet(skip)]
    PrayersLoaded(#[facet(opaque)] Result<Option<Vec<u8>>, KeyValueError>),
    /// Internal: shell finished reading the session blob.
    #[serde(skip)]
    #[facet(skip)]
    SessionLoaded(#[facet(opaque)] Result<Option<Vec<u8>>, KeyValueError>),
    /// Internal: shell finished writing the prayers blob.
    #[serde(skip)]
    #[facet(skip)]
    Persisted(#[facet(opaque)] Result<Option<Vec<u8>>, KeyValueError>),
    /// Internal: shell finished writing or deleting the session blob.
    #[serde(skip)]
    #[facet(skip)]
    SessionPersisted(#[facet(opaque)] Result<Option<Vec<u8>>, KeyValueError>),
    /// Internal: auth HTTP response.
    #[serde(skip)]
    #[facet(skip)]
    AuthCompleted {
        email: String,
        #[facet(opaque)]
        result: crux_http::Result<crux_http::Response<AuthResponse>>,
    },
    /// Internal: GET /sync response.
    #[serde(skip)]
    #[facet(skip)]
    SyncGetCompleted(#[facet(opaque)] crux_http::Result<crux_http::Response<StoredState>>),
    /// Internal: PUT /sync response.
    #[serde(skip)]
    #[facet(skip)]
    SyncPutCompleted(#[facet(opaque)] crux_http::Result<crux_http::Response<StoredState>>),
    /// Internal: best-effort server session revoke finished.
    #[serde(skip)]
    #[facet(skip)]
    SignOutCompleted(#[facet(opaque)] crux_http::Result<crux_http::Response<Vec<u8>>>),
}

#[derive(Default, Clone)]
pub struct Model {
    prayers: Vec<Prayer>,
    next_id: u64,
    reminder_settings: ReminderSettings,
    local_now: Option<CivilDateTime>,
    /// Browse selection for the Calendar tab; defaults to local today after sync.
    calendar_date: Option<reminder::CivilDate>,
    session: Option<Session>,
    account_status: AccountStatus,
    account_operation: AccountOperation,
    account_error: Option<String>,
    last_synced_at: Option<u64>,
    /// Document version for LWW sync.
    updated_at: u64,
    /// Latest unix seconds from [`Event::SyncLocalTime`].
    unix_seconds: Option<u64>,
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

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct StoredState {
    pub prayers: Vec<Prayer>,
    pub next_id: u64,
    pub reminder_settings: ReminderSettings,
    #[serde(default)]
    pub updated_at: u64,
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
    /// Start of a classic 9-day novena; only set when [`IntentionCadence::Novena`].
    #[serde(default)]
    pub novena_start: Option<reminder::CivilDate>,
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
    /// Selected day on the Calendar tab.
    pub calendar_date: Option<reminder::CivilDate>,
    /// Earliest selectable calendar day (±1 year from local today).
    pub calendar_min_date: Option<reminder::CivilDate>,
    /// Latest selectable calendar day (±1 year from local today).
    pub calendar_max_date: Option<reminder::CivilDate>,
    /// Temporal cycle for [`Self::calendar_date`].
    pub calendar_liturgical_day: Option<LiturgicalDay>,
    /// Intentions due on [`Self::calendar_date`] (same ordering as today).
    pub calendar_prayers: Vec<TodayPrayer>,
    /// Max tags per intention (shell UX + [`normalize_tags`]).
    pub max_tags: u8,
    /// Max characters per tag (shell UX + [`normalize_tags`]).
    pub max_tag_len: u8,
    /// Max characters in an intention title (shell UX + [`normalize_intention`]).
    pub max_intention_len: u8,
    /// Max characters in optional details (shell UX + [`normalize_details`]).
    pub max_details_len: u16,
    pub account_status: AccountStatus,
    /// Empty when signed out.
    pub signed_in_email: String,
    pub last_synced_at: Option<u64>,
    /// Empty when no account error.
    pub account_error: String,
    pub account_operation: AccountOperation,
    /// Sync API base URL the core will call (for Settings / debugging).
    pub api_base_url: String,
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
            calendar_date: None,
            calendar_min_date: None,
            calendar_max_date: None,
            calendar_liturgical_day: None,
            calendar_prayers: Vec::new(),
            max_tags: MAX_TAGS as u8,
            max_tag_len: MAX_TAG_LEN as u8,
            max_intention_len: MAX_INTENTION_LEN as u8,
            max_details_len: MAX_DETAILS_LEN as u16,
            account_status: AccountStatus::SignedOut,
            signed_in_email: String::new(),
            last_synced_at: None,
            account_error: String::new(),
            account_operation: AccountOperation::Idle,
            api_base_url: API_BASE_URL.to_string(),
        }
    }
}

#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    KeyValue(KeyValueOperation),
    Http(HttpRequest),
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
            novena_start: None,
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
                novena_start: None,
            },
            model,
        )
    }

    #[test]
    fn restore_requests_get() {
        let app = Implore;
        let mut model = Model::default();

        let mut cmd = app.update(Event::Restore, &mut model);
        let mut keys = Vec::new();
        for _ in 0..2 {
            cmd.expect_key_value_with(|op| {
                let KeyValueOperation::Get { key } = op else {
                    panic!("expected get");
                };
                keys.push(key.clone());
            });
        }
        keys.sort();
        assert_eq!(keys, vec![PRAYERS_KEY.to_string(), SESSION_KEY.to_string()]);
    }

    #[test]
    fn loads_persisted_prayers() {
        let app = Implore;
        let mut model = Model::default();

        let stored = StoredState {
            prayers: vec![prayer(3, "Mom", None, &[], PrayerStatus::Active)],
            next_id: 4,
            reminder_settings: ReminderSettings::default(),
            updated_at: 0,
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
                unix_seconds: 1_700_000_000,
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
                unix_seconds: 1_700_000_000,
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
                unix_seconds: 1_700_000_000,
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
    fn adds_and_persists_novena_with_start() {
        let app = Implore;
        let mut model = Model::default();
        let start = reminder::CivilDate {
            year: 2026,
            month: 8,
            day: 12,
        };

        app.update(
            Event::AddPrayer {
                intention: "Peace".into(),
                details: String::new(),
                tags: vec![],
                cadence: IntentionCadence::Novena,
                saint_id: String::new(),
                color: IntentionColor::None,
                novena_start: Some(start),
            },
            &mut model,
        )
        .expect_render()
        .expect_key_value_with(|op| {
            let KeyValueOperation::Set { value, .. } = op else {
                panic!("expected KeyValue set effect");
            };
            let stored: StoredState = serde_json::from_slice(value).unwrap();
            assert_eq!(stored.prayers[0].cadence, IntentionCadence::Novena);
            assert_eq!(stored.prayers[0].novena_start, Some(start));
        });

        assert_eq!(app.view(&model).prayers[0].novena_start, Some(start));
    }

    #[test]
    fn clearing_novena_cadence_clears_start() {
        let app = Implore;
        let mut model = Model::default();
        let start = reminder::CivilDate {
            year: 2026,
            month: 8,
            day: 12,
        };

        let _ = app.update(
            Event::AddPrayer {
                intention: "Peace".into(),
                details: String::new(),
                tags: vec![],
                cadence: IntentionCadence::Novena,
                saint_id: String::new(),
                color: IntentionColor::None,
                novena_start: Some(start),
            },
            &mut model,
        );

        app.update(
            Event::UpdatePrayer {
                id: 0,
                intention: "Peace".into(),
                details: String::new(),
                tags: vec![],
                cadence: IntentionCadence::Daily,
                saint_id: String::new(),
                color: IntentionColor::None,
                novena_start: Some(start),
            },
            &mut model,
        )
        .expect_render()
        .expect_key_value_with(|op| {
            let KeyValueOperation::Set { value, .. } = op else {
                panic!("expected KeyValue set effect");
            };
            let stored: StoredState = serde_json::from_slice(value).unwrap();
            assert_eq!(stored.prayers[0].cadence, IntentionCadence::Daily);
            assert_eq!(stored.prayers[0].novena_start, None);
        });
    }

    #[test]
    fn loads_persisted_prayers_without_novena_start_field() {
        let app = Implore;
        let mut model = Model::default();

        let legacy = r#"{"prayers":[{"id":1,"intention":"Mom","details":null,"tags":[],"status":"Active","cadence":"Unscheduled","prayed_on":[]}],"next_id":2,"reminder_settings":{"enabled":false,"hour":8,"minute":0}}"#;
        app.update(
            Event::PrayersLoaded(Ok(Some(legacy.as_bytes().to_vec()))),
            &mut model,
        )
        .expect_only_render();

        assert_eq!(app.view(&model).prayers[0].novena_start, None);
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
                novena_start: None,
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
            assert_eq!(stored.prayers[0].novena_start, None);
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
                novena_start: None,
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
                unix_seconds: 1_700_000_000,
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
                unix_seconds: 1_700_000_000,
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
                novena_start: None,
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
                novena_start: None,
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
                unix_seconds: 1_700_000_000,
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
    fn sync_local_time_defaults_calendar_date_to_today() {
        let app = Implore;
        let mut model = Model::default();

        app.update(
            Event::SyncLocalTime {
                year: 2026,
                month: 8,
                day: 15,
                hour: 8,
                minute: 0,
                unix_seconds: 1_700_000_000,
            },
            &mut model,
        )
        .expect_only_render();

        let view = app.view(&model);
        assert_eq!(
            view.calendar_date,
            Some(reminder::CivilDate {
                year: 2026,
                month: 8,
                day: 15
            })
        );
        assert_eq!(
            view.calendar_min_date,
            Some(reminder::CivilDate {
                year: 2025,
                month: 8,
                day: 15
            })
        );
        assert_eq!(
            view.calendar_max_date,
            Some(reminder::CivilDate {
                year: 2027,
                month: 8,
                day: 15
            })
        );
        assert_eq!(view.calendar_liturgical_day, view.liturgical_day);
    }

    #[test]
    fn select_calendar_date_clamps_and_lists_due() {
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

        app.update(
            Event::SyncLocalTime {
                year: 2026,
                month: 8,
                day: 15,
                hour: 8,
                minute: 0,
                unix_seconds: 1_700_000_000,
            },
            &mut model,
        )
        .expect_only_render();

        // Sunday: both daily and weekly due
        app.update(
            Event::SelectCalendarDate {
                year: 2026,
                month: 8,
                day: 16,
            },
            &mut model,
        )
        .expect_only_render();

        let view = app.view(&model);
        assert_eq!(
            view.calendar_date,
            Some(reminder::CivilDate {
                year: 2026,
                month: 8,
                day: 16
            })
        );
        assert_eq!(view.calendar_prayers.len(), 2);

        // Far past clamps to min
        app.update(
            Event::SelectCalendarDate {
                year: 2020,
                month: 1,
                day: 1,
            },
            &mut model,
        )
        .expect_only_render();
        assert_eq!(
            app.view(&model).calendar_date,
            Some(reminder::CivilDate {
                year: 2025,
                month: 8,
                day: 15
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
                unix_seconds: 1_700_000_000,
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
                unix_seconds: 1_700_000_000,
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

    #[test]
    fn sign_in_requests_auth_http() {
        let app = Implore;
        let mut model = Model::default();

        app.update(
            Event::SignIn {
                email: "  Me@Example.com ".into(),
                password: "secret".into(),
            },
            &mut model,
        )
        .expect_render()
        .expect_http_with(|req| {
            assert_eq!(req.method, "POST");
            assert_eq!(req.url, format!("{API_BASE_URL}/auth/sign-in"));
            let body: AuthRequest = serde_json::from_slice(&req.body).unwrap();
            assert_eq!(body.email, "me@example.com");
            assert_eq!(body.password, "secret");
        });

        assert_eq!(model.account_status, AccountStatus::SigningIn);
    }

    #[test]
    fn sign_up_requests_auth_http() {
        let app = Implore;
        let mut model = Model::default();

        app.update(
            Event::SignUp {
                email: "  Me@Example.com ".into(),
                password: "secret12".into(),
            },
            &mut model,
        )
        .expect_render()
        .expect_http_with(|req| {
            assert_eq!(req.method, "POST");
            assert_eq!(req.url, format!("{API_BASE_URL}/auth/sign-up"));
            let body: AuthRequest = serde_json::from_slice(&req.body).unwrap();
            assert_eq!(body.email, "me@example.com");
            assert_eq!(body.password, "secret12");
        });

        assert_eq!(model.account_status, AccountStatus::SigningIn);
    }

    #[test]
    fn auth_completed_stores_session() {
        let app = Implore;
        let mut model = Model::default();
        model.account_status = AccountStatus::SigningIn;

        let response = crux_http::testing::ResponseBuilder::ok()
            .body(AuthResponse {
                user_id: "u1".into(),
                token: "tok".into(),
            })
            .build();

        app.update(
            Event::AuthCompleted {
                email: "me@example.com".into(),
                result: Ok(response),
            },
            &mut model,
        )
        .expect_render()
        .expect_key_value_with(|op| {
            let KeyValueOperation::Set { key, value } = op else {
                panic!("expected session set");
            };
            assert_eq!(key, SESSION_KEY);
            let session: Session = serde_json::from_slice(value).unwrap();
            assert_eq!(session.email, "me@example.com");
            assert_eq!(session.token, "tok");
        });

        assert_eq!(model.account_status, AccountStatus::SignedIn);
        assert_eq!(app.view(&model).signed_in_email, "me@example.com");
    }

    #[test]
    fn sign_out_clears_session() {
        let app = Implore;
        let mut model = Model::default();
        model.session = Some(Session {
            user_id: "u1".into(),
            token: "tok".into(),
            email: "me@example.com".into(),
            last_synced_at: None,
        });
        model.account_status = AccountStatus::SignedIn;
        model.last_synced_at = Some(99);

        app.update(Event::SignOut, &mut model)
            .expect_render()
            .expect_key_value_with(|op| {
                assert!(matches!(
                    op,
                    KeyValueOperation::Delete { key } if key == SESSION_KEY
                ));
            })
            .expect_http_with(|req| {
                assert_eq!(req.method, "POST");
                assert_eq!(req.url, format!("{API_BASE_URL}/auth/sign-out"));
                assert!(req.headers.iter().any(|h| {
                    h.name.eq_ignore_ascii_case("authorization") && h.value == "Bearer tok"
                }));
            });

        assert!(model.session.is_none());
        assert_eq!(model.account_status, AccountStatus::SignedOut);
        assert!(model.last_synced_at.is_none());
        assert!(app.view(&model).signed_in_email.is_empty());
    }

    #[test]
    fn sign_up_rejects_short_password() {
        let app = Implore;
        let mut model = Model::default();

        app.update(
            Event::SignUp {
                email: "me@example.com".into(),
                password: "short".into(),
            },
            &mut model,
        )
        .expect_only_render();

        assert_eq!(model.account_status, AccountStatus::Error);
        assert_eq!(
            model.account_error.as_deref(),
            Some("Password must be at least 8 characters")
        );
        assert_eq!(model.account_operation, AccountOperation::SignUp);
    }

    #[test]
    fn sign_up_rejects_invalid_email() {
        let app = Implore;
        let mut model = Model::default();

        app.update(
            Event::SignUp {
                email: "not-an-email".into(),
                password: "secret12".into(),
            },
            &mut model,
        )
        .expect_only_render();

        assert_eq!(model.account_status, AccountStatus::Error);
        assert_eq!(
            model.account_error.as_deref(),
            Some("Enter a valid email address")
        );
        assert_eq!(model.account_operation, AccountOperation::SignUp);
    }

    #[test]
    fn dismiss_account_error_returns_to_signed_out() {
        let app = Implore;
        let mut model = Model::default();
        model.account_status = AccountStatus::Error;
        model.account_operation = AccountOperation::SignUp;
        model.account_error = Some("An account with this email already exists".into());

        app.update(Event::DismissAccountError, &mut model)
            .expect_only_render();

        assert_eq!(model.account_status, AccountStatus::SignedOut);
        assert_eq!(model.account_operation, AccountOperation::Idle);
        assert!(model.account_error.is_none());
        assert!(app.view(&model).account_error.is_empty());
    }

    #[test]
    fn auth_completed_ignored_when_not_signing_in() {
        let app = Implore;
        let mut model = Model::default();
        model.account_status = AccountStatus::SignedOut;

        let response = crux_http::testing::ResponseBuilder::ok()
            .body(AuthResponse {
                user_id: "u1".into(),
                token: "tok".into(),
            })
            .build();

        app.update(
            Event::AuthCompleted {
                email: "me@example.com".into(),
                result: Ok(response),
            },
            &mut model,
        )
        .expect_done();

        assert!(model.session.is_none());
        assert_eq!(model.account_status, AccountStatus::SignedOut);
    }

    #[test]
    fn busy_auth_is_ignored() {
        let app = Implore;
        let mut model = Model::default();
        model.account_status = AccountStatus::SigningIn;

        app.update(
            Event::SignIn {
                email: "other@example.com".into(),
                password: "secret12".into(),
            },
            &mut model,
        )
        .expect_done();

        assert_eq!(model.account_status, AccountStatus::SigningIn);
    }

    #[test]
    fn sync_401_expires_session() {
        let app = Implore;
        let mut model = Model::default();
        model.session = Some(Session {
            user_id: "u1".into(),
            token: "tok".into(),
            email: "me@example.com".into(),
            last_synced_at: Some(9),
        });
        model.account_status = AccountStatus::Syncing;
        model.last_synced_at = Some(9);

        let result = crux_http::testing::rejection::<StoredState>(401, "");
        app.update(Event::SyncGetCompleted(result), &mut model)
            .expect_render()
            .expect_key_value_with(|op| {
                assert!(matches!(
                    op,
                    KeyValueOperation::Delete { key } if key == SESSION_KEY
                ));
            });

        assert!(model.session.is_none());
        assert!(model.last_synced_at.is_none());
        assert_eq!(model.account_status, AccountStatus::Error);
        assert_eq!(
            model.account_error.as_deref(),
            Some("Session expired. Sign in again.")
        );
        assert_eq!(model.account_operation, AccountOperation::SignIn);
    }

    #[test]
    fn session_loaded_restores_last_synced() {
        let app = Implore;
        let mut model = Model::default();
        let session = Session {
            user_id: "u1".into(),
            token: "tok".into(),
            email: "me@example.com".into(),
            last_synced_at: Some(42),
        };
        let bytes = serde_json::to_vec(&session).unwrap();

        app.update(Event::SessionLoaded(Ok(Some(bytes))), &mut model)
            .expect_only_render();

        assert_eq!(model.account_status, AccountStatus::SignedIn);
        assert_eq!(app.view(&model).signed_in_email, "me@example.com");
        assert_eq!(model.last_synced_at, Some(42));
    }

    #[test]
    fn sync_pull_wins_when_remote_newer() {
        let app = Implore;
        let mut model = Model::default();
        model.session = Some(Session {
            user_id: "u1".into(),
            token: "tok".into(),
            email: "me@example.com".into(),
            last_synced_at: None,
        });
        model.account_status = AccountStatus::Syncing;
        model.updated_at = 10;
        model.unix_seconds = Some(50);
        model.prayers = vec![prayer(1, "Local", None, &[], PrayerStatus::Active)];
        model.next_id = 2;

        let remote = StoredState {
            prayers: vec![prayer(7, "Remote", None, &[], PrayerStatus::Active)],
            next_id: 8,
            reminder_settings: ReminderSettings::default(),
            updated_at: 20,
        };
        let response = crux_http::testing::ResponseBuilder::ok().body(remote).build();

        app.update(Event::SyncGetCompleted(Ok(response)), &mut model)
            .expect_render()
            .expect_key_value_with(|op| {
                let KeyValueOperation::Set { key, value } = op else {
                    panic!("expected prayers set");
                };
                assert_eq!(key, PRAYERS_KEY);
                let stored: StoredState = serde_json::from_slice(value).unwrap();
                assert_eq!(stored.updated_at, 20);
                assert_eq!(stored.prayers[0].intention, "Remote");
            })
            .expect_key_value_with(|op| {
                let KeyValueOperation::Set { key, value } = op else {
                    panic!("expected session set");
                };
                assert_eq!(key, SESSION_KEY);
                let session: Session = serde_json::from_slice(value).unwrap();
                assert_eq!(session.last_synced_at, Some(50));
            });

        assert_eq!(model.updated_at, 20);
        assert_eq!(app.view(&model).prayers[0].intention, "Remote");
        assert_eq!(model.last_synced_at, Some(50));
        assert_eq!(model.account_status, AccountStatus::SignedIn);
    }

    #[test]
    fn sync_pushes_when_local_newer() {
        let app = Implore;
        let mut model = Model::default();
        model.session = Some(Session {
            user_id: "u1".into(),
            token: "tok".into(),
            email: "me@example.com".into(),
            last_synced_at: None,
        });
        model.account_status = AccountStatus::Syncing;
        model.updated_at = 30;
        model.unix_seconds = Some(40);
        model.prayers = vec![prayer(1, "Local", None, &[], PrayerStatus::Active)];
        model.next_id = 2;

        let remote = StoredState {
            prayers: vec![prayer(7, "Remote", None, &[], PrayerStatus::Active)],
            next_id: 8,
            reminder_settings: ReminderSettings::default(),
            updated_at: 10,
        };
        let response = crux_http::testing::ResponseBuilder::ok().body(remote).build();

        app.update(Event::SyncGetCompleted(Ok(response)), &mut model)
            .expect_http_with(|req| {
                assert_eq!(req.method, "PUT");
                assert_eq!(req.url, format!("{API_BASE_URL}/sync"));
                assert!(req
                    .headers
                    .iter()
                    .any(|h| h.name.eq_ignore_ascii_case("authorization")
                        && h.value == "Bearer tok"));
                let body: StoredState = serde_json::from_slice(&req.body).unwrap();
                assert_eq!(body.updated_at, 40);
                assert_eq!(body.prayers[0].intention, "Local");
            });
    }
}
