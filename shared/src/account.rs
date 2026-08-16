//! Private account session and cloud sync types (Crux core).

use facet::Facet;
use serde::{Deserialize, Serialize};

/// Local API base URL for development (iOS Simulator → host loopback).
pub const API_BASE_URL: &str = "http://127.0.0.1:3000";

/// Key-value key for the persisted [`Session`].
pub const SESSION_KEY: &str = "session";

/// Minimum password length enforced at sign-up.
pub const MIN_PASSWORD_LEN: usize = 8;

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[repr(C)]
pub enum AccountStatus {
    #[default]
    SignedOut,
    SigningIn,
    SignedIn,
    Syncing,
    Error,
}

/// Which account request last ran — used so Sign In / Create Account / Sync
/// each show only their own errors.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[repr(C)]
pub enum AccountOperation {
    #[default]
    Idle,
    SignIn,
    SignUp,
    Sync,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub user_id: String,
    pub token: String,
    pub email: String,
    /// When this device last finished a successful sync.
    #[serde(default)]
    pub last_synced_at: Option<u64>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct AuthRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AuthResponse {
    pub user_id: String,
    pub token: String,
}

pub(crate) fn auth_url(path: &str) -> String {
    format!("{API_BASE_URL}{path}")
}

pub(crate) fn normalize_email(email: String) -> String {
    email.trim().to_lowercase()
}

pub(crate) fn email_looks_valid(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    if local.starts_with('.') || local.ends_with('.') || local.contains(' ') {
        return false;
    }
    if domain.starts_with('.') || domain.ends_with('.') || domain.contains(' ') {
        return false;
    }
    domain.contains('.')
}

pub(crate) fn auth_error_message(error: &crux_http::HttpError) -> String {
    if let Some(body) = error.body() {
        if let Ok(text) = std::str::from_utf8(body) {
            let trimmed = text.trim();
            if !looks_like_html(trimmed) && trimmed.len() <= 160 {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    if let Some(msg) = json.get("error").and_then(|v| v.as_str()) {
                        if !msg.is_empty() && msg.len() <= 160 {
                            return msg.to_string();
                        }
                    }
                }
            }
        }
    }
    match error.code() {
        Some(401) => "Invalid email or password".into(),
        Some(409) => "An account with this email already exists".into(),
        Some(code) => format!("Request failed ({code})"),
        None => "Network error. Check your connection and try again.".into(),
    }
}

pub(crate) fn session_expired_message() -> String {
    "Session expired. Sign in again.".into()
}

fn looks_like_html(text: &str) -> bool {
    let head = text.get(..64).unwrap_or(text).to_ascii_lowercase();
    head.starts_with('<') || head.contains("<html") || head.contains("<!doctype")
}
