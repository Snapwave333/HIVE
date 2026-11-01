use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum HiveEvent {
    #[serde(rename = "agent.message.created")]
    MessageCreated {
        message_id: String,
        channel_id: i64,
        sender: String,
        sender_role: Option<String>,
        content: String,
        timestamp: String,
    },
    #[serde(rename = "agent.typing")]
    AgentTyping {
        channel_id: i64,
        codename: String,
        activity: String,
        is_typing: bool,
    },
    #[serde(rename = "agent.status.changed")]
    AgentStatusChanged {
        codename: String,
        status: String,
    },
    #[serde(rename = "task.created")]
    TaskCreated {
        task_id: String,
        title: String,
        assignee: String,
    },
    #[serde(rename = "task.step_completed")]
    TaskStepCompleted {
        task_id: String,
        step_index: usize,
        checkpoint: Option<String>,
    },
    #[serde(rename = "decision.proposed")]
    DecisionProposed {
        decision_id: String,
        title: String,
        level: u8,
        proposer: String,
    },
    #[serde(rename = "decision.resolved")]
    DecisionResolved {
        decision_id: String,
        approved: bool,
        vote_summary: String,
    },
    #[serde(rename = "memory.created")]
    MemoryCreated {
        memory_id: String,
        category: String,
        content: String,
    },
    #[serde(rename = "system.lifecycle")]
    SystemLifecycle {
        state: String, // "running" | "paused" | "stopped"
        timestamp: String,
    },
    #[serde(rename = "audio.mute_toggled")]
    AudioMuteToggled {
        is_muted: bool,
    },
    #[serde(rename = "ui.deployment")]
    UiDeploymentEvent {
        version: String,
        status: String,
        author_agent: String,
    },
}

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<HiveEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: HiveEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<HiveEvent> {
        self.sender.subscribe()
    }

    pub fn emit_message(&self, message_id: &str, channel_id: i64, sender: &str, content: &str) {
        self.publish(HiveEvent::MessageCreated {
            message_id: message_id.to_string(),
            channel_id,
            sender: sender.to_string(),
            sender_role: None,
            content: content.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        });
    }

    pub fn emit_stored_message(&self, msg: &crate::db::models::Message) {
        self.publish(HiveEvent::MessageCreated {
            message_id: msg.id.clone(),
            channel_id: msg.channel_id,
            sender: msg.sender.clone(),
            sender_role: msg.sender_role.clone(),
            content: msg.content.clone(),
            timestamp: msg.created_at.clone(),
        });
    }

    pub fn emit_typing(&self, channel_id: i64, codename: &str, activity: &str, is_typing: bool) {
        self.publish(HiveEvent::AgentTyping {
            channel_id,
            codename: codename.to_string(),
            activity: activity.to_string(),
            is_typing,
        });
    }
}
