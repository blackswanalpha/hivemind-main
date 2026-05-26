use chrono::{DateTime, Utc};
use hivemind_ai_provider::{Message, ModelHint, Role};

#[derive(Clone, Debug)]
pub struct TurnMessage {
    pub role: Role,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl TurnMessage {
    pub fn now(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            created_at: Utc::now(),
        }
    }

    pub fn to_provider_message(&self) -> Message {
        Message {
            role: self.role,
            content: self.content.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConversationState {
    pub conversation_id: String,
    pub workspace_id: String,
    pub messages: Vec<TurnMessage>,
    pub model_hint: ModelHint,
}

impl ConversationState {
    pub fn new(conversation_id: impl Into<String>, workspace_id: impl Into<String>) -> Self {
        Self {
            conversation_id: conversation_id.into(),
            workspace_id: workspace_id.into(),
            messages: Vec::new(),
            model_hint: ModelHint::Auto,
        }
    }
}
