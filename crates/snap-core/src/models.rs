//! Database models and the queries that operate on them.

use crate::db::Db;
use crate::{ids, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A captured photo (single shot, collage or animated GIF).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Photo {
    pub id: i64,
    pub ulid: String,
    /// Unguessable share/download token.
    pub token: String,
    pub session_id: Option<i64>,
    pub original_path: String,
    pub thumb_path: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    /// `single` | `collage` | `gif`
    pub kind: String,
    pub created_at: String,
}

/// Fields required to record a freshly captured photo.
#[derive(Debug, Clone)]
pub struct NewPhoto {
    pub session_id: Option<i64>,
    pub original_path: String,
    pub thumb_path: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub kind: String,
}

impl Photo {
    /// Insert a new photo, assigning a fresh ULID and share token.
    pub async fn insert(db: &Db, new: NewPhoto) -> Result<Photo> {
        let ulid = ids::new_ulid();
        let token = ids::new_token();
        let photo: Photo = sqlx::query_as(
            "INSERT INTO photos
               (ulid, token, session_id, original_path, thumb_path, width, height, kind)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING *",
        )
        .bind(&ulid)
        .bind(&token)
        .bind(new.session_id)
        .bind(&new.original_path)
        .bind(&new.thumb_path)
        .bind(new.width)
        .bind(new.height)
        .bind(&new.kind)
        .fetch_one(db)
        .await?;
        Ok(photo)
    }

    /// Most recent photos first, paginated.
    pub async fn list(db: &Db, limit: i64, offset: i64) -> Result<Vec<Photo>> {
        let rows: Vec<Photo> = sqlx::query_as(
            "SELECT * FROM photos ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Look up a photo by its public share token.
    pub async fn by_token(db: &Db, token: &str) -> Result<Photo> {
        let row: Option<Photo> = sqlx::query_as("SELECT * FROM photos WHERE token = ?")
            .bind(token)
            .fetch_optional(db)
            .await?;
        row.ok_or(crate::Error::NotFound)
    }

    /// Total number of photos.
    pub async fn count(db: &Db) -> Result<i64> {
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM photos")
            .fetch_one(db)
            .await?;
        Ok(n)
    }
}

/// A background image available for chroma-key compositing.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Background {
    pub id: i64,
    pub ulid: String,
    pub name: String,
    /// Path relative to the data directory.
    pub path: String,
    pub created_at: String,
}

impl Background {
    pub async fn insert(db: &Db, name: &str, path: &str) -> Result<Background> {
        let ulid = ids::new_ulid();
        let bg: Background = sqlx::query_as(
            "INSERT INTO backgrounds (ulid, name, path) VALUES (?, ?, ?) RETURNING *",
        )
        .bind(&ulid)
        .bind(name)
        .bind(path)
        .fetch_one(db)
        .await?;
        Ok(bg)
    }

    pub async fn list(db: &Db) -> Result<Vec<Background>> {
        Ok(
            sqlx::query_as("SELECT * FROM backgrounds ORDER BY created_at DESC, id DESC")
                .fetch_all(db)
                .await?,
        )
    }

    pub async fn by_ulid(db: &Db, ulid: &str) -> Result<Background> {
        let row: Option<Background> = sqlx::query_as("SELECT * FROM backgrounds WHERE ulid = ?")
            .bind(ulid)
            .fetch_optional(db)
            .await?;
        row.ok_or(crate::Error::NotFound)
    }

    /// Delete the row and return its stored path (so the caller can remove the file).
    pub async fn delete(db: &Db, ulid: &str) -> Result<String> {
        let bg = Self::by_ulid(db, ulid).await?;
        sqlx::query("DELETE FROM backgrounds WHERE ulid = ?")
            .bind(ulid)
            .execute(db)
            .await?;
        Ok(bg.path)
    }
}

/// A print job and its current status, joined with the photo's share token so
/// the admin queue can show a thumbnail.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PrintJobView {
    pub id: i64,
    pub printer: String,
    pub job_ref: Option<String>,
    pub status: String,
    pub message: Option<String>,
    pub created_at: String,
    pub token: Option<String>,
}

/// Print-job persistence.
pub mod print_jobs {
    use super::*;

    /// Record a new print job; returns its id.
    pub async fn insert(
        db: &Db,
        photo_id: i64,
        printer: &str,
        job_ref: Option<&str>,
        status: &str,
        message: Option<&str>,
    ) -> Result<i64> {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO print_jobs (photo_id, printer, job_ref, status, message)
             VALUES (?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(photo_id)
        .bind(printer)
        .bind(job_ref)
        .bind(status)
        .bind(message)
        .fetch_one(db)
        .await?;
        Ok(id)
    }

    pub async fn update_status(
        db: &Db,
        id: i64,
        status: &str,
        message: Option<&str>,
    ) -> Result<()> {
        sqlx::query("UPDATE print_jobs SET status = ?, message = ? WHERE id = ?")
            .bind(status)
            .bind(message)
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }

    /// Number of non-errored prints already made for a photo (for print limits).
    pub async fn count_for_photo(db: &Db, photo_id: i64) -> Result<i64> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM print_jobs WHERE photo_id = ? AND status != 'error'",
        )
        .bind(photo_id)
        .fetch_one(db)
        .await?;
        Ok(n)
    }

    /// Recent jobs (newest first) for the admin queue.
    pub async fn recent(db: &Db, limit: i64) -> Result<Vec<PrintJobView>> {
        Ok(sqlx::query_as(
            "SELECT j.id, j.printer, j.job_ref, j.status, j.message, j.created_at, p.token
             FROM print_jobs j
             LEFT JOIN photos p ON p.id = j.photo_id
             ORDER BY j.created_at DESC, j.id DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(db)
        .await?)
    }
}

/// Offline-friendly queue of outgoing share e-mails.
pub mod email_queue {
    use super::*;

    /// Add a failed/pending send to the queue.
    pub async fn enqueue(db: &Db, token: &str, to_addr: &str, error: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO email_queue (token, to_addr, status, error) VALUES (?, ?, 'pending', ?)",
        )
        .bind(token)
        .bind(to_addr)
        .bind(error)
        .execute(db)
        .await?;
        Ok(())
    }

    /// Up to `limit` pending entries as `(id, token, to_addr)`.
    pub async fn pending(db: &Db, limit: i64) -> Result<Vec<(i64, String, String)>> {
        Ok(sqlx::query_as(
            "SELECT id, token, to_addr FROM email_queue WHERE status = 'pending' LIMIT ?",
        )
        .bind(limit)
        .fetch_all(db)
        .await?)
    }

    pub async fn mark_sent(db: &Db, id: i64) -> Result<()> {
        sqlx::query("UPDATE email_queue SET status = 'sent', error = NULL WHERE id = ?")
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }

    pub async fn mark_attempt(db: &Db, id: i64, error: &str) -> Result<()> {
        sqlx::query("UPDATE email_queue SET attempts = attempts + 1, error = ? WHERE id = ?")
            .bind(error)
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }
}

/// Admin credential storage (single administrator). Hashing/verification is
/// performed by the web layer; this module only persists the hash.
pub mod admin {
    use super::*;

    /// The stored Argon2 password hash, if an admin password has been set.
    pub async fn password_hash(db: &Db) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as("SELECT password_hash FROM admin WHERE id = 1")
            .fetch_optional(db)
            .await?;
        Ok(row.map(|r| r.0))
    }

    /// Store (or replace) the admin password hash.
    pub async fn set_password_hash(db: &Db, hash: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO admin (id, password_hash) VALUES (1, ?)
             ON CONFLICT(id) DO UPDATE SET password_hash = excluded.password_hash",
        )
        .bind(hash)
        .execute(db)
        .await?;
        Ok(())
    }
}
