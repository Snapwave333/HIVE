pub mod models;
pub mod schema;

use anyhow::{Context, Result};
use chrono::Utc;
use models::*;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    pub db_path: PathBuf,
}

impl Database {
    pub fn new(custom_path: Option<PathBuf>) -> Result<Self> {
        let path = match custom_path {
            Some(p) => p,
            None => {
                let app_dir = dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("Lushapps")
                    .join("HIVE")
                    .join("data");
                std::fs::create_dir_all(&app_dir)
                    .with_context(|| format!("Failed to create data dir: {:?}", app_dir))?;
                app_dir.join("hive.db")
            }
        };

        let conn = Connection::open(&path)
            .with_context(|| format!("Failed to open SQLite database at {:?}", path))?;

        // Enable WAL mode & foreign keys
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;",
        )?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: path,
        };

        db.init_schema()?;
        db.seed_defaults()?;

        Ok(db)
    }

    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;",
        )?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: PathBuf::from(":memory:"),
        };

        db.init_schema()?;
        db.seed_defaults()?;

        Ok(db)
    }

    pub fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(schema::SCHEMA_V1)
            .context("Failed to execute initial schema migrations")?;
        Ok(())
    }

    pub fn seed_defaults(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        // 1. Seed #agency-hq (ID 0)
        let channel_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM channels WHERE id = 0",
            [],
            |row| row.get(0),
        )?;

        if channel_count == 0 {
            conn.execute(
                "INSERT INTO channels (id, name, slug, is_group, agent_codename, description, pinned, created_at)
                 VALUES (0, 'agency-hq', 'agency-hq', 1, NULL, 'Primary Agency Group Chat — Collective Town Square', 1, ?1)",
                params![now],
            )?;
        }

        // 2. Seed 12 Permanent Agents
        let agents_data = vec![
            ("PRIME", "Prime", "Command / Strategy", "nvidia/nemotron-3-ultra-550b-a55b", "#85D94B", "#42CFC4", "Direction, high-stakes decisions, owner liaison"),
            ("SAGE", "Sage", "Research & Analysis", "nvidia/nemotron-3.5-super-vl-120b-a12b", "#42CFC4", "#85D94B", "Deep research, synthesis, competitive intelligence"),
            ("FORGE", "Forge", "Engineering / Build", "nvidia/nemotron-3.5-lightning-30b-a3b", "#85D94B", "#42CFC4", "Code generation, architecture, CI/CD, tooling"),
            ("SENTINEL", "Sentinel", "Security & Audit", "mistralai/mistral-large-4-1t-a52b", "#E45B5B", "#B8E8D8", "Threat modeling, permission audits, incident response"),
            ("VOICE", "Voice", "Communications", "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning", "#42CFC4", "#B8E8D8", "External comms, stakeholder updates, documentation"),
            ("ARCHIVIST", "Archivist", "Knowledge & Memory", "openai/gpt-oss-20b", "#B8E8D8", "#42CFC4", "Memory indexing, retrieval, knowledge graph maintenance"),
            ("CONDUCTOR", "Conductor", "Operations / Queue", "minimaxai/minimax-m3.1-flash", "#85D94B", "#B8E8D8", "Task routing, dependency resolution, throughput optimization"),
            ("VISION", "Vision", "Creative / Design", "nvidia/nemotron-3.5-super-vl-120b-a12b", "#42CFC4", "#85D94B", "Visual assets, UI/UX, branding, presentation design"),
            ("LEDGER", "Ledger", "Finance / Resources", "nvidia/nemotron-3-ultra-550b-a55b", "#E5A94B", "#85D94B", "Budget tracking, API cost optimization, resource allocation"),
            ("LINK", "Link", "Integration / Tools", "mistralai/mistral-large-4-1t-a52b", "#42CFC4", "#85D94B", "API integrations, plugin development, sandbox management"),
            ("GAUGE", "Gauge", "Quality / Verification", "nvidia/nemotron-3.5-lightning-30b-a3b", "#385C32", "#85D94B", "Testing, verification, regression detection, standards"),
            ("SPARK", "Spark", "Innovation / Experiments", "nvidia/nemotron-3.5-bolt-e4b", "#85D94B", "#42CFC4", "R&D, prototype evaluation, emerging tech assessment"),
        ];

        for (idx, (codename, display, role, model, primary_col, secondary_col, desc)) in agents_data.iter().enumerate() {
            let agent_exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM agents WHERE codename = ?1",
                params![codename],
                |row| row.get(0),
            )?;

            let avatar_cfg = serde_json::json!({
                "silhouette": "geometric_hexagon",
                "primary_color": primary_col,
                "secondary_color": secondary_col,
                "animation_set": "active"
            }).to_string();

            if agent_exists == 0 {
                conn.execute(
                    "INSERT INTO agents (codename, display_name, role, model_tier, status, avatar_config, updated_at)
                     VALUES (?1, ?2, ?3, ?4, 'available', ?5, ?6)",
                    params![codename, display, role, model, avatar_cfg, now],
                )?;
            }

            // Seed agent DM channel (IDs 1 through 12)
            let channel_id = (idx + 1) as i64;
            let dm_exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM channels WHERE id = ?1",
                params![channel_id],
                |row| row.get(0),
            )?;

            if dm_exists == 0 {
                conn.execute(
                    "INSERT INTO channels (id, name, slug, is_group, agent_codename, description, pinned, created_at)
                     VALUES (?1, ?2, ?3, 0, ?4, ?5, 0, ?6)",
                    params![
                        channel_id,
                        display,
                        codename.to_lowercase(),
                        codename,
                        format!("Direct channel with {} ({}) — {}", display, role, desc),
                        now
                    ],
                )?;
            }
        }

        // 3. Seed welcome message in #agency-hq if empty
        let msg_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE channel_id = 0",
            [],
            |row| row.get(0),
        )?;

        if msg_count == 0 {
            let welcome_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO messages (id, channel_id, sender, sender_role, content, is_pinned, created_at, updated_at)
                 VALUES (?1, 0, 'PRIME', 'Command / Strategy', ?2, 1, ?3, ?3)",
                params![
                    welcome_id,
                    "⬡ **H.I.V.E. Agency Online**\n\nAll 12 permanent agents are initialized and synchronized. The agency operates 24/7 across `#agency-hq` and 12 dedicated agent direct lines.\n\n* **PRIME**: Standing by for command directives and strategic planning.\n* **FORGE & GAUGE**: Engineering and verification pipelines active.\n* **SENTINEL**: Threat modeling and permission sandbox locked.\n* **ARCHIVIST**: Knowledge archive and memory graph ready.\n\nType `/help` or `@agent` to coordinate.",
                    now
                ],
            )?;
        }

        // 4. Seed default settings
        let settings = vec![
            ("audio_muted", "true"),
            ("agency_state", "running"),
            ("theme", "dark"),
            ("font", "segoe"),
            ("font_size", "14"),
            ("focus_mode", "false"),
            ("sensory_mode", "silent"),
        ];

        for (k, v) in settings {
            conn.execute(
                "INSERT OR IGNORE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
                params![k, v, now],
            )?;
        }

        Ok(())
    }

    // Channels
    pub fn get_channels(&self) -> Result<Vec<Channel>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, slug, is_group, agent_codename, description, pinned, created_at
             FROM channels ORDER BY pinned DESC, id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Channel {
                id: row.get(0)?,
                name: row.get(1)?,
                slug: row.get(2)?,
                is_group: row.get(3)?,
                agent_codename: row.get(4)?,
                description: row.get(5)?,
                pinned: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;

        let mut channels = Vec::new();
        for r in rows {
            channels.push(r?);
        }
        Ok(channels)
    }

    // Agents
    pub fn get_agents(&self) -> Result<Vec<Agent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT codename, display_name, role, model_tier, status, avatar_config, personality, skills, tools, permissions, updated_at
             FROM agents ORDER BY codename ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Agent {
                codename: row.get(0)?,
                display_name: row.get(1)?,
                role: row.get(2)?,
                model_tier: row.get(3)?,
                status: row.get(4)?,
                avatar_config: row.get(5)?,
                personality: row.get(6)?,
                skills: row.get(7)?,
                tools: row.get(8)?,
                permissions: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?;

        let mut agents = Vec::new();
        for r in rows {
            agents.push(r?);
        }
        Ok(agents)
    }

    pub fn update_agent_status(&self, codename: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE agents SET status = ?1, updated_at = ?2 WHERE codename = ?3",
            params![status, now, codename],
        )?;
        Ok(())
    }

    // Messages
    pub fn get_messages(&self, channel_id: i64, limit: i64, before_id: Option<String>) -> Result<Vec<Message>> {
        let conn = self.conn.lock().unwrap();
        let mut messages = Vec::new();

        if let Some(before) = before_id {
            let mut stmt = conn.prepare(
                "SELECT id, channel_id, sender, sender_role, content, attachments, reactions, reply_to_id, thread_root_id, is_pinned, is_edited, is_deleted, created_at, updated_at
                 FROM messages
                 WHERE channel_id = ?1 AND created_at < (SELECT created_at FROM messages WHERE id = ?2)
                 ORDER BY created_at DESC LIMIT ?3",
            )?;
            let rows = stmt.query_map(params![channel_id, before, limit], Self::map_message)?;
            for r in rows {
                messages.push(r?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, channel_id, sender, sender_role, content, attachments, reactions, reply_to_id, thread_root_id, is_pinned, is_edited, is_deleted, created_at, updated_at
                 FROM messages
                 WHERE channel_id = ?1
                 ORDER BY created_at DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![channel_id, limit], Self::map_message)?;
            for r in rows {
                messages.push(r?);
            }
        }

        // Reverse to display chronologically
        messages.reverse();
        Ok(messages)
    }

    fn map_message(row: &rusqlite::Row) -> rusqlite::Result<Message> {
        Ok(Message {
            id: row.get(0)?,
            channel_id: row.get(1)?,
            sender: row.get(2)?,
            sender_role: row.get(3)?,
            content: row.get(4)?,
            attachments: row.get(5)?,
            reactions: row.get(6)?,
            reply_to_id: row.get(7)?,
            thread_root_id: row.get(8)?,
            is_pinned: row.get(9)?,
            is_edited: row.get(10)?,
            is_deleted: row.get(11)?,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
        })
    }

    pub fn insert_message(&self, msg: &Message) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO messages (id, channel_id, sender, sender_role, content, attachments, reactions, reply_to_id, thread_root_id, is_pinned, is_edited, is_deleted, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                msg.id,
                msg.channel_id,
                msg.sender,
                msg.sender_role,
                msg.content,
                msg.attachments,
                msg.reactions,
                msg.reply_to_id,
                msg.thread_root_id,
                msg.is_pinned,
                msg.is_edited,
                msg.is_deleted,
                msg.created_at,
                msg.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn update_message(&self, id: &str, content: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE messages SET content = ?1, is_edited = 1, updated_at = ?2 WHERE id = ?3",
            params![content, now, id],
        )?;
        Ok(())
    }

    pub fn delete_message(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE messages SET is_deleted = 1, content = '[Message deleted]', updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn set_reactions(&self, id: &str, reactions_json: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE messages SET reactions = ?1, updated_at = ?2 WHERE id = ?3",
            params![reactions_json, now, id],
        )?;
        Ok(())
    }

    // Tasks (5-minute chunking)
    pub fn get_tasks(&self, status_filter: Option<&str>) -> Result<Vec<TaskChunk>> {
        let conn = self.conn.lock().unwrap();
        let mut tasks = Vec::new();

        let query = match status_filter {
            Some(_) => "SELECT id, title, description, assignee, priority, status, steps, current_step_index, resume_checkpoint, created_at, updated_at FROM tasks WHERE status = ?1 ORDER BY updated_at DESC",
            None => "SELECT id, title, description, assignee, priority, status, steps, current_step_index, resume_checkpoint, created_at, updated_at FROM tasks ORDER BY updated_at DESC",
        };

        let mut stmt = conn.prepare(query)?;
        let rows = if let Some(sf) = status_filter {
            stmt.query_map(params![sf], Self::map_task)?
        } else {
            stmt.query_map([], Self::map_task)?
        };

        for r in rows {
            tasks.push(r?);
        }
        Ok(tasks)
    }

    fn map_task(row: &rusqlite::Row) -> rusqlite::Result<TaskChunk> {
        let steps_raw: String = row.get(6)?;
        let steps: Vec<TaskStep> = serde_json::from_str(&steps_raw).unwrap_or_default();

        Ok(TaskChunk {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            assignee: row.get(3)?,
            priority: row.get(4)?,
            status: row.get(5)?,
            steps,
            current_step_index: row.get(7)?,
            resume_checkpoint: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    }

    pub fn insert_task(&self, task: &TaskChunk) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let steps_json = serde_json::to_string(&task.steps)?;
        conn.execute(
            "INSERT INTO tasks (id, title, description, assignee, priority, status, steps, current_step_index, resume_checkpoint, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                task.id,
                task.title,
                task.description,
                task.assignee,
                task.priority,
                task.status,
                steps_json,
                task.current_step_index,
                task.resume_checkpoint,
                task.created_at,
                task.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn update_task_step(&self, task_id: &str, step_index: usize, completed: bool, checkpoint: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        let steps_raw: String = conn.query_row(
            "SELECT steps FROM tasks WHERE id = ?1",
            params![task_id],
            |row| row.get(0),
        )?;

        let mut steps: Vec<TaskStep> = serde_json::from_str(&steps_raw)?;
        if let Some(step) = steps.get_mut(step_index) {
            step.completed = completed;
            step.completed_at = if completed { Some(now.clone()) } else { None };
        }

        let all_done = steps.iter().all(|s| s.completed);
        let next_index = if all_done {
            steps.len()
        } else {
            steps.iter().position(|s| !s.completed).unwrap_or(steps.len())
        };

        let new_status = if all_done { "completed" } else { "active" };
        let new_steps_json = serde_json::to_string(&steps)?;

        conn.execute(
            "UPDATE tasks SET steps = ?1, current_step_index = ?2, status = ?3, resume_checkpoint = ?4, updated_at = ?5 WHERE id = ?6",
            params![new_steps_json, next_index, new_status, checkpoint, now, task_id],
        )?;

        Ok(())
    }

    // Memories (4-tier)
    pub fn get_memories(&self, category: Option<&str>) -> Result<Vec<MemoryItem>> {
        let conn = self.conn.lock().unwrap();
        let mut memories = Vec::new();

        let query = match category {
            Some(_) => "SELECT id, category, content, provenance, confidence, is_speculation, tags, created_at FROM memories WHERE category = ?1 ORDER BY created_at DESC",
            None => "SELECT id, category, content, provenance, confidence, is_speculation, tags, created_at FROM memories ORDER BY created_at DESC",
        };

        let mut stmt = conn.prepare(query)?;
        let rows = if let Some(cat) = category {
            stmt.query_map(params![cat], Self::map_memory)?
        } else {
            stmt.query_map([], Self::map_memory)?
        };

        for r in rows {
            memories.push(r?);
        }
        Ok(memories)
    }

    fn map_memory(row: &rusqlite::Row) -> rusqlite::Result<MemoryItem> {
        Ok(MemoryItem {
            id: row.get(0)?,
            category: row.get(1)?,
            content: row.get(2)?,
            provenance: row.get(3)?,
            confidence: row.get(4)?,
            is_speculation: row.get(5)?,
            tags: row.get(6)?,
            created_at: row.get(7)?,
        })
    }

    pub fn insert_memory(&self, mem: &MemoryItem) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO memories (id, category, content, provenance, confidence, is_speculation, tags, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                mem.id,
                mem.category,
                mem.content,
                mem.provenance,
                mem.confidence,
                mem.is_speculation,
                mem.tags,
                mem.created_at,
            ],
        )?;
        Ok(())
    }

    // Unified Search across messages, tasks, memories
    pub fn search(&self, query: &str) -> Result<serde_json::Value> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", query);

        // Messages
        let mut msg_stmt = conn.prepare(
            "SELECT id, channel_id, sender, content, created_at FROM messages
             WHERE content LIKE ?1 AND is_deleted = 0
             ORDER BY created_at DESC LIMIT 20",
        )?;
        let msg_rows = msg_stmt.query_map(params![pattern], |row| {
            Ok(serde_json::json!({
                "type": "message",
                "id": row.get::<_, String>(0)?,
                "channel_id": row.get::<_, i64>(1)?,
                "sender": row.get::<_, String>(2)?,
                "snippet": row.get::<_, String>(3)?,
                "created_at": row.get::<_, String>(4)?,
            }))
        })?;
        let mut messages = Vec::new();
        for r in msg_rows {
            messages.push(r?);
        }

        // Tasks
        let mut task_stmt = conn.prepare(
            "SELECT id, title, assignee, status, created_at FROM tasks
             WHERE title LIKE ?1 OR description LIKE ?1
             ORDER BY created_at DESC LIMIT 10",
        )?;
        let task_rows = task_stmt.query_map(params![pattern], |row| {
            Ok(serde_json::json!({
                "type": "task",
                "id": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "assignee": row.get::<_, String>(2)?,
                "status": row.get::<_, String>(3)?,
                "created_at": row.get::<_, String>(4)?,
            }))
        })?;
        let mut tasks = Vec::new();
        for r in task_rows {
            tasks.push(r?);
        }

        // Memories
        let mut mem_stmt = conn.prepare(
            "SELECT id, category, content, provenance, created_at FROM memories
             WHERE content LIKE ?1
             ORDER BY created_at DESC LIMIT 10",
        )?;
        let mem_rows = mem_stmt.query_map(params![pattern], |row| {
            Ok(serde_json::json!({
                "type": "memory",
                "id": row.get::<_, String>(0)?,
                "category": row.get::<_, String>(1)?,
                "content": row.get::<_, String>(2)?,
                "provenance": row.get::<_, String>(3)?,
                "created_at": row.get::<_, String>(4)?,
            }))
        })?;
        let mut memories = Vec::new();
        for r in mem_rows {
            memories.push(r?);
        }

        Ok(serde_json::json!({
            "query": query,
            "messages": messages,
            "tasks": tasks,
            "memories": memories,
        }))
    }

    // Audit Log
    pub fn log_audit(&self, event_type: &str, actor: &str, details: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO audit_log (event_type, actor, details, status, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![event_type, actor, details, status, now],
        )?;
        Ok(())
    }

    // App Settings
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, now],
        )?;
        Ok(())
    }
}
