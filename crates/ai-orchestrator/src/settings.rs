//! AI settings persisted in the `config` table.
//!
//! Single key `ai_settings` holds a JSON blob with provider/model/policy.
//! Mutations live in the Tauri command layer, which also calls `apply` on the
//! shared `Router`.

use serde::{Deserialize, Serialize};

use hivemind_ai_provider::{ModelHint, RoutingPolicy};

use crate::persistence::{AiPersistence, PersistenceError};

pub const SETTINGS_KEY: &str = "ai_settings";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiSettings {
    /// Provider name selected as chat default (e.g. `"ollama"`, `"anthropic"`).
    pub provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub policy: PolicyChoice,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            model: None,
            policy: PolicyChoice::PreferLocal,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyChoice {
    PreferLocal,
    PreferCloud,
    ExplicitName,
}

impl PolicyChoice {
    pub fn as_str(&self) -> &'static str {
        match self {
            PolicyChoice::PreferLocal => "prefer_local",
            PolicyChoice::PreferCloud => "prefer_cloud",
            PolicyChoice::ExplicitName => "explicit_name",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "prefer_local" => Some(Self::PreferLocal),
            "prefer_cloud" => Some(Self::PreferCloud),
            "explicit_name" => Some(Self::ExplicitName),
            _ => None,
        }
    }
}

impl AiSettings {
    pub fn to_routing_policy(&self) -> RoutingPolicy {
        match self.policy {
            PolicyChoice::PreferLocal => RoutingPolicy::PreferLocal,
            PolicyChoice::PreferCloud => RoutingPolicy::PreferCloud,
            PolicyChoice::ExplicitName => RoutingPolicy::ExplicitName(self.provider.clone()),
        }
    }

    pub async fn load(p: &dyn AiPersistence) -> Result<Self, PersistenceError> {
        let raw = p.get_config(SETTINGS_KEY).await?;
        match raw {
            Some(s) => serde_json::from_str(&s)
                .map_err(|e| PersistenceError::new(anyhow::anyhow!("ai_settings json: {e}"))),
            None => Ok(Self::default()),
        }
    }

    pub async fn save(&self, p: &dyn AiPersistence) -> Result<(), PersistenceError> {
        let s = serde_json::to_string(self)
            .map_err(|e| PersistenceError::new(anyhow::anyhow!("ai_settings ser: {e}")))?;
        p.set_config(SETTINGS_KEY, &s).await
    }
}

pub fn hint_from_str(s: Option<&str>) -> ModelHint {
    match s.map(str::to_ascii_lowercase).as_deref() {
        Some("fast") => ModelHint::Fast,
        Some("smart") => ModelHint::Smart,
        Some("local") => ModelHint::Local,
        _ => ModelHint::Auto,
    }
}
