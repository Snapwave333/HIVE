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
  notes?: string;
  updated_at: string;
}

export interface TempSpecialist {
  codename: string;
  display_name: string;
  role: string;
  sponsor_agent: string;
  permissions: string;
  expires_at: string;
  status: string;
  created_at: string;
}

export interface AttachmentRef {
  hash: string;
  filename: string;
  mime_type: string;
  size_bytes: number;
}

export interface AttachmentRecord {
  hash: string;
  filename: string;
  mime_type: string;
  size_bytes: number;
  file_path: string;
  data_url?: string;
  created_at: string;
}

export interface UiDeployment {
  version: string;
  directory_path: string;
  sha256_hash: string;
  author_agent: string;
  verification_log?: string;
  is_active: boolean;
  created_at: string;
}

export interface AuditEntry {
  id?: number;
  event_type: string;
  actor: string;
  details: string;
  status: string;
  timestamp: string;
}

export interface BridgeStatus {
  google_connected: boolean;
  google_account: string;
  obsidian_available: boolean;
  obsidian_vault_path: string;
  browser_relay_available: boolean;
  voice_engine_available: boolean;
  llm_provider: string;
}

export interface ConstitutionInvariant {
  immutable: boolean;
  rule: string;
}

export interface Constitution {
  version: string;
  invariants: Record<string, ConstitutionInvariant>;
}
