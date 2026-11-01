use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub is_group: bool,
    pub agent_codename: Option<String>,
    pub description: Option<String>,
    pub pinned: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub codename: String,
    pub display_name: String,
    pub role: String,
    pub model_tier: String,
    pub status: String, // "available" | "thinking" | "busy" | "offline"
    pub avatar_config: String, // JSON string
    pub personality: Option<String>, // JSON string
    pub skills: Option<String>, // JSON string
    pub tools: Option<String>, // JSON string
    pub permissions: Option<String>, // JSON string
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEdge {
    pub source_agent: String,
    pub target_agent: String,
    pub edge_type: String, // "collaborated" | "handed_off" | "disagreed" | "mentored"
    pub weight: f64,
    pub notes: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempSpecialist {
    pub codename: String, // must start with "temp_"
    pub display_name: String,
    pub role: String,
    pub sponsor_agent: String,
    pub permissions: String,
    pub expires_at: String,
    pub status: String, // "active" | "retired"
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentMeta {
    pub hash: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub file_path: String,
    pub data_url: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub channel_id: i64,
    pub sender: String,
    pub sender_role: Option<String>,
    pub content: String,
    pub attachments: Option<String>, // JSON array of AttachmentMeta
    pub reactions: Option<String>,   // JSON object: { "emoji": ["user", "PRIME"] }
    pub reply_to_id: Option<String>,
    pub thread_root_id: Option<String>,
    pub is_pinned: bool,
    pub is_edited: bool,
    pub is_deleted: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    pub index: usize,
    pub title: String,
    pub estimated_minutes: u32,
    pub completed: bool,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskChunk {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub assignee: String,
    pub priority: String, // "low" | "medium" | "high" | "urgent"
    pub status: String,   // "queued" | "active" | "checkpoint_saved" | "completed" | "failed"
    pub steps: Vec<TaskStep>,
    pub current_step_index: usize,
    pub resume_checkpoint: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub category: String, // "individual" | "shared" | "relationship" | "operational"
    pub content: String,
    pub provenance: String,
    pub confidence: f64,
    pub is_speculation: bool,
    pub tags: Option<String>, // comma separated or JSON
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiDeployment {
    pub version: String,
    pub directory_path: String,
    pub sha256_hash: String,
    pub author_agent: String,
    pub verification_log: Option<String>,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: Option<i64>,
    pub event_type: String,
    pub actor: String,
    pub details: String,
    pub status: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRow {
    pub id: String,
    pub title: String,
    pub description: String,
    pub level: u8,
    pub proposer: String,
    pub status: String,
    pub votes_json: String,
    pub dissent_summary: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}
