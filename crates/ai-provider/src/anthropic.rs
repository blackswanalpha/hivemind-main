//! Anthropic `messages` API implementation.
//!
//! - Endpoint: `POST https://api.anthropic.com/v1/messages` with `stream: true`.
//! - Auth header: `x-api-key`, loaded from the `ANTHROPIC_API_KEY` env var.
//! - Prompt caching is REQUIRED per `docs/ai.md` §6.1; pass through any
//!   `cache_control` markers placed by the orchestrator on system blocks.
//!
//! SSE parsing is event-type driven (`eventsource-stream`). The stream
//! accumulates `tool_use` blocks across deltas and yields exactly one
//! [`Delta::ToolCall`] per block on `content_block_stop`. A single
//! [`Delta::Done`] is yielded on `message_stop`.

use std::collections::HashMap;

use async_stream::try_stream;
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::stream::BoxStream;
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::warn;

use crate::error::{parse_retry_after, ProviderError};
use crate::provider::{
    CacheControl, Capabilities, ChatRequest, Delta, EmbedRequest, EmbedResponse, Provider, Role,
    StopReason, ToolSchema, Usage,
};
use crate::secrets;

const API_BASE: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL_SMART: &str = "claude-sonnet-4-6";
const DEFAULT_MODEL_FAST: &str = "claude-haiku-4-5-20251001";
const DEFAULT_MAX_TOKENS: u32 = 4096;

#[derive(Clone)]
pub struct AnthropicProvider {
    http: Client,
    model_smart: String,
    model_fast: String,
}

impl AnthropicProvider {
    /// Construct a provider, loading the key from the env var. Fails with
    /// [`ProviderError::Auth`] if no key is set so `AppState::build` can decide
    /// whether to register the provider.
    pub fn from_env() -> Result<Self, ProviderError> {
        let key = secrets::anthropic_api_key()?;
        Self::new(&key)
    }

    pub fn new(api_key: &str) -> Result<Self, ProviderError> {
        let mut headers = HeaderMap::new();
        let mut key_value = HeaderValue::from_str(api_key)
            .map_err(|_| ProviderError::Auth)?;
        // Mark the secret so any future tracing layer that inspects headers
        // does not accidentally log it.
        key_value.set_sensitive(true);
        headers.insert("x-api-key", key_value);
        headers.insert("anthropic-version", HeaderValue::from_static(API_VERSION));
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        let http = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(ProviderError::Network)?;

        Ok(Self {
            http,
            model_smart: DEFAULT_MODEL_SMART.to_string(),
            model_fast: DEFAULT_MODEL_FAST.to_string(),
        })
    }

    pub fn with_models(mut self, smart: impl Into<String>, fast: impl Into<String>) -> Self {
        self.model_smart = smart.into();
        self.model_fast = fast.into();
        self
    }

    pub fn smart_model(&self) -> &str {
        &self.model_smart
    }

    pub fn fast_model(&self) -> &str {
        &self.model_fast
    }

    fn pick_model(&self, req: &ChatRequest) -> String {
        if let Some(m) = req.model.as_deref().filter(|m| !m.is_empty()) {
            return m.to_string();
        }
        use crate::provider::ModelHint::*;
        match req.model_hint {
            Fast => self.model_fast.clone(),
            Smart | Auto | Local => self.model_smart.clone(),
        }
    }

    fn build_body(&self, req: &ChatRequest) -> Value {
        let model = self.pick_model(req);
        let max_tokens = req.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);

        let system_blocks: Vec<Value> = req
            .system
            .iter()
            .map(|b| {
                let mut v = json!({ "type": "text", "text": b.text });
                if let Some(CacheControl::Ephemeral) = b.cache_control {
                    v["cache_control"] = json!({ "type": "ephemeral" });
                }
                v
            })
            .collect();

        let messages: Vec<Value> = req
            .messages
            .iter()
            .map(|m| {
                json!({
                    "role": role_to_anthropic(m.role),
                    "content": [{ "type": "text", "text": m.content }],
                })
            })
            .collect();

        let mut body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "stream": true,
            "messages": messages,
        });

        if !system_blocks.is_empty() {
            body["system"] = Value::Array(system_blocks);
        }
        if let Some(t) = req.temperature {
            body["temperature"] = json!(t);
        }
        if !req.tools.is_empty() {
            body["tools"] = Value::Array(tools_to_anthropic(&req.tools));
        }
        body
    }
}

fn role_to_anthropic(role: Role) -> &'static str {
    match role {
        Role::Assistant => "assistant",
        // Anthropic uses "user" for tool results too; the orchestrator pre-wraps them.
        Role::User | Role::Tool => "user",
        Role::System => "user", // System content goes into the top-level `system` field, not messages.
    }
}

fn tools_to_anthropic(tools: &[ToolSchema]) -> Vec<Value> {
    tools
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema,
            })
        })
        .collect()
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            supports_tools: true,
            supports_streaming: true,
            supports_prompt_caching: true,
            supports_embeddings: false,
            max_input_tokens: 200_000,
            max_output_tokens: 8_192,
            local: false,
        }
    }

    async fn complete(
        &self,
        req: ChatRequest,
    ) -> Result<BoxStream<'static, Result<Delta, ProviderError>>, ProviderError> {
        let body = self.build_body(&req);
        let resp = self.http.post(API_BASE).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let retry_after = parse_retry_after(
                resp.headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok()),
            );
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::from_status(status, retry_after, body));
        }

        let bytes = resp.bytes_stream();
        let stream = try_stream! {
            let mut sse = bytes.eventsource();
            let mut state = AnthropicStreamState::default();
            while let Some(event) = sse.next().await {
                let event = event.map_err(|e| {
                    ProviderError::SchemaDrift(format!("sse: {e}"))
                })?;
                if event.data.is_empty() { continue; }
                let event_name = if event.event.is_empty() { "message".to_string() } else { event.event };
                match parse_event(&event_name, &event.data, &mut state)? {
                    EventOutcome::None => {}
                    EventOutcome::Emit(deltas) => {
                        for d in deltas { yield d; }
                    }
                }
            }
        };
        Ok(Box::pin(stream))
    }

    async fn embed(&self, _req: EmbedRequest) -> Result<EmbedResponse, ProviderError> {
        Err(ProviderError::Unsupported(
            "anthropic does not provide embeddings".into(),
        ))
    }
}

#[derive(Default)]
struct AnthropicStreamState {
    /// Tool-use block accumulators keyed by content_block index.
    pending_tools: HashMap<u32, ToolAccumulator>,
    /// Aggregated usage; populated from message_start and message_delta events.
    usage: Usage,
    /// Captured from message_delta.delta.stop_reason.
    stop_reason: Option<StopReason>,
}

struct ToolAccumulator {
    id: String,
    name: String,
    args_buf: String,
}

enum EventOutcome {
    None,
    Emit(Vec<Delta>),
}

#[derive(Deserialize)]
struct EvtMessageStart {
    message: EvtMessageStartInner,
}
#[derive(Deserialize)]
struct EvtMessageStartInner {
    usage: Option<UsageJson>,
}
#[derive(Deserialize)]
struct EvtContentBlockStart {
    index: u32,
    content_block: ContentBlockJson,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlockJson {
    Text {
        #[serde(default)]
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        #[allow(dead_code)] // accumulator pattern: input arrives via input_json_delta
        input: Value,
    },
    /// Catch-all for future block types so we don't 500 on Anthropic schema changes.
    #[serde(other)]
    Other,
}
#[derive(Deserialize)]
struct EvtContentBlockDelta {
    index: u32,
    delta: ContentDeltaJson,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentDeltaJson {
    TextDelta {
        text: String,
    },
    InputJsonDelta {
        partial_json: String,
    },
    #[serde(other)]
    Other,
}
#[derive(Deserialize)]
struct EvtContentBlockStop {
    index: u32,
}
#[derive(Deserialize)]
struct EvtMessageDelta {
    delta: MessageDeltaJson,
    usage: Option<UsageJson>,
}
#[derive(Deserialize)]
struct MessageDeltaJson {
    stop_reason: Option<String>,
}
#[derive(Deserialize)]
struct EvtError {
    error: EvtErrorInner,
}
#[derive(Deserialize)]
struct EvtErrorInner {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    message: String,
}
#[derive(Deserialize, Default, Clone, Copy)]
struct UsageJson {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
    #[serde(default)]
    cache_creation_input_tokens: Option<u32>,
    #[serde(default)]
    cache_read_input_tokens: Option<u32>,
}

#[derive(Serialize)]
struct _Unused;

fn parse_event(
    name: &str,
    data: &str,
    state: &mut AnthropicStreamState,
) -> Result<EventOutcome, ProviderError> {
    match name {
        "message_start" => {
            let e: EvtMessageStart = serde_json::from_str(data)
                .map_err(|e| ProviderError::SchemaDrift(format!("message_start: {e}")))?;
            if let Some(u) = e.message.usage {
                merge_usage(&mut state.usage, u);
            }
            Ok(EventOutcome::None)
        }
        "content_block_start" => {
            let e: EvtContentBlockStart = serde_json::from_str(data)
                .map_err(|e| ProviderError::SchemaDrift(format!("content_block_start: {e}")))?;
            match e.content_block {
                ContentBlockJson::ToolUse { id, name: tool_name, input: _ } => {
                    // Per Anthropic SSE: tool_use blocks start with an empty `input`
                    // (often `{}`) and the actual arguments arrive as a sequence of
                    // `input_json_delta` chunks. Discard the initial input and
                    // accumulate from deltas only.
                    state.pending_tools.insert(
                        e.index,
                        ToolAccumulator {
                            id,
                            name: tool_name,
                            args_buf: String::new(),
                        },
                    );
                }
                ContentBlockJson::Text { text } => {
                    if !text.is_empty() {
                        return Ok(EventOutcome::Emit(vec![Delta::TextChunk(text)]));
                    }
                }
                ContentBlockJson::Other => {}
            }
            Ok(EventOutcome::None)
        }
        "content_block_delta" => {
            let e: EvtContentBlockDelta = serde_json::from_str(data)
                .map_err(|e| ProviderError::SchemaDrift(format!("content_block_delta: {e}")))?;
            match e.delta {
                ContentDeltaJson::TextDelta { text } => {
                    if text.is_empty() {
                        Ok(EventOutcome::None)
                    } else {
                        Ok(EventOutcome::Emit(vec![Delta::TextChunk(text)]))
                    }
                }
                ContentDeltaJson::InputJsonDelta { partial_json } => {
                    if let Some(acc) = state.pending_tools.get_mut(&e.index) {
                        acc.args_buf.push_str(&partial_json);
                    }
                    Ok(EventOutcome::None)
                }
                ContentDeltaJson::Other => Ok(EventOutcome::None),
            }
        }
        "content_block_stop" => {
            let e: EvtContentBlockStop = serde_json::from_str(data)
                .map_err(|e| ProviderError::SchemaDrift(format!("content_block_stop: {e}")))?;
            if let Some(acc) = state.pending_tools.remove(&e.index) {
                let args: Value = if acc.args_buf.trim().is_empty() {
                    Value::Object(Default::default())
                } else {
                    serde_json::from_str(&acc.args_buf).map_err(|e| {
                        ProviderError::SchemaDrift(format!("tool_use args: {e}"))
                    })?
                };
                return Ok(EventOutcome::Emit(vec![Delta::ToolCall {
                    id: acc.id,
                    name: acc.name,
                    args,
                }]));
            }
            Ok(EventOutcome::None)
        }
        "message_delta" => {
            let e: EvtMessageDelta = serde_json::from_str(data)
                .map_err(|e| ProviderError::SchemaDrift(format!("message_delta: {e}")))?;
            if let Some(u) = e.usage {
                merge_usage(&mut state.usage, u);
            }
            if let Some(s) = e.delta.stop_reason {
                state.stop_reason = Some(StopReason::from_anthropic(&s));
            }
            Ok(EventOutcome::None)
        }
        "message_stop" => {
            let stop_reason = state.stop_reason.unwrap_or(StopReason::EndTurn);
            Ok(EventOutcome::Emit(vec![Delta::Done {
                stop_reason,
                usage: state.usage.clone(),
            }]))
        }
        "error" => {
            let e: EvtError = serde_json::from_str(data).map_err(|e| {
                ProviderError::SchemaDrift(format!("error event: {e}"))
            })?;
            Err(ProviderError::Status {
                code: 0,
                body: format!("{}: {}", e.error.kind, e.error.message),
            })
        }
        "ping" => Ok(EventOutcome::None),
        other => {
            warn!(target: "ai-provider::anthropic", event = other, "unknown sse event");
            Ok(EventOutcome::None)
        }
    }
}

fn merge_usage(dst: &mut Usage, src: UsageJson) {
    // Anthropic reports input_tokens on message_start and output_tokens on
    // message_delta; cache_* on message_start. Take the max so later events
    // overwriting with zeros don't clobber earlier non-zero values.
    if src.input_tokens > dst.input_tokens {
        dst.input_tokens = src.input_tokens;
    }
    if src.output_tokens > dst.output_tokens {
        dst.output_tokens = src.output_tokens;
    }
    if src.cache_creation_input_tokens.is_some() {
        dst.cache_creation_input_tokens = src.cache_creation_input_tokens;
    }
    if src.cache_read_input_tokens.is_some() {
        dst.cache_read_input_tokens = src.cache_read_input_tokens;
    }
}

// Suppress unused-warning for the helper struct kept for symmetry.
#[allow(dead_code)]
fn _force_link() -> _Unused {
    _Unused
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{Message, ModelHint};

    fn parse_sequence(events: &[(&str, &str)]) -> Vec<Delta> {
        let mut state = AnthropicStreamState::default();
        let mut out = Vec::new();
        for (name, data) in events {
            match parse_event(name, data, &mut state).unwrap() {
                EventOutcome::None => {}
                EventOutcome::Emit(ds) => out.extend(ds),
            }
        }
        out
    }

    #[test]
    fn happy_path_text_only() {
        let events = vec![
            (
                "message_start",
                r#"{"message":{"usage":{"input_tokens":12,"output_tokens":0,"cache_creation_input_tokens":4,"cache_read_input_tokens":8}}}"#,
            ),
            (
                "content_block_start",
                r#"{"index":0,"content_block":{"type":"text","text":""}}"#,
            ),
            (
                "content_block_delta",
                r#"{"index":0,"delta":{"type":"text_delta","text":"Hello "}}"#,
            ),
            (
                "content_block_delta",
                r#"{"index":0,"delta":{"type":"text_delta","text":"world"}}"#,
            ),
            ("content_block_stop", r#"{"index":0}"#),
            (
                "message_delta",
                r#"{"delta":{"stop_reason":"end_turn"},"usage":{"input_tokens":0,"output_tokens":7}}"#,
            ),
            ("message_stop", "{}"),
        ];
        let out = parse_sequence(&events);
        assert_eq!(out.len(), 3);
        match &out[0] {
            Delta::TextChunk(s) => assert_eq!(s, "Hello "),
            _ => panic!("expected text chunk"),
        }
        match &out[1] {
            Delta::TextChunk(s) => assert_eq!(s, "world"),
            _ => panic!("expected text chunk"),
        }
        match &out[2] {
            Delta::Done { stop_reason, usage } => {
                assert_eq!(*stop_reason, StopReason::EndTurn);
                assert_eq!(usage.input_tokens, 12);
                assert_eq!(usage.output_tokens, 7);
                assert_eq!(usage.cache_creation_input_tokens, Some(4));
                assert_eq!(usage.cache_read_input_tokens, Some(8));
            }
            _ => panic!("expected done"),
        }
    }

    #[test]
    fn tool_use_accumulates_partial_json() {
        let events = vec![
            (
                "content_block_start",
                r#"{"index":1,"content_block":{"type":"tool_use","id":"toolu_1","name":"search_memory","input":{}}}"#,
            ),
            (
                "content_block_delta",
                r#"{"index":1,"delta":{"type":"input_json_delta","partial_json":"{\"query\":"}}"#,
            ),
            (
                "content_block_delta",
                r#"{"index":1,"delta":{"type":"input_json_delta","partial_json":"\"rust async\"}"}}"#,
            ),
            ("content_block_stop", r#"{"index":1}"#),
            (
                "message_delta",
                r#"{"delta":{"stop_reason":"tool_use"},"usage":{"input_tokens":0,"output_tokens":3}}"#,
            ),
            ("message_stop", "{}"),
        ];
        let out = parse_sequence(&events);
        assert_eq!(out.len(), 2);
        match &out[0] {
            Delta::ToolCall { id, name, args } => {
                assert_eq!(id, "toolu_1");
                assert_eq!(name, "search_memory");
                assert_eq!(args["query"], "rust async");
            }
            _ => panic!("expected tool call"),
        }
        match &out[1] {
            Delta::Done { stop_reason, .. } => assert_eq!(*stop_reason, StopReason::ToolUse),
            _ => panic!("expected done"),
        }
    }

    #[test]
    fn unknown_event_does_not_break_stream() {
        let events = vec![
            (
                "frobnicate",
                r#"{"some":"future"}"#,
            ),
            ("ping", "{}"),
            (
                "content_block_delta",
                r#"{"index":0,"delta":{"type":"text_delta","text":"ok"}}"#,
            ),
            ("message_stop", "{}"),
        ];
        let out = parse_sequence(&events);
        assert_eq!(out.len(), 2);
        match &out[0] {
            Delta::TextChunk(s) => assert_eq!(s, "ok"),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn build_body_attaches_cache_control() {
        use crate::provider::SystemBlock;
        let p = AnthropicProvider::new("sk-test").unwrap();
        let mut req = ChatRequest::new(ModelHint::Smart);
        req.system = vec![
            SystemBlock { text: "preamble".into(), cache_control: None },
            SystemBlock { text: "capabilities".into(), cache_control: Some(CacheControl::Ephemeral) },
        ];
        req.messages = vec![Message { role: Role::User, content: "hi".into() }];
        let body = p.build_body(&req);
        let system = body["system"].as_array().unwrap();
        assert!(system[0].get("cache_control").is_none());
        assert_eq!(
            system[1]["cache_control"]["type"].as_str(),
            Some("ephemeral")
        );
        assert_eq!(body["model"], DEFAULT_MODEL_SMART);
        assert_eq!(body["stream"], true);
    }
}
