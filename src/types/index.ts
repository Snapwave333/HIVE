export interface Channel {
  id: number;
  name: string;
  slug: string;
  is_group: boolean;
  agent_codename?: string;
  description?: string;
  pinned: boolean;
  created_at: string;
}

export interface Agent {
  codename: string;
  display_name: string;
  role: string;
  model_tier: string;
  status: "available" | "thinking" | "busy" | "offline";
  avatar_config: string;
  personality?: string;
  skills?: string;
  tools?: string;
  permissions?: string;
  updated_at: string;
}

export interface Message {
  id: string;
  channel_id: number;
  sender: string;
  sender_role?: string;
  content: string;
  attachments?: string;
  reactions?: string;
  reply_to_id?: string;
  thread_root_id?: string;
  is_pinned: boolean;
  is_edited: boolean;
  is_deleted: boolean;
  created_at: string;
  updated_at: string;
}

export interface TaskStep {
  index: number;
  title: string;
  estimated_minutes: number;
  completed: boolean;
  completed_at?: string;
}

export interface TaskChunk {
  id: string;
  title: string;
  description?: string;
  assignee: string;
  priority: "low" | "medium" | "high" | "urgent";
  status: "queued" | "active" | "checkpoint_saved" | "completed" | "failed";
  steps: TaskStep[];
  current_step_index: number;
  resume_checkpoint?: string;
  created_at: string;
  updated_at: string;
}

export interface MemoryItem {
  id: string;
  category: "individual" | "shared" | "relationship" | "operational";
  content: string;
  provenance: string;
  confidence: number;
  is_speculation: boolean;
  tags?: string;
  created_at: string;
}

export interface RelationshipEdge {
  source_agent: string;
  target_agent: string;
  edge_type: string;
  weight: number;
  interaction_count: number;
  last_interaction_at: string;
  notes?: string;
}

export interface TempSpecialist {
  id: string;
  codename: string;
  display_name: string;
  role: string;
  sponsor_agent: string;
  task_id?: string;
  status: string;
  expires_at: string;
  created_at: string;
}

export interface AttachmentRecord {
  id: string;
  message_id?: string;
  filename: string;
  mime_type: string;
  size_bytes: number;
  sha256_hash: string;
  storage_path: string;
  created_at: string;
}

export interface UiDeployment {
  version: string;
  bundle_hash: string;
  author_agent: string;
  changelog: string;
  approval_tier: string;
  status: string;
  previous_version?: string;
  created_at: string;
}

export interface AuditEntry {
  id: number;
  actor: string;
  action: string;
  target?: string;
  details?: string;
  timestamp: string;
}

export interface BridgeStatus {
  google_connected: boolean;
  google_account: string;
  obsidian_available: boolean;
  browser_relay_available: boolean;
  voice_engine_available: boolean;
}

export interface ConstitutionInvariant {
  immutable: boolean;
  rule: string;
}

export interface Constitution {
  version: string;
  invariants: Record<string, ConstitutionInvariant>;
}
