//! Tauri commands for the AI sidebar (work00 step 07).

use std::sync::Arc;
use std::time::Instant;

use hivemind_ai_orchestrator::{
    hint_from_str, AiSettings, PolicyChoice, StreamCallbacks, SETTINGS_KEY,
};
use hivemind_ai_provider::{
    ChatRequest, EmbedRequest, ModelHint, Provider, ProviderError, Role, RoutingPolicy,
};
use hivemind_ai_provider::Message as ProviderMessage;
use hivemind_ipc_types::{
    AiProviderInfo, AiSettingsPayload, ChatCompletePayload, ChatErrorPayload, ChatTokenPayload,
    ConversationInfo, MessageInfo, TestProviderResult,
};
use tauri::{AppHandle, Emitter, State};
use tracing::warn;

use crate::state::AppState;

fn err_to_str<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[tauri::command]
pub async fn create_conversation(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<ConversationInfo, String> {
    state
        .orchestrator
        .create_conversation(&workspace_id)
        .await
        .map_err(err_to_str)
}

#[tauri::command]
pub async fn list_conversations(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<Vec<ConversationInfo>, String> {
    state
        .orchestrator
        .list_conversations(&workspace_id)
        .await
        .map_err(err_to_str)
}

#[tauri::command]
pub async fn load_messages(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<MessageInfo>, String> {
    state
        .orchestrator
        .load_messages(&conversation_id)
        .await
        .map_err(err_to_str)
}

#[tauri::command]
pub async fn delete_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    state
        .orchestrator
        .delete_conversation(&conversation_id)
        .await
        .map_err(err_to_str)
}

#[tauri::command]
pub async fn send_message(
    app: AppHandle,
    state: State<'_, AppState>,
    conversation_id: String,
    text: String,
    hint: Option<String>,
) -> Result<String, String> {
    let hint = hint_from_str(hint.as_deref());

    let app_token = app.clone();
    let app_complete = app.clone();
    let app_error = app.clone();
    let callbacks = StreamCallbacks {
        on_token: Arc::new(move |p: ChatTokenPayload| {
            if let Err(e) = app_token.emit("hm:chat-token", p) {
                warn!(target: "ai_commands", error = ?e, "failed to emit hm:chat-token");
            }
        }),
        on_complete: Arc::new(move |p: ChatCompletePayload| {
            if let Err(e) = app_complete.emit("hm:chat-complete", p) {
                warn!(target: "ai_commands", error = ?e, "failed to emit hm:chat-complete");
            }
        }),
        on_error: Arc::new(move |p: ChatErrorPayload| {
            if let Err(e) = app_error.emit("hm:chat-error", p) {
                warn!(target: "ai_commands", error = ?e, "failed to emit hm:chat-error");
            }
        }),
    };

    let outcome = state
        .orchestrator
        .send_message(&conversation_id, text, hint, callbacks)
        .await
        .map_err(err_to_str)?;
    Ok(outcome.assistant_message_id)
}

#[tauri::command]
pub async fn list_providers(state: State<'_, AppState>) -> Result<Vec<AiProviderInfo>, String> {
    let router = state.orchestrator.router();
    let r = router.read().await;
    let mut out = Vec::new();
    for name in r.names() {
        if let Some(p) = r.get(&name) {
            let caps = p.capabilities();
            out.push(AiProviderInfo {
                name: name.clone(),
                local: caps.local,
                supports_tools: caps.supports_tools,
                supports_embeddings: caps.supports_embeddings,
                supports_prompt_caching: caps.supports_prompt_caching,
                registered: true,
            });
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn get_ai_settings(state: State<'_, AppState>) -> Result<AiSettingsPayload, String> {
    let persistence = state.orchestrator.persistence();
    let settings = AiSettings::load(&*persistence).await.map_err(err_to_str)?;
    Ok(AiSettingsPayload {
        provider: settings.provider,
        model: settings.model,
        policy: settings.policy.as_str().to_string(),
    })
}

#[tauri::command]
pub async fn set_ai_settings(
    state: State<'_, AppState>,
    settings: AiSettingsPayload,
) -> Result<(), String> {
    let policy = PolicyChoice::from_str(&settings.policy)
        .ok_or_else(|| format!("invalid policy {}", settings.policy))?;
    let new_settings = AiSettings {
        provider: settings.provider.clone(),
        model: settings.model.clone(),
        policy,
    };
    let persistence = state.orchestrator.persistence();
    new_settings.save(&*persistence).await.map_err(err_to_str)?;

    // Apply to the live router.
    let router = state.orchestrator.router();
    let mut r = router.write().await;
    r.set_chat_default(&new_settings.provider);
    r.set_policy(match new_settings.policy {
        PolicyChoice::PreferLocal => RoutingPolicy::PreferLocal,
        PolicyChoice::PreferCloud => RoutingPolicy::PreferCloud,
        PolicyChoice::ExplicitName => RoutingPolicy::ExplicitName(new_settings.provider.clone()),
    });
    Ok(())
}

#[tauri::command]
pub async fn test_provider(
    state: State<'_, AppState>,
    name: String,
) -> Result<TestProviderResult, String> {
    let router = state.orchestrator.router();
    let provider: Arc<dyn Provider> = {
        let r = router.read().await;
        match r.get(&name) {
            Some(p) => p,
            None => {
                return Ok(TestProviderResult {
                    ok: false,
                    latency_ms: None,
                    error: Some(format!("provider '{name}' not registered")),
                })
            }
        }
    };

    let started = Instant::now();
    let result = if provider.capabilities().supports_embeddings {
        // Cheaper probe for local: round-trip the embed endpoint with one tiny string.
        provider
            .embed(EmbedRequest {
                texts: vec!["ping".to_string()],
                model: None,
            })
            .await
            .map(|_| ())
    } else {
        // For cloud chat: open a tiny streaming completion and drop it after first delta.
        let mut req = ChatRequest::new(ModelHint::Fast);
        req.max_tokens = Some(8);
        req.messages = vec![ProviderMessage {
            role: Role::User,
            content: "ping".to_string(),
        }];
        match provider.complete(req).await {
            Ok(mut stream) => {
                use futures::StreamExt;
                let _ = stream.next().await;
                Ok(())
            }
            Err(e) => Err(e),
        }
    };
    let elapsed = started.elapsed().as_millis() as u32;
    Ok(match result {
        Ok(_) => TestProviderResult {
            ok: true,
            latency_ms: Some(elapsed),
            error: None,
        },
        Err(e) => TestProviderResult {
            ok: false,
            latency_ms: Some(elapsed),
            error: Some(provider_error_to_message(&e)),
        },
    })
}

fn provider_error_to_message(e: &ProviderError) -> String {
    format!("{} — {e}", e.code())
}

#[allow(dead_code)]
fn _force_link() -> &'static str {
    SETTINGS_KEY
}
