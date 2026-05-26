//! The `Provider` trait and its associated wire-domain types.
//!
//! Every concrete backend (Anthropic, Ollama, future Voyage…) implements
//! [`Provider`]. The orchestrator only knows the trait; vendor-specific quirks
//! (SSE event layouts, NDJSON framing, cache-control plumbing) are confined to
//! the implementation modules.

use std::time::Duration;

use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ProviderError;

#[async_trait]
pub trait Provider: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    fn capabilities(&self) -> Capabilities;

    async fn complete(
        &self,
        req: ChatRequest,
    ) -> Result<BoxStream<'static, Result<Delta, ProviderError>>, ProviderError>;

    async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse, ProviderError>;
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Capabilities {
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub supports_prompt_caching: bool,
    pub supports_embeddings: bool,
    pub max_input_tokens: u32,
    pub max_output_tokens: u32,
    pub local: bool,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelHint {
    #[default]
    Auto,
    Fast,
    Smart,
    Local,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemBlock {
    pub text: String,
    /// When `Some`, the provider applies `cache_control: { type: <kind> }` on
    /// the marshalled block (Anthropic only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CacheControl {
    Ephemeral,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model_hint: ModelHint,
    /// Optional explicit model identifier; overrides the provider default for
    /// `hint` resolution but is otherwise just plumbed through.
    pub model: Option<String>,
    pub system: Vec<SystemBlock>,
    pub messages: Vec<Message>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

impl ChatRequest {
    pub fn new(model_hint: ModelHint) -> Self {
        Self {
            model_hint,
            model: None,
            system: Vec::new(),
            messages: Vec::new(),
            tools: Vec::new(),
            max_tokens: None,
            temperature: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbedRequest {
    pub texts: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbedResponse {
    pub vectors: Vec<Vec<f32>>,
    pub dim: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Delta {
    TextChunk(String),
    ToolCall {
        id: String,
        name: String,
        args: Value,
    },
    Done {
        stop_reason: StopReason,
        usage: Usage,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    ToolUse,
    StopSequence,
    Cancelled,
    Error,
}

impl StopReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            StopReason::EndTurn => "end_turn",
            StopReason::MaxTokens => "max_tokens",
            StopReason::ToolUse => "tool_use",
            StopReason::StopSequence => "stop_sequence",
            StopReason::Cancelled => "cancelled",
            StopReason::Error => "error",
        }
    }

    pub fn from_anthropic(s: &str) -> Self {
        match s {
            "end_turn" => Self::EndTurn,
            "max_tokens" => Self::MaxTokens,
            "tool_use" => Self::ToolUse,
            "stop_sequence" => Self::StopSequence,
            _ => Self::EndTurn,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

/// Hint used by retry helpers to decide whether to pause before retrying.
pub fn retry_after_or_default(d: Option<Duration>) -> Duration {
    d.unwrap_or(Duration::from_millis(500))
}
