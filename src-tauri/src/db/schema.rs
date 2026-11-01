pub const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS channels (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    is_group BOOLEAN NOT NULL DEFAULT 0,
    agent_codename TEXT,
    description TEXT,
    pinned BOOLEAN NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS agents (
    codename TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    role TEXT NOT NULL,
    model_tier TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'available',
    avatar_config TEXT NOT NULL,
    personality TEXT,
    skills TEXT,
    tools TEXT,
    permissions TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS relationships (
    source_agent TEXT NOT NULL,
    target_agent TEXT NOT NULL,
    edge_type TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 1.0,
    notes TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (source_agent, target_agent, edge_type)
);

CREATE INDEX IF NOT EXISTS idx_rel_source ON relationships(source_agent);
CREATE INDEX IF NOT EXISTS idx_rel_target ON relationships(target_agent);

CREATE TABLE IF NOT EXISTS temp_specialists (
    codename TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    role TEXT NOT NULL,
    sponsor_agent TEXT NOT NULL,
    permissions TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL,
    FOREIGN KEY(sponsor_agent) REFERENCES agents(codename)
);

CREATE TABLE IF NOT EXISTS attachments (
    hash TEXT PRIMARY KEY,
    filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    channel_id INTEGER NOT NULL,
    sender TEXT NOT NULL,
    sender_role TEXT,
    content TEXT NOT NULL,
    attachments TEXT,
    reactions TEXT,
    reply_to_id TEXT,
    thread_root_id TEXT,
    is_pinned BOOLEAN NOT NULL DEFAULT 0,
    is_edited BOOLEAN NOT NULL DEFAULT 0,
    is_deleted BOOLEAN NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(channel_id) REFERENCES channels(id)
);

CREATE INDEX IF NOT EXISTS idx_messages_channel_created ON messages(channel_id, created_at);
CREATE INDEX IF NOT EXISTS idx_messages_thread ON messages(thread_root_id);

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    assignee TEXT NOT NULL,
    priority TEXT NOT NULL DEFAULT 'medium',
    status TEXT NOT NULL DEFAULT 'queued',
    steps TEXT NOT NULL,
    current_step_index INTEGER NOT NULL DEFAULT 0,
    resume_checkpoint TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(assignee) REFERENCES agents(codename)
);

CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_assignee ON tasks(assignee);

CREATE TABLE IF NOT EXISTS memories (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    content TEXT NOT NULL,
    provenance TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 1.0,
    is_speculation BOOLEAN NOT NULL DEFAULT 0,
    tags TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memories_cat ON memories(category);

CREATE TABLE IF NOT EXISTS decisions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    level INTEGER NOT NULL,
    proposer TEXT NOT NULL,
    status TEXT NOT NULL,
    votes_json TEXT NOT NULL,
    dissent_summary TEXT,
    created_at TEXT NOT NULL,
    resolved_at TEXT
);

CREATE TABLE IF NOT EXISTS ui_deployments (
    version TEXT PRIMARY KEY,
    directory_path TEXT NOT NULL,
    sha256_hash TEXT NOT NULL,
    author_agent TEXT NOT NULL,
    verification_log TEXT,
    is_active BOOLEAN NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    actor TEXT NOT NULL,
    details TEXT NOT NULL,
    status TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_time ON audit_log(timestamp);

CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
"#;
