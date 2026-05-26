//! Provider selection.
//!
//! Holds the set of registered providers, the routing policy (`PreferLocal` by
//! default per the user's decision), and resolves `ModelHint` to an
//! `Arc<dyn Provider>`. This is **not** a load-balancer / fallback chain — see
//! `docs/ai.md` §3.2 for the deliberate non-features.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::ProviderError;
use crate::provider::{ModelHint, Provider};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RoutingPolicy {
    PreferLocal,
    PreferCloud,
    ExplicitName(String),
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self::PreferLocal
    }
}

#[derive(Clone)]
pub struct Router {
    providers: HashMap<String, Arc<dyn Provider>>,
    /// Insertion order for deterministic fallback within a policy.
    order: Vec<String>,
    chat_default: Option<String>,
    embed_default: Option<String>,
    policy: RoutingPolicy,
}

impl std::fmt::Debug for Router {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Router")
            .field("providers", &self.order)
            .field("chat_default", &self.chat_default)
            .field("embed_default", &self.embed_default)
            .field("policy", &self.policy)
            .finish()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            order: Vec::new(),
            chat_default: None,
            embed_default: None,
            policy: RoutingPolicy::default(),
        }
    }

    pub fn register(&mut self, name: impl Into<String>, provider: Arc<dyn Provider>) {
        let name = name.into();
        if !self.providers.contains_key(&name) {
            self.order.push(name.clone());
        }
        self.providers.insert(name, provider);
    }

    pub fn set_chat_default(&mut self, name: impl Into<String>) {
        self.chat_default = Some(name.into());
    }

    pub fn set_embed_default(&mut self, name: impl Into<String>) {
        self.embed_default = Some(name.into());
    }

    pub fn set_policy(&mut self, policy: RoutingPolicy) {
        self.policy = policy;
    }

    pub fn policy(&self) -> &RoutingPolicy {
        &self.policy
    }

    pub fn names(&self) -> Vec<String> {
        self.order.clone()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(name).cloned()
    }

    pub fn select_chat(&self, hint: &ModelHint) -> Result<Arc<dyn Provider>, ProviderError> {
        if let RoutingPolicy::ExplicitName(name) = &self.policy {
            return self.providers.get(name).cloned().ok_or_else(|| {
                ProviderError::Unsupported(format!("provider '{name}' not registered"))
            });
        }

        // Explicit hint `Local` always prefers a local provider regardless of policy.
        if matches!(hint, ModelHint::Local) {
            if let Some(p) = self.find_first(|c| c.local) {
                return Ok(p);
            }
            return self.fallback("no local provider registered");
        }

        let prefer_local = matches!(self.policy, RoutingPolicy::PreferLocal);
        if prefer_local {
            if let Some(p) = self.find_first(|c| c.local) {
                return Ok(p);
            }
        } else {
            // PreferCloud: try cloud first.
            if let Some(p) = self.find_first(|c| !c.local) {
                return Ok(p);
            }
        }

        // Fall back to the other class, then chat_default, then first registered.
        if let Some(p) = self.find_first(|c| if prefer_local { !c.local } else { c.local }) {
            return Ok(p);
        }
        if let Some(name) = &self.chat_default {
            if let Some(p) = self.providers.get(name).cloned() {
                return Ok(p);
            }
        }
        self.fallback("no provider registered")
    }

    pub fn select_embed(&self) -> Result<Arc<dyn Provider>, ProviderError> {
        if let Some(name) = &self.embed_default {
            if let Some(p) = self.providers.get(name).cloned() {
                if p.capabilities().supports_embeddings {
                    return Ok(p);
                }
            }
        }
        if let Some(p) = self.find_first(|c| c.supports_embeddings) {
            return Ok(p);
        }
        Err(ProviderError::Unsupported(
            "no embedding-capable provider registered".into(),
        ))
    }

    fn find_first<F>(&self, pred: F) -> Option<Arc<dyn Provider>>
    where
        F: Fn(&crate::provider::Capabilities) -> bool,
    {
        for name in &self.order {
            if let Some(p) = self.providers.get(name) {
                if pred(&p.capabilities()) {
                    return Some(p.clone());
                }
            }
        }
        None
    }

    fn fallback(&self, reason: &str) -> Result<Arc<dyn Provider>, ProviderError> {
        Err(ProviderError::Unsupported(reason.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ProviderError;
    use crate::provider::{
        Capabilities, ChatRequest, Delta, EmbedRequest, EmbedResponse, Provider,
    };
    use async_trait::async_trait;
    use futures::stream::BoxStream;

    #[derive(Clone)]
    struct MockProvider {
        name: &'static str,
        caps: Capabilities,
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &'static str {
            self.name
        }
        fn capabilities(&self) -> Capabilities {
            self.caps.clone()
        }
        async fn complete(
            &self,
            _req: ChatRequest,
        ) -> Result<BoxStream<'static, Result<Delta, ProviderError>>, ProviderError> {
            Err(ProviderError::Unsupported("mock".into()))
        }
        async fn embed(&self, _req: EmbedRequest) -> Result<EmbedResponse, ProviderError> {
            Err(ProviderError::Unsupported("mock".into()))
        }
    }

    fn cloud() -> Arc<dyn Provider> {
        Arc::new(MockProvider {
            name: "anthropic",
            caps: Capabilities {
                supports_tools: true,
                supports_streaming: true,
                supports_prompt_caching: true,
                supports_embeddings: false,
                max_input_tokens: 200_000,
                max_output_tokens: 8_192,
                local: false,
            },
        })
    }

    fn local() -> Arc<dyn Provider> {
        Arc::new(MockProvider {
            name: "ollama",
            caps: Capabilities {
                supports_tools: true,
                supports_streaming: true,
                supports_prompt_caching: false,
                supports_embeddings: true,
                max_input_tokens: 32_768,
                max_output_tokens: 4_096,
                local: true,
            },
        })
    }

    #[test]
    fn default_policy_is_prefer_local() {
        let r = Router::new();
        assert_eq!(*r.policy(), RoutingPolicy::PreferLocal);
    }

    #[test]
    fn prefer_local_resolves_to_local_when_smart() {
        let mut r = Router::new();
        r.register("anthropic", cloud());
        r.register("ollama", local());
        let p = r.select_chat(&ModelHint::Smart).unwrap();
        assert_eq!(p.name(), "ollama");
    }

    #[test]
    fn prefer_cloud_resolves_to_cloud_when_smart() {
        let mut r = Router::new();
        r.register("anthropic", cloud());
        r.register("ollama", local());
        r.set_policy(RoutingPolicy::PreferCloud);
        let p = r.select_chat(&ModelHint::Smart).unwrap();
        assert_eq!(p.name(), "anthropic");
    }

    #[test]
    fn hint_local_always_picks_local() {
        let mut r = Router::new();
        r.register("anthropic", cloud());
        r.register("ollama", local());
        r.set_policy(RoutingPolicy::PreferCloud);
        let p = r.select_chat(&ModelHint::Local).unwrap();
        assert_eq!(p.name(), "ollama");
    }

    #[test]
    fn explicit_name_overrides_hint() {
        let mut r = Router::new();
        r.register("anthropic", cloud());
        r.register("ollama", local());
        r.set_policy(RoutingPolicy::ExplicitName("ollama".into()));
        let p = r.select_chat(&ModelHint::Smart).unwrap();
        assert_eq!(p.name(), "ollama");
    }

    #[test]
    fn explicit_name_missing_errors() {
        let mut r = Router::new();
        r.register("ollama", local());
        r.set_policy(RoutingPolicy::ExplicitName("anthropic".into()));
        match r.select_chat(&ModelHint::Smart) {
            Err(ProviderError::Unsupported(_)) => {}
            Ok(_) => panic!("expected Unsupported, got Ok"),
            Err(e) => panic!("expected Unsupported, got {}", e.code()),
        }
    }

    #[test]
    fn empty_router_errors() {
        let r = Router::new();
        match r.select_chat(&ModelHint::Smart) {
            Err(ProviderError::Unsupported(_)) => {}
            Ok(_) => panic!("expected Unsupported, got Ok"),
            Err(e) => panic!("expected Unsupported, got {}", e.code()),
        }
        match r.select_embed() {
            Err(ProviderError::Unsupported(_)) => {}
            Ok(_) => panic!("expected Unsupported, got Ok"),
            Err(e) => panic!("expected Unsupported, got {}", e.code()),
        }
    }

    #[test]
    fn select_embed_prefers_embed_default_then_capability() {
        let mut r = Router::new();
        r.register("anthropic", cloud());
        r.register("ollama", local());
        let p = r.select_embed().unwrap();
        assert_eq!(p.name(), "ollama");
        r.set_embed_default("anthropic"); // anthropic doesn't support embeddings; falls through
        let p2 = r.select_embed().unwrap();
        assert_eq!(p2.name(), "ollama");
    }

    #[test]
    fn fallback_when_preferred_class_missing() {
        // Only cloud registered; PreferLocal should still find it.
        let mut r = Router::new();
        r.register("anthropic", cloud());
        let p = r.select_chat(&ModelHint::Smart).unwrap();
        assert_eq!(p.name(), "anthropic");
    }
}
