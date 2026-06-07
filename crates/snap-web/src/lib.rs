//! Axum web layer for SnapStation: REST API, MJPEG live-view stream and the
//! embedded PWA frontend, all served from a single process.

mod api;
mod assets;
mod auth;
mod branding;
mod camera;
mod chroma;
mod comfort;
mod error;
mod print;
mod share;
mod state;
mod stream;
mod update;

pub use state::AppState;

use axum::routing::{get, post};
use axum::Router;
use snap_camera::CameraHandle;
use snap_core::{Config, Db};
use snap_print::PrintService;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;

/// Returns the running SnapStation version (re-exported from `snap-core`).
pub fn version() -> &'static str {
    snap_core::VERSION
}

/// Build the full application router from a constructed [`AppState`].
pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(api::health))
        .route("/login", post(auth::login))
        .route("/logout", post(auth::logout))
        .route("/auth", get(auth::auth_status))
        .route("/status", get(api::status))
        .route("/camera", get(camera::get_camera).post(camera::set_camera))
        .route("/camera/detect", post(camera::detect_camera))
        .route("/photos", get(api::list_photos))
        .route("/capture", post(api::capture))
        .route("/preview/stream", get(stream::preview_stream))
        .route("/setup", get(api::setup_status).post(api::setup_submit))
        .route("/chroma", get(chroma::get_chroma).put(chroma::put_chroma))
        .route("/chroma/calibrate", post(chroma::calibrate))
        .route("/chroma/preview", post(chroma::preview))
        .route("/chroma/background", post(chroma::select_background))
        .route("/printers", get(print::printers))
        .route("/print", post(print::print_photo))
        .route("/print/jobs", get(print::jobs))
        .route(
            "/print/settings",
            get(print::get_settings).post(print::set_settings),
        )
        .route(
            "/backgrounds",
            get(chroma::list_backgrounds).post(chroma::upload_background),
        )
        .route(
            "/backgrounds/{ulid}",
            axum::routing::delete(chroma::delete_background),
        )
        .route("/qr", get(share::qr))
        .route("/share/email", post(share::share_email))
        .route(
            "/email/settings",
            get(share::get_email_settings).post(share::set_email_settings),
        )
        .route("/theme", get(branding::get_theme).put(branding::put_theme))
        .route("/branding/logo", post(branding::upload_logo))
        .route("/branding/kiosk-bg", post(branding::upload_kiosk_bg))
        .route(
            "/comfort",
            get(comfort::get_comfort).post(comfort::set_comfort),
        )
        .route("/backup/targets", get(comfort::backup_targets))
        .route("/backup", post(comfort::backup))
        .route("/update/check", get(update::check))
        .route("/update/apply", post(update::apply))
        // Allow image uploads (backgrounds, logos) larger than the 2 MB default.
        .layer(axum::extract::DefaultBodyLimit::max(25 * 1024 * 1024));

    Router::new()
        .nest("/api", api)
        .route("/p/{token}", get(api::photo_file))
        .route("/p/{token}/thumb", get(api::thumb_file))
        .route("/bg/{ulid}", get(chroma::background_file))
        .route("/branding/logo", get(branding::logo_file))
        .route("/branding/kiosk-bg", get(branding::kiosk_bg_file))
        .route("/d/{token}", get(share::download_page))
        .with_state(state)
        .fallback(assets::static_handler)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}

/// Bind and serve the application until shutdown.
pub async fn serve(
    db: Db,
    camera: CameraHandle,
    printer: Arc<PrintService>,
    config: Arc<Config>,
) -> std::io::Result<()> {
    let bind = config.bind;
    let state = AppState {
        db,
        camera,
        printer,
        config,
        sessions: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
    };

    // Background retry of any queued e-mails.
    tokio::spawn(share::process_queue(state.clone()));

    let app = router(state);
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!("SnapStation listening on http://{bind}");
    axum::serve(listener, app).await
}
