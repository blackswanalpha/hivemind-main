use std::sync::Arc;

use hivemind_ai_orchestrator::{AiPersistence, AiSettings, Orchestrator, PersistenceError};
use hivemind_ai_provider::{
    AnthropicProvider, OllamaProvider, Provider, Router, RoutingPolicy, ProviderError,
};
use hivemind_browser_core::{Session, SessionStore};
use hivemind_ipc_types::AppStartedPayload;
use hivemind_storage::{
    default_db_path, open_pool, SqliteAiPersistence, SqliteSessionStore, StorageError,
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Store(#[from] hivemind_browser_core::StoreError),
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
}

pub struct AppState {
    pub store: Arc<SqliteSessionStore>,
    pub session: Arc<RwLock<Session>>,
    pub orchestrator: Arc<Orchestrator>,
}

impl AppState {
    pub async fn build() -> Result<Self, InitError> {
        let path = default_db_path()?;
        tracing::info!(?path, "opening hivemind database");
        let pool = open_pool(&path).await?;
        let store = Arc::new(SqliteSessionStore::new(pool.clone()));
        let session = store.load_session().await?;
        let tabs: usize = session
            .tabs_by_workspace
            .values()
            .map(Vec::len)
            .sum();
        tracing::info!(
            workspaces = session.workspaces.len(),
            tabs,
            "loaded session"
        );

        // AI subsystem wiring.
        let persistence: Arc<dyn AiPersistence> =
            Arc::new(SqliteAiPersistence::new(pool));
        let router = build_router(&*persistence).await;
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(RwLock::new(router)), persistence));

        Ok(Self {
            store,
            session: Arc::new(RwLock::new(session)),
            orchestrator,
        })
    }
}

async fn build_router(persistence: &dyn AiPersistence) -> Router {
    let mut router = Router::new();

    // Ollama: registration is cheap; surface connection errors on first call.
    match OllamaProvider::new(OllamaProvider::default_base()) {
        Ok(p) => {
            let p: Arc<dyn Provider> = Arc::new(p);
            router.register("ollama", p);
            router.set_embed_default("ollama");
            info!("registered Ollama provider at default localhost base");
        }
        Err(e) => warn!(error = ?e, "failed to construct Ollama provider"),
    }

    // Anthropic: only register if the env var is set.
    match AnthropicProvider::from_env() {
        Ok(p) => {
            let p: Arc<dyn Provider> = Arc::new(p);
            router.register("anthropic", p);
            info!("registered Anthropic provider from env var");
        }
        Err(ProviderError::Auth) => {
            info!("ANTHROPIC_API_KEY not set; skipping Anthropic registration");
        }
        Err(e) => warn!(error = ?e, "failed to construct Anthropic provider"),
    }

    // Apply persisted settings if any; otherwise honour the locked default
    // (PreferLocal + ollama as chat default).
    match AiSettings::load(persistence).await {
        Ok(settings) => {
            router.set_chat_default(&settings.provider);
            router.set_policy(settings.to_routing_policy());
            info!(provider = %settings.provider, policy = ?settings.policy, "applied persisted ai settings");
        }
        Err(e) => {
            warn!(error = ?e, "could not load ai_settings; using defaults");
            router.set_chat_default("ollama");
            router.set_policy(RoutingPolicy::PreferLocal);
        }
    }

    router
}

/// Called from the Tauri setup hook. Builds the app state, manages it, and
/// emits `AppStarted` once everything is wired.
pub async fn initialize(handle: &AppHandle) -> Result<(), InitError> {
    let state = AppState::build().await?;
    handle.manage(state);
    let payload = AppStartedPayload {
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    if let Err(err) = handle.emit("AppStarted", payload) {
        tracing::warn!(error = ?err, "failed to emit AppStarted");
    }
    Ok(())
}
