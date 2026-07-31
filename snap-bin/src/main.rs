//! SnapStation binary entry point: wires configuration, database, the camera
//! actor and the web server into a single process.

use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,snap_web=debug,snap_camera=debug".into()),
        )
        .init();

    let config = snap_core::Config::from_env();
    config.ensure_dirs()?;

    let bind = config.bind;
    // A 0.0.0.0/:: bind is reachable on the LAN, but for the local browser we
    // open the loopback address. "localhost" instead of "127.0.0.1": Safari's
    // HTTPS-Only mode blocks plain-HTTP loopback URLs (WebKitErrorDomain:305,
    // WebKit bug 284559); the name form is what WebKit's exemption targets.
    let host = if bind.ip().is_unspecified() || bind.ip().is_loopback() {
        "localhost".to_string()
    } else {
        bind.ip().to_string()
    };
    let url = format!("http://{host}:{}", bind.port());

    let db = snap_core::db::connect(&config.db_path).await?;
    let camera = snap_camera::spawn_auto();
    let printer = Arc::new(snap_print::PrintService::detect());

    // Apply a previously chosen webcam index, if one was selected in the admin.
    if let Ok(Some(idx)) =
        snap_core::settings::get(&db, snap_core::settings::keys::CAMERA_INDEX).await
    {
        if let Ok(i) = idx.parse::<u32>() {
            if i > 0 {
                let _ = camera.use_webcam(i).await;
            }
        }
    }

    println!("\n  ▶ SnapStation läuft:  {url}\n    (im Browser öffnen · Strg+C zum Beenden)\n");

    // Open the default browser for easy desktop use, unless disabled (e.g. the
    // systemd service sets SNAP_NO_OPEN=1 on a headless station).
    if std::env::var_os("SNAP_NO_OPEN").is_none() {
        let u = url.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(800)).await;
            let _ = open::that(u);
        });
    }

    snap_web::serve(db, camera, printer, Arc::new(config)).await?;
    Ok(())
}
