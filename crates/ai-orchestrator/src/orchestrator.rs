//! Single-turn chat orchestrator (P0 — no tool loop, no memory recall).
//!
//! Owns the in-memory `ConversationState` map, persists user/assistant
//! messages via [`AiPersistence`], and pumps `Delta`s from the provider as
//! Tauri-friendly callbacks.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use futures::StreamExt;
use hivemind_ai_provider::{
    ChatRequest, Delta, ModelHint, ProviderError, Role, Router, StopReason,
};
use hivemind_ipc_types::{
    ChatCompletePayload, ChatErrorPayload, ChatTokenPayload, ConversationInfo, MessageInfo,
    UsageInfo,
};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::error::OrchestratorError;
use crate::persistence::{AiPersistence, ConversationRecord, MessageRecord};
use crate::state::{ConversationState, TurnMessage};
use crate::system_prompt::{assemble_system_prompt, SystemPromptLayers};

pub type TokenCallback = Arc<dyn Fn(ChatTokenPayload) + Send + Sync + 'static>;
pub type CompleteCallback = Arc<dyn Fn(ChatCompletePayload) + Send + Sync + 'static>;
pub type ErrorCallback = Arc<dyn Fn(ChatErrorPayload) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct StreamCallbacks {
    pub on_token: TokenCallback,
    pub on_complete: CompleteCallback,
    pub on_error: ErrorCallback,
}

type StateMap = Arc<RwLock<HashMap<String, ConversationState>>>;

pub struct Orchestrator {
    router: Arc<RwLock<Router>>,
    persistence: Arc<dyn AiPersistence>,
    states: StateMap,
}

#[derive(Clone, Debug)]
pub struct SendOutcome {
    pub user_message_id: String,
    pub assistant_message_id: String,
}

impl Orchestrator {
    pub fn new(router: Arc<RwLock<Router>>, persistence: Arc<dyn AiPersistence>) -> Self {
        Self {
            router,
            persistence,
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn router(&self) -> Arc<RwLock<Router>> {
        self.router.clone()
    }

    pub fn persistence(&self) -> Arc<dyn AiPersistence> {
        self.persistence.clone()
    }

    pub async fn create_conversation(
        &self,
        workspace_id: &str,
    ) -> Result<ConversationInfo, OrchestratorError> {
        let id = self.persistence.create_conversation(workspace_id).await?;
        let started_at = Utc::now();
        let info = ConversationInfo {
            id: id.clone(),
            workspace_id: workspace_id.to_string(),
            started_at: started_at.timestamp(),
            preview: None,
        };
        let state = ConversationState::new(id.clone(), workspace_id);
        self.states.write().await.insert(id, state);
        Ok(info)
    }

    pub async fn list_conversations(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<ConversationInfo>, OrchestratorError> {
        let records = self.persistence.list_conversations(workspace_id).await?;
        let mut out = Vec::with_capacity(records.len());
        for c in records {
            let preview = self
                .persistence
                .first_user_message(&c.id)
                .await
                .ok()
                .flatten();
            out.push(conversation_to_info(&c, preview));
        }
        Ok(out)
    }

    pub async fn delete_conversation(&self, conversation_id: &str) -> Result<(), OrchestratorError> {
        self.persistence.delete_conversation(conversation_id).await?;
        self.states.write().await.remove(conversation_id);
        Ok(())
    }

    pub async fn load_messages(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MessageInfo>, OrchestratorError> {
        let rows = self.persistence.load_messages(conversation_id).await?;
        let mut states = self.states.write().await;
        let entry = states
            .entry(conversation_id.to_string())
            .or_insert_with(|| ConversationState::new(conversation_id, ""));
        entry.messages = rows.iter().map(message_record_to_turn).collect();
        drop(states);
        Ok(rows.iter().map(message_record_to_info).collect())
    }

    /// Send a single user turn. Persists the user message immediately, fires
    /// the streaming task in the background, returns [`SendOutcome`] with
    /// stable IDs so the frontend can attach pending bubbles before the first
    /// token arrives.
    pub async fn send_message(
        &self,
        conversation_id: &str,
        text: String,
        hint: ModelHint,
        callbacks: StreamCallbacks,
    ) -> Result<SendOutcome, OrchestratorError> {
        let workspace_id = self.ensure_state(conversation_id).await?;
        let user_id = self
            .persistence
            .append_message(conversation_id, "user", &text)
            .await?;

        let history_snapshot = {
            let mut states = self.states.write().await;
            let state = states.get_mut(conversation_id).ok_or_else(|| {
                OrchestratorError::NoSuchConversation(conversation_id.to_string())
            })?;
            state
                .messages
                .push(TurnMessage::now(Role::User, text.clone()));
            state.model_hint = hint;
            state.messages.clone()
        };

        let provider = {
            let router = self.router.read().await;
            router
                .select_chat(&hint)
                .map_err(OrchestratorError::Provider)?
        };

        let assistant_id = ulid::Ulid::new().to_string();
        let req = build_request(hint, &workspace_id, &history_snapshot);

        let outcome = SendOutcome {
            user_message_id: user_id.to_string(),
            assistant_message_id: assistant_id.clone(),
        };

        let stream_result = provider.complete(req).await;
        match stream_result {
            Ok(mut stream) => {
                let states_handle = self.states.clone();
                let persistence = self.persistence.clone();
                let conv_id = conversation_id.to_string();
                let assistant_clone = assistant_id.clone();
                let cb = callbacks;
                tokio::spawn(async move {
                    let mut buffer = String::new();
                    let mut last_usage = UsageInfo::default();
                    let mut stop_reason: Option<StopReason> = None;
                    while let Some(item) = stream.next().await {
                        match item {
                            Ok(Delta::TextChunk(s)) => {
                                buffer.push_str(&s);
                                (cb.on_token)(ChatTokenPayload {
                                    conversation_id: conv_id.clone(),
                                    message_id: assistant_clone.clone(),
                                    delta: s,
                                });
                            }
                            Ok(Delta::ToolCall { name, .. }) => {
                                warn!(
                                    target: "ai-orchestrator",
                                    tool = %name,
                                    "tool_call in P0 ignored"
                                );
                            }
                            Ok(Delta::Done { stop_reason: sr, usage }) => {
                                stop_reason = Some(sr);
                                last_usage = UsageInfo {
                                    input_tokens: usage.input_tokens,
                                    output_tokens: usage.output_tokens,
                                    cache_creation_input_tokens: usage.cache_creation_input_tokens,
                                    cache_read_input_tokens: usage.cache_read_input_tokens,
                                };
                            }
                            Err(e) => {
                                error!(
                                    target: "ai-orchestrator",
                                    error = %e,
                                    "provider error mid-stream"
                                );
                                emit_error(&cb, &conv_id, &assistant_clone, &e);
                                return;
                            }
                        }
                    }
                    if let Err(e) = persistence
                        .append_message(&conv_id, "assistant", &buffer)
                        .await
                    {
                        error!(
                            target: "ai-orchestrator",
                            error = %e,
                            "failed to persist assistant message"
                        );
                    }
                    {
                        let mut states = states_handle.write().await;
                        if let Some(state) = states.get_mut(&conv_id) {
                            state
                                .messages
                                .push(TurnMessage::now(Role::Assistant, buffer.clone()));
                        }
                    }
                    let stop = stop_reason.unwrap_or(StopReason::EndTurn);
                    (cb.on_complete)(ChatCompletePayload {
                        conversation_id: conv_id,
                        message_id: assistant_clone,
                        stop_reason: stop.as_str().to_string(),
                        usage: last_usage,
                    });
                    info!(target: "ai-orchestrator", "send complete");
                });
            }
            Err(e) => {
                emit_error(&callbacks, conversation_id, &assistant_id, &e);
                return Err(OrchestratorError::Provider(e));
            }
        }

        Ok(outcome)
    }

    async fn ensure_state(&self, conversation_id: &str) -> Result<String, OrchestratorError> {
        if let Some(state) = self.states.read().await.get(conversation_id) {
            return Ok(state.workspace_id.clone());
        }
        // Cold load: pull messages to warm history. Workspace id is unknown
        // here; the orchestrator's callers should always create the
        // conversation first (which seeds the state) so this path is only
        // hit after a restart.
        let rows = self.persistence.load_messages(conversation_id).await?;
        let workspace_id = String::new();
        let mut state = ConversationState::new(conversation_id, &workspace_id);
        state.messages = rows.iter().map(message_record_to_turn).collect();
        self.states
            .write()
            .await
            .insert(conversation_id.to_string(), state);
        Ok(workspace_id)
    }
}

fn emit_error(cb: &StreamCallbacks, conv: &str, msg: &str, err: &ProviderError) {
    (cb.on_error)(ChatErrorPayload {
        conversation_id: conv.to_string(),
        message_id: msg.to_string(),
        code: err.code().to_string(),
        message: err.to_string(),
    });
}

fn build_request(hint: ModelHint, workspace_id: &str, history: &[TurnMessage]) -> ChatRequest {
    let mut layers = SystemPromptLayers::default();
    if !workspace_id.is_empty() {
        layers.workspace_context = format!("Active workspace id: {workspace_id}");
    }
    let system = assemble_system_prompt(&layers);
    let messages = history
        .iter()
        .filter(|m| !matches!(m.role, Role::System))
        .map(|m| m.to_provider_message())
        .collect();
    let mut req = ChatRequest::new(hint);
    req.system = system;
    req.messages = messages;
    req
}

fn message_record_to_info(r: &MessageRecord) -> MessageInfo {
    MessageInfo {
        id: r.id.to_string(),
        role: r.role.clone(),
        content: r.content.clone(),
        created_at: r.created_at.timestamp(),
    }
}

fn message_record_to_turn(r: &MessageRecord) -> TurnMessage {
    let role = match r.role.as_str() {
        "assistant" => Role::Assistant,
        "system" => Role::System,
        "tool" => Role::Tool,
        _ => Role::User,
    };
    TurnMessage {
        role,
        content: r.content.clone(),
        created_at: r.created_at,
    }
}

fn conversation_to_info(r: &ConversationRecord, preview: Option<String>) -> ConversationInfo {
    ConversationInfo {
        id: r.id.clone(),
        workspace_id: r.workspace_id.clone(),
        started_at: r.started_at.timestamp(),
        preview,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use hivemind_ai_provider::{
        ChatRequest, EmbedRequest, EmbedResponse, Provider, Router, RoutingPolicy, Usage,
    };
    use futures::stream::{self, BoxStream};
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockPersistence {
        next_msg_id: Mutex<i64>,
        msgs: Mutex<Vec<(String, String, String)>>, // (conv_id, role, content)
        next_conv: Mutex<u32>,
    }

    #[async_trait]
    impl AiPersistence for MockPersistence {
        async fn create_conversation(&self, workspace_id: &str) -> Result<String, crate::persistence::PersistenceError> {
            let mut n = self.next_conv.lock().unwrap();
            *n += 1;
            Ok(format!("conv-{n}-{workspace_id}"))
        }
        async fn list_conversations(
            &self,
            _workspace_id: &str,
        ) -> Result<Vec<ConversationRecord>, crate::persistence::PersistenceError> {
            Ok(vec![])
        }
        async fn delete_conversation(&self, _conv: &str) -> Result<(), crate::persistence::PersistenceError> {
            Ok(())
        }
        async fn load_messages(
            &self,
            _conv: &str,
        ) -> Result<Vec<MessageRecord>, crate::persistence::PersistenceError> {
            Ok(vec![])
        }
        async fn append_message(
            &self,
            conv: &str,
            role: &str,
            content: &str,
        ) -> Result<i64, crate::persistence::PersistenceError> {
            let mut id = self.next_msg_id.lock().unwrap();
            *id += 1;
            self.msgs
                .lock()
                .unwrap()
                .push((conv.to_string(), role.to_string(), content.to_string()));
            Ok(*id)
        }
        async fn first_user_message(
            &self,
            _conv: &str,
        ) -> Result<Option<String>, crate::persistence::PersistenceError> {
            Ok(None)
        }
        async fn get_config(&self, _key: &str) -> Result<Option<String>, crate::persistence::PersistenceError> {
            Ok(None)
        }
        async fn set_config(&self, _key: &str, _value: &str) -> Result<(), crate::persistence::PersistenceError> {
            Ok(())
        }
    }

    struct MockProvider {
        // Each invocation drains the buffer. Tests must seed exactly one
        // turn's worth of deltas.
        deltas: Mutex<Vec<Result<Delta, ProviderError>>>,
    }

    impl MockProvider {
        fn new(deltas: Vec<Result<Delta, ProviderError>>) -> Self {
            Self { deltas: Mutex::new(deltas) }
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &'static str {
            "mock"
        }
        fn capabilities(&self) -> hivemind_ai_provider::Capabilities {
            hivemind_ai_provider::Capabilities {
                supports_tools: false,
                supports_streaming: true,
                supports_prompt_caching: false,
                supports_embeddings: false,
                max_input_tokens: 8192,
                max_output_tokens: 1024,
                local: true,
            }
        }
        async fn complete(
            &self,
            _req: ChatRequest,
        ) -> Result<BoxStream<'static, Result<Delta, ProviderError>>, ProviderError> {
            let v = std::mem::take(&mut *self.deltas.lock().unwrap());
            Ok(Box::pin(stream::iter(v)))
        }
        async fn embed(&self, _req: EmbedRequest) -> Result<EmbedResponse, ProviderError> {
            Err(ProviderError::Unsupported("no".into()))
        }
    }

    fn router_with(provider: Arc<dyn Provider>) -> Arc<RwLock<Router>> {
        let mut r = Router::new();
        r.register("mock", provider);
        r.set_policy(RoutingPolicy::PreferLocal);
        Arc::new(RwLock::new(r))
    }

    fn callbacks() -> (
        StreamCallbacks,
        Arc<Mutex<Vec<String>>>,
        Arc<Mutex<Vec<ChatCompletePayload>>>,
        Arc<Mutex<Vec<ChatErrorPayload>>>,
    ) {
        let tokens = Arc::new(Mutex::new(Vec::<String>::new()));
        let completes = Arc::new(Mutex::new(Vec::<ChatCompletePayload>::new()));
        let errors = Arc::new(Mutex::new(Vec::<ChatErrorPayload>::new()));
        let tok = tokens.clone();
        let comp = completes.clone();
        let err = errors.clone();
        let cb = StreamCallbacks {
            on_token: Arc::new(move |p| tok.lock().unwrap().push(p.delta)),
            on_complete: Arc::new(move |p| comp.lock().unwrap().push(p)),
            on_error: Arc::new(move |p| err.lock().unwrap().push(p)),
        };
        (cb, tokens, completes, errors)
    }

    #[tokio::test]
    async fn send_happy_path_streams_tokens_and_persists() {
        let provider: Arc<dyn Provider> = Arc::new(MockProvider::new(vec![
            Ok(Delta::TextChunk("Hel".into())),
            Ok(Delta::TextChunk("lo ".into())),
            Ok(Delta::TextChunk("world".into())),
            Ok(Delta::Done {
                stop_reason: StopReason::EndTurn,
                usage: Usage {
                    input_tokens: 5,
                    output_tokens: 3,
                    ..Default::default()
                },
            }),
        ]));
        let router = router_with(provider);
        let persistence: Arc<dyn AiPersistence> = Arc::new(MockPersistence::default());
        let orch = Orchestrator::new(router, persistence.clone());
        let conv = orch.create_conversation("ws-1").await.unwrap();
        let (cb, tokens, completes, errors) = callbacks();
        let outcome = orch
            .send_message(&conv.id, "hi".into(), ModelHint::Local, cb)
            .await
            .unwrap();
        assert!(!outcome.user_message_id.is_empty());
        assert!(!outcome.assistant_message_id.is_empty());
        // Wait for the spawned task to finish.
        for _ in 0..50 {
            if !completes.lock().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let toks = tokens.lock().unwrap();
        assert_eq!(*toks, vec!["Hel", "lo ", "world"]);
        let comps = completes.lock().unwrap();
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].stop_reason, "end_turn");
        assert_eq!(comps[0].usage.input_tokens, 5);
        assert!(errors.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn send_provider_error_fires_on_error_and_returns_err() {
        // Wrapper provider whose complete returns Err immediately so the
        // orchestrator's synchronous-error branch fires.
        struct AuthFail;
        #[async_trait]
        impl Provider for AuthFail {
            fn name(&self) -> &'static str { "fail" }
            fn capabilities(&self) -> hivemind_ai_provider::Capabilities {
                hivemind_ai_provider::Capabilities { local: true, supports_streaming: true, ..Default::default() }
            }
            async fn complete(&self, _: ChatRequest) -> Result<BoxStream<'static, Result<Delta, ProviderError>>, ProviderError> {
                Err(ProviderError::Auth)
            }
            async fn embed(&self, _: EmbedRequest) -> Result<EmbedResponse, ProviderError> {
                Err(ProviderError::Unsupported("no".into()))
            }
        }
        let router = router_with(Arc::new(AuthFail));
        let persistence: Arc<dyn AiPersistence> = Arc::new(MockPersistence::default());
        let orch = Orchestrator::new(router, persistence.clone());
        let conv = orch.create_conversation("ws-1").await.unwrap();
        let (cb, _toks, comps, errs) = callbacks();
        let res = orch
            .send_message(&conv.id, "hi".into(), ModelHint::Local, cb)
            .await;
        assert!(res.is_err(), "expected Err");
        assert_eq!(errs.lock().unwrap().len(), 1);
        assert!(comps.lock().unwrap().is_empty());
        assert_eq!(errs.lock().unwrap()[0].code, "auth");
    }

    #[tokio::test]
    async fn list_and_delete_conversation_round_trip() {
        let provider: Arc<dyn Provider> = Arc::new(MockProvider::new(vec![Ok(Delta::Done {
            stop_reason: StopReason::EndTurn,
            usage: Default::default(),
        })]));
        let router = router_with(provider);
        let persistence: Arc<dyn AiPersistence> = Arc::new(MockPersistence::default());
        let orch = Orchestrator::new(router, persistence.clone());
        let _conv = orch.create_conversation("ws-1").await.unwrap();
        // MockPersistence::list returns empty; just verify the orchestrator's
        // delete path forwards without error.
        let _ = orch.delete_conversation("nope").await.unwrap();
    }
}
