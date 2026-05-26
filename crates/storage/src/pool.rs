use std::path::{Path, PathBuf};

use directories_next::ProjectDirs;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{ConnectOptions, SqlitePool};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("could not determine application data directory")]
    NoAppDir,

    #[error("io error preparing database directory: {0}")]
    Io(#[from] std::io::Error),

    #[error("sqlite error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

/// `~/.local/share/hivemind/hivemind.db` on Linux, equivalent on other
/// platforms via `directories-next`.
pub fn default_db_path() -> Result<PathBuf, StorageError> {
    let dirs = ProjectDirs::from("dev", "hivemind", "hivemind").ok_or(StorageError::NoAppDir)?;
    let data = dirs.data_dir();
    Ok(data.join("hivemind.db"))
}

/// Open a pool against `db_path`, ensuring the parent dir exists, enabling
/// WAL + normal synchronous, and applying embedded migrations.
pub async fn open_pool(db_path: &Path) -> Result<SqlitePool, StorageError> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .log_statements(tracing::log::LevelFilter::Trace);

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .min_connections(1)
        .connect_with(opts)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

/// Open an in-memory pool with migrations applied. Test-only helper.
#[cfg(test)]
pub(crate) async fn open_pool_in_memory() -> Result<SqlitePool, StorageError> {
    open_pool_in_memory_impl().await
}

#[cfg(test)]
async fn open_pool_in_memory_impl() -> Result<SqlitePool, StorageError> {
    let opts = SqliteConnectOptions::new()
        .in_memory(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
