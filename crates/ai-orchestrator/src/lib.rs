//! AI orchestration crate.
//!
//! Streams provider deltas, persists conversations, exposes the
//! [`Orchestrator`] used by the Tauri commands. Tool loop, memory recall and
//! agent runtime land in later phases.

mod error;
pub mod orchestrator;
pub mod persistence;
pub mod settings;
pub mod state;
pub mod system_prompt;

pub use error::OrchestratorError;
pub use orchestrator::{
    CompleteCallback, ErrorCallback, Orchestrator, SendOutcome, StreamCallbacks, TokenCallback,
};
pub use persistence::{AiPersistence, ConversationRecord, MessageRecord, PersistenceError};
pub use settings::{hint_from_str, AiSettings, PolicyChoice, SETTINGS_KEY};
pub use state::{ConversationState, TurnMessage};
pub use system_prompt::{
    assemble_system_prompt, default_capabilities, default_persona, default_product,
    SystemPromptLayers,
};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
