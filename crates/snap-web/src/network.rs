//! Network helpers: list the device's local IPv4 addresses so the admin can
//! pick which one the QR-code / download links should point to. The kiosk's own
//! browser host is often `127.0.0.1`, which phones cannot reach — selecting the
//! LAN IP makes the QR codes scannable from guests' devices.

use crate::auth::AdminAuth;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use snap_core::settings::{self, keys};

/// All usable IPv4 addresses of the device (no loopback / link-local).
fn list_ipv4() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Ok(ifas) = local_ip_address::list_afinet_netifas() {
        for (_name, ip) in ifas {
            if let std::net::IpAddr::V4(v4) = ip {
                if !v4.is_loopback() && !v4.is_link_local() && !v4.is_unspecified() {
                    out.push(v4.to_string());
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

#[derive(Serialize)]
pub struct NetworkInfo {
    /// Detected IPv4 addresses to choose from.
    ips: Vec<String>,
    /// Currently selected host (empty = automatic / use the kiosk's own host).
    selected: String,
}

/// Public: the kiosk/gallery need the selected host to build QR links.
pub async fn get_network(State(st): State<AppState>) -> Result<Json<NetworkInfo>, ApiError> {
    Ok(Json(NetworkInfo {
        ips: list_ipv4(),
        selected: settings::get_or(&st.db, keys::PUBLIC_HOST, "").await?,
    }))
}

#[derive(Deserialize)]
pub struct SetNetwork {
    /// Selected host/IP, or empty string for automatic.
    host: String,
}

pub async fn set_network(
    _admin: AdminAuth,
    State(st): State<AppState>,
    Json(req): Json<SetNetwork>,
) -> Result<Json<Value>, ApiError> {
    let host = req.host.trim();
    // Allow empty (auto) or one of the detected addresses only.
    if !host.is_empty() && !list_ipv4().iter().any(|ip| ip == host) {
        return Err(ApiError::bad_request("unbekannte Adresse"));
    }
    settings::set(&st.db, keys::PUBLIC_HOST, host).await?;
    Ok(Json(json!({ "ok": true })))
}
