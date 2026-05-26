//! AI provider trait, error types, and concrete implementations.
//!
//! The orchestrator only knows the trait + types in `provider` and the
//! `Router`. Vendor-specific wire concerns (Anthropic SSE, Ollama NDJSON,
//! header marshalling) stay inside the per-vendor modules.

pub mod anthropic;
mod error;
pub mod ollama;
mod provider;
pub mod router;
pub mod secrets;

pub use error::{parse_retry_after, ProviderError};
pub use provider::{
    retry_after_or_default, CacheControl, Capabilities, ChatRequest, Delta, EmbedRequest,
    EmbedResponse, Message, ModelHint, Provider, Role, StopReason, SystemBlock, ToolSchema,
    Usage,
};
pub use router::{Router, RoutingPolicy};

pub use anthropic::AnthropicProvider;
pub use ollama::OllamaProvider;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
