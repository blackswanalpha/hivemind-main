//! Pure-Rust domain model for HiveMind.
//!
//! No Tauri, no networking, no SQL — every dep here is also available in
//! `crates/storage` and the Tauri app. The persistence boundary is the
//! [`SessionStore`] trait, implemented by `hivemind-storage::SqliteSessionStore`.

mod error;
mod ids;
mod session;
mod tab;
mod workspace;

pub use error::StoreError;
pub use ids::{ConversationId, TabId, WorkspaceId};
pub use session::{Session, SessionStore};
pub use tab::{move_tab, repack_positions, Tab};
pub use workspace::Workspace;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
