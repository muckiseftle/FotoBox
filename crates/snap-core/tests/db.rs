//! Integration test exercising migrations and the photo/settings queries
//! against a real temporary SQLite database.

use snap_core::{
    db,
    models::{NewPhoto, Photo},
    settings,
};

#[tokio::test]
async fn migrate_settings_and_photos_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let pool = db::connect(&dir.path().join("test.db")).await.unwrap();

    // Settings round-trip (upsert semantics).
    settings::set(&pool, settings::keys::EVENT_NAME, "Hochzeit")
        .await
        .unwrap();
    settings::set(&pool, settings::keys::EVENT_NAME, "Geburtstag")
        .await
        .unwrap();
    assert_eq!(
        settings::get_or(&pool, settings::keys::EVENT_NAME, "?")
            .await
            .unwrap(),
        "Geburtstag"
    );
    assert!(!settings::is_setup_complete(&pool).await.unwrap());

    // Insert a photo and read it back through each query path.
    let inserted = Photo::insert(
        &pool,
        NewPhoto {
            session_id: None,
            original_path: "photos/2026/a.jpg".into(),
            thumb_path: Some("photos/2026/a_thumb.jpg".into()),
            width: Some(6000),
            height: Some(4000),
            kind: "single".into(),
        },
    )
    .await
    .unwrap();

    assert_eq!(inserted.ulid.len(), 26);
    assert_eq!(inserted.token.len(), 24);

    let listed = Photo::list(&pool, 10, 0).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, inserted.id);

    let by_token = Photo::by_token(&pool, &inserted.token).await.unwrap();
    assert_eq!(by_token.original_path, "photos/2026/a.jpg");

    assert_eq!(Photo::count(&pool).await.unwrap(), 1);
    assert!(matches!(
        Photo::by_token(&pool, "does-not-exist").await,
        Err(snap_core::Error::NotFound)
    ));
}
