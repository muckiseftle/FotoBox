//! Exercises the camera actor end-to-end using the mock backend: capture
//! returns JPEG bytes with sane dimensions, and registering a preview viewer
//! makes the shared frame channel produce frames.

use snap_camera::{spawn, MockCamera};

#[tokio::test]
async fn mock_capture_returns_jpeg() {
    let cam = spawn(Box::new(MockCamera::new()));

    let shot = cam.capture().await.unwrap();
    assert_eq!((shot.width, shot.height), (1920, 1280));
    // JPEG magic bytes.
    assert_eq!(&shot.jpeg[..2], &[0xFF, 0xD8]);

    cam.shutdown().await;
}

#[tokio::test]
async fn preview_produces_shared_frames() {
    let cam = spawn(Box::new(MockCamera::new()));
    let mut preview = cam.subscribe_preview();

    // No viewers yet -> no frame.
    assert!(preview.borrow_and_update().is_none());

    let guard = cam.preview_guard().await;
    // Wait for the actor to publish at least one frame.
    preview.changed().await.unwrap();
    let frame = preview.borrow_and_update().clone();
    assert!(frame.map(|f| !f.is_empty()).unwrap_or(false));

    drop(guard);
    cam.shutdown().await;
}
