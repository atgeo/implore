//! Local sync API for Implore private accounts.

use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Context;
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Extension, Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use password_hash::rand_core::OsRng;
use rand::{distributions::Alphanumeric, Rng};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
}

#[derive(Debug, Deserialize)]
struct AuthBody {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct AuthResponse {
    user_id: String,
    token: String,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status, Json(ErrorBody { error: self.message })).into_response()
    }
}

type ApiResult<T> = Result<T, AppError>;

#[derive(Clone)]
struct AuthedUser {
    user_id: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let db_path = std::env::var("IMPLORE_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("implore-sync.sqlite3"));

    let conn = Connection::open(&db_path)
        .with_context(|| format!("open sqlite at {}", db_path.display()))?;
    migrate(&conn)?;

    let state = AppState {
        db: Arc::new(Mutex::new(conn)),
    };

    let authed = Router::new()
        .route("/sync", get(get_sync).put(put_sync))
        .route("/auth/sign-out", post(sign_out))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_bearer,
        ));

    let app = Router::new()
        .route("/health", get(|| async { StatusCode::OK }))
        .route("/auth/sign-up", post(sign_up))
        .route("/auth/sign-in", post(sign_in))
        .merge(authed)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = std::env::var("IMPLORE_BIND")
        .unwrap_or_else(|_| "0.0.0.0:3000".into())
        .parse()
        .context("parse IMPLORE_BIND")?;

    tracing::info!("listening on http://{addr} (db {})", db_path.display());
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn migrate(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY NOT NULL,
            email TEXT NOT NULL UNIQUE COLLATE NOCASE,
            password_hash TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sessions (
            token TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS sync_docs (
            user_id TEXT PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            body TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        "#,
    )?;
    Ok(())
}

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

fn hash_password(password: &str) -> ApiResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "Could not hash password"))
}

fn verify_password(password: &str, password_hash: &str) -> ApiResult<()> {
    let parsed = PasswordHash::new(password_hash)
        .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "Invalid password hash"))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| AppError::new(StatusCode::UNAUTHORIZED, "Invalid email or password"))
}

fn new_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect()
}

fn with_db<T>(state: &AppState, f: impl FnOnce(&Connection) -> ApiResult<T>) -> ApiResult<T> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "Database locked"))?;
    f(&conn)
}

async fn sign_up(
    State(state): State<AppState>,
    Json(body): Json<AuthBody>,
) -> ApiResult<Json<AuthResponse>> {
    let email = normalize_email(&body.email);
    let password = body.password.trim();
    if email.is_empty() || password.is_empty() {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "Email and password are required",
        ));
    }
    if password.len() < 8 {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "Password must be at least 8 characters",
        ));
    }

    let user_id = Uuid::new_v4().to_string();
    let password_hash = hash_password(password)?;
    let token = new_token();

    with_db(&state, |conn| {
        let inserted = conn.execute(
            "INSERT INTO users (id, email, password_hash) VALUES (?1, ?2, ?3)",
            params![user_id, email, password_hash],
        );
        match inserted {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err(AppError::new(
                    StatusCode::CONFLICT,
                    "An account with this email already exists",
                ));
            }
            Err(_) => {
                return Err(AppError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Could not create account",
                ));
            }
        }
        conn.execute(
            "INSERT INTO sessions (token, user_id) VALUES (?1, ?2)",
            params![token, user_id],
        )
        .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "Could not create session"))?;
        Ok(())
    })?;

    Ok(Json(AuthResponse { user_id, token }))
}

async fn sign_in(
    State(state): State<AppState>,
    Json(body): Json<AuthBody>,
) -> ApiResult<Json<AuthResponse>> {
    let email = normalize_email(&body.email);
    let password = body.password.trim();
    if email.is_empty() || password.is_empty() {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "Email and password are required",
        ));
    }

    let (user_id, password_hash) = with_db(&state, |conn| {
        conn.query_row(
            "SELECT id, password_hash FROM users WHERE email = ?1",
            params![email],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
        .ok_or_else(|| AppError::new(StatusCode::UNAUTHORIZED, "Invalid email or password"))
    })?;

    verify_password(password, &password_hash)?;
    let token = new_token();
    with_db(&state, |conn| {
        conn.execute(
            "INSERT INTO sessions (token, user_id) VALUES (?1, ?2)",
            params![token, user_id],
        )
        .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "Could not create session"))?;
        Ok(())
    })?;

    Ok(Json(AuthResponse { user_id, token }))
}

async fn sign_out(State(state): State<AppState>, request: Request) -> ApiResult<StatusCode> {
    let token = bearer_token(&request)
        .ok_or_else(|| AppError::new(StatusCode::UNAUTHORIZED, "Missing bearer token"))?;
    with_db(&state, |conn| {
        conn.execute("DELETE FROM sessions WHERE token = ?1", params![token])
            .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "Could not sign out"))?;
        Ok(())
    })?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_sync(
    State(state): State<AppState>,
    Extension(user): Extension<AuthedUser>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = with_db(&state, |conn| {
        conn.query_row(
            "SELECT body, updated_at FROM sync_docs WHERE user_id = ?1",
            params![user.user_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "Database error"))
    })?;

    let Some((body, updated_at)) = row else {
        return Err(AppError::new(StatusCode::NOT_FOUND, "No sync document"));
    };

    let mut doc: serde_json::Value = serde_json::from_str(&body)
        .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "Corrupt sync document"))?;
    if let Some(obj) = doc.as_object_mut() {
        obj.insert(
            "updated_at".into(),
            serde_json::Value::from(updated_at as u64),
        );
    }
    Ok(Json(doc))
}

async fn put_sync(
    State(state): State<AppState>,
    Extension(user): Extension<AuthedUser>,
    Json(mut doc): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let updated_at = doc
        .get("updated_at")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    if let Some(obj) = doc.as_object_mut() {
        obj.insert("updated_at".into(), serde_json::Value::from(updated_at));
    }
    let body = serde_json::to_string(&doc)
        .map_err(|_| AppError::new(StatusCode::BAD_REQUEST, "Invalid sync document"))?;

    with_db(&state, |conn| {
        conn.execute(
            r#"
            INSERT INTO sync_docs (user_id, body, updated_at) VALUES (?1, ?2, ?3)
            ON CONFLICT(user_id) DO UPDATE SET
                body = excluded.body,
                updated_at = excluded.updated_at
            "#,
            params![user.user_id, body, updated_at as i64],
        )
        .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "Could not save sync"))?;
        Ok(())
    })?;

    Ok(Json(doc))
}

fn bearer_token(request: &Request) -> Option<String> {
    let value = request.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))?;
    let token = token.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

async fn require_bearer(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = bearer_token(&request)
        .ok_or_else(|| AppError::new(StatusCode::UNAUTHORIZED, "Missing bearer token"))?;

    let user_id = with_db(&state, |conn| {
        conn.query_row(
            "SELECT user_id FROM sessions WHERE token = ?1",
            params![token],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
        .ok_or_else(|| AppError::new(StatusCode::UNAUTHORIZED, "Invalid session"))
    })?;

    request.extensions_mut().insert(AuthedUser { user_id });
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_app() -> Router {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let state = AppState {
            db: Arc::new(Mutex::new(conn)),
        };
        let authed = Router::new()
            .route("/sync", get(get_sync).put(put_sync))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                require_bearer,
            ));
        Router::new()
            .route("/auth/sign-up", post(sign_up))
            .route("/auth/sign-in", post(sign_in))
            .merge(authed)
            .with_state(state)
    }

    async fn json_body(response: Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn sign_up_put_get_sync() {
        let app = test_app();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/sign-up")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"me@example.com","password":"secret12"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let auth = json_body(response).await;
        let token = auth["token"].as_str().unwrap();

        let doc = serde_json::json!({
            "prayers": [],
            "next_id": 1,
            "reminder_settings": {"enabled": false, "hour": 8, "minute": 0},
            "updated_at": 42
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/sync")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::from(doc.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/sync")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let got = json_body(response).await;
        assert_eq!(got["updated_at"], 42);
        assert_eq!(got["next_id"], 1);
    }

    #[tokio::test]
    async fn sign_up_existing_email_conflicts() {
        let app = test_app();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/sign-up")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"me@example.com","password":"secret12"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/sign-up")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"me@example.com","password":"secret12"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(
            json_body(response).await["error"],
            "An account with this email already exists"
        );
    }
}
