//! Serves the embedded PWA frontend.
//!
//! In release builds the frontend (built by Vite into `crates/snap-web/assets`)
//! is embedded into the binary by `rust-embed`. In debug builds it is read from
//! disk at runtime, so the UI can be iterated on without recompiling Rust.
//! Unknown non-asset paths fall back to `index.html` for client-side routing.

use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets"]
struct Assets;

pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    if let Some(content) = Assets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return (
            [(header::CONTENT_TYPE, mime.as_ref())],
            content.data.into_owned(),
        )
            .into_response();
    }

    // SPA fallback: serve index.html for app routes that aren't real files.
    match Assets::get("index.html") {
        Some(content) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            content.data.into_owned(),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            "Frontend not built yet. Run the Vite build into crates/snap-web/assets.",
        )
            .into_response(),
    }
}
