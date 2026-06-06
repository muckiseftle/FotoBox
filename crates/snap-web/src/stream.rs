//! MJPEG live-view stream.
//!
//! A single preview-grab loop in the camera actor publishes the latest frame
//! into a `watch` channel; this handler relays that shared frame to the client
//! as `multipart/x-mixed-replace`. Many clients (kiosk + phones) can stream at
//! once while the camera is read only once. A [`PreviewGuard`] keeps the actor
//! grabbing frames for as long as the response body is alive.

use crate::state::AppState;
use async_stream::stream;
use axum::body::Body;
use axum::extract::State;
use axum::http::header;
use axum::response::Response;
use bytes::Bytes;

const BOUNDARY: &str = "snapframe";

pub async fn preview_stream(State(st): State<AppState>) -> Response {
    let guard = st.camera.preview_guard().await;
    let mut rx = st.camera.subscribe_preview();

    let body = stream! {
        // Hold the viewer registration for the lifetime of the stream.
        let _guard = guard;
        loop {
            if rx.changed().await.is_err() {
                break; // actor shut down
            }
            let frame = rx.borrow_and_update().clone();
            if let Some(jpeg) = frame {
                let header = format!(
                    "--{BOUNDARY}\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                    jpeg.len()
                );
                let mut buf = Vec::with_capacity(header.len() + jpeg.len() + 2);
                buf.extend_from_slice(header.as_bytes());
                buf.extend_from_slice(&jpeg);
                buf.extend_from_slice(b"\r\n");
                yield Ok::<Bytes, std::io::Error>(Bytes::from(buf));
            }
        }
    };

    Response::builder()
        .header(
            header::CONTENT_TYPE,
            format!("multipart/x-mixed-replace; boundary={BOUNDARY}"),
        )
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from_stream(body))
        .expect("valid response")
}
