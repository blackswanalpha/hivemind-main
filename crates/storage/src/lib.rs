//! SQLite persistence for HiveMind (sqlx, runtime checked queries).

mod ai_persistence;
mod pool;
mod sqlite_session_store;

pub use ai_persistence::SqliteAiPersistence;
pub use pool::{default_db_path, open_pool, StorageError};
pub use sqlite_session_store::SqliteSessionStore;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
