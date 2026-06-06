//! Admin authentication: password login (Argon2) → in-memory session cookie,
//! plus an [`AdminAuth`] extractor that guards admin/config endpoints. Guest
//! (kiosk/phone) endpoints stay open; only configuration and mutation routes
//! require a logged-in admin.

use crate::error::ApiError;
use crate::state::AppState;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::extract::{FromRef, FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

const COOKIE: &str = "snap_session";

/// Read the session token from the Cookie header.
fn cookie_token(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';').find_map(|kv| {
        kv.trim()
            .strip_prefix(&format!("{COOKIE}="))
            .map(str::to_string)
    })
}

/// Whether the request carries a valid admin session.
pub fn is_authed(state: &AppState, headers: &HeaderMap) -> bool {
    match cookie_token(headers) {
        Some(tok) => state
            .sessions
            .lock()
            .map(|s| s.contains(&tok))
            .unwrap_or(false),
        None => false,
    }
}

#[derive(Deserialize)]
pub struct LoginReq {
    password: String,
}

pub async fn login(
    State(st): State<AppState>,
    Json(req): Json<LoginReq>,
) -> Result<Response, ApiError> {
    let hash = snap_core::models::admin::password_hash(&st.db)
        .await?
        .ok_or_else(|| {
            ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "Setup nicht abgeschlossen")
        })?;
    let parsed = PasswordHash::new(&hash)
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed)
        .is_err()
    {
        return Err(ApiError::new(StatusCode::UNAUTHORIZED, "Falsches Passwort"));
    }

    let token = snap_core::ids::new_token();
    if let Ok(mut s) = st.sessions.lock() {
        s.insert(token.clone());
    }
    let cookie = format!("{COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age=86400");
    Ok(([(header::SET_COOKIE, cookie)], Json(json!({ "ok": true }))).into_response())
}

pub async fn logout(State(st): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(tok) = cookie_token(&headers) {
        if let Ok(mut s) = st.sessions.lock() {
            s.remove(&tok);
        }
    }
    let clear = format!("{COOKIE}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0");
    ([(header::SET_COOKIE, clear)], Json(json!({ "ok": true }))).into_response()
}

pub async fn auth_status(State(st): State<AppState>, headers: HeaderMap) -> Json<Value> {
    Json(json!({ "authenticated": is_authed(&st, &headers) }))
}

/// Extractor that admits only authenticated admins. Add it as a handler
/// parameter to protect a route; it rejects with 401 before the handler runs.
pub struct AdminAuth;

impl<S> FromRequestParts<S> for AdminAuth
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app = AppState::from_ref(state);
        if is_authed(&app, &parts.headers) {
            Ok(AdminAuth)
        } else {
            Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Anmeldung erforderlich" })),
            )
                .into_response())
        }
    }
}
