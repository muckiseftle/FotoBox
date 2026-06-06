//! SQLite connection pool and migration runner.

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::path::Path;
use std::time::Duration;

/// Shared database handle (clone-cheap connection pool).
pub type Db = sqlx::SqlitePool;

/// Open (creating if necessary) the SQLite database with WAL mode for good
/// concurrent read performance, then run embedded migrations.
pub async fn connect(db_path: &Path) -> crate::Result<Db> {
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
