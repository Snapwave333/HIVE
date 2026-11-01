pub mod models;
pub mod schema;

use anyhow::{Context, Result};
use base64::Engine;
use chrono::{Duration as ChronoDuration, Utc};
use models::*;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const AGENT_JSONS: &[(&str, &str)] = &[
    ("PRIME", include_str!("../../../Assets/StreamingAssets/agents/PRIME.json")),
    ("SAGE", include_str!("../../../Assets/StreamingAssets/agents/SAGE.json")),
    ("FORGE", include_str!("../../../Assets/StreamingAssets/agents/FORGE.json")),
    ("SENTINEL", include_str!("../../../Assets/StreamingAssets/agents/SENTINEL.json")),
    ("VOICE", include_str!("../../../Assets/StreamingAssets/agents/VOICE.json")),
    ("ARCHIVIST", include_str!("../../../Assets/StreamingAssets/agents/ARCHIVIST.json")),
    ("CONDUCTOR", include_str!("../../../Assets/StreamingAssets/agents/CONDUCTOR.json")),
    ("VISION", include_str!("../../../Assets/StreamingAssets/agents/VISION.json")),
    ("LEDGER", include_str!("../../../Assets/StreamingAssets/agents/LEDGER.json")),
    ("LINK", include_str!("../../../Assets/StreamingAssets/agents/LINK.json")),
    ("GAUGE", include_str!("../../../Assets/StreamingAssets/agents/GAUGE.json")),
    ("SPARK", include_str!("../../../Assets/StreamingAssets/agents/SPARK.json")),
];

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    pub db_path: PathBuf,
}

impl Database {
    pub fn app_root_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Lushapps")
            .join("HIVE")
    }

    pub fn new(custom_path: Option<PathBuf>) -> Result<Self> {
        let path = match custom_path {
            Some(p) => p,
            None => {
                let app_dir = Self::app_root_dir().join("data");
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
             PRAGMA synchronous = NORMAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA temp_store = MEMORY;",
        )?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: path,
        };

        db.init_schema()?;
        db.apply_migrations()?;
        db.seed_defaults()?;

        if let Err(e) = db.create_backup() {
            log::warn!("Daily backup failed: {}", e);
        }

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
        db.apply_migrations()?;
        db.seed_defaults()?;

        Ok(db)
    }

    pub fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(schema::SCHEMA_V1)
            .context("Failed to execute initial schema migrations")?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at) VALUES (1, 'V1__initial_schema', ?1)",
            params![now],
        )?;
        Ok(())
    }

    pub fn create_backup(&self) -> Result<PathBuf> {
        if self.db_path.as_os_str() == ":memory:" {
            anyhow::bail!("Cannot backup in-memory database to disk");
        }
        let backup_dir = Self::app_root_dir().join("backups");
        std::fs::create_dir_all(&backup_dir)?;
        let date_tag = Utc::now().format("%Y-%m-%d").to_string();
        let backup_file = backup_dir.join(format!("hive_{}.db", date_tag));

        if !backup_file.exists() {
            let conn = self.conn.lock().unwrap();
            let escaped = backup_file.to_string_lossy().replace('\'', "''");
            conn.execute_batch(&format!("VACUUM INTO '{}';", escaped))
                .with_context(|| format!("VACUUM INTO {:?} failed", backup_file))?;
        }
        if std::fs::metadata(&backup_file).map(|m| m.len()).unwrap_or(0) == 0 {
            anyhow::bail!("Backup file {:?} is missing or empty", backup_file);
        }
        Ok(backup_file)
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

        // 2. Seed 12 Permanent Agents from StreamingAssets JSON definitions
        for (idx, (codename, raw_json)) in AGENT_JSONS.iter().enumerate() {
            let parsed: serde_json::Value = serde_json::from_str(raw_json).unwrap_or_default();
            let display = parsed["display_name"].as_str().unwrap_or(codename);
            let role = parsed["role"].as_str().unwrap_or("Specialist");
            let model = parsed["model_tier"].as_str().unwrap_or("nvidia/nemotron-3.5-lightning-30b-a3b");
            let avatar_cfg = parsed["avatar_config"].to_string();
            let personality = parsed.get("personality").map(|v| v.to_string());
            let skills = parsed.get("skills").map(|v| v.to_string());
            let tools = parsed.get("tools").map(|v| v.to_string());
            let permissions = parsed.get("permissions").map(|v| v.to_string());

            conn.execute(
                "INSERT INTO agents (codename, display_name, role, model_tier, status, avatar_config, personality, skills, tools, permissions, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'available', ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(codename) DO UPDATE SET
                    display_name = excluded.display_name,
                    role = excluded.role,
                    model_tier = excluded.model_tier,
                    avatar_config = excluded.avatar_config,
                    personality = COALESCE(excluded.personality, agents.personality),
                    skills = COALESCE(excluded.skills, agents.skills),
                    tools = COALESCE(excluded.tools, agents.tools),
                    permissions = COALESCE(excluded.permissions, agents.permissions),
                    status = 'available'",
                params![
                    codename,
                    display,
                    role,
                    model,
                    avatar_cfg,
                    personality,
                    skills,
                    tools,
                    permissions,
                    now
                ],
            )?;

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
                        format!("Direct channel with {} ({}) — Model: {}", display, role, model),
                        now
                    ],
                )?;
            }
        }

        let mem_count: i64 = conn.query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))?;
        if mem_count == 0 {
            if let Ok(constitution) = serde_json::from_str::<serde_json::Value>(include_str!("../../../app/constitution.json")) {
                let version = constitution["version"].as_str().unwrap_or("1.0.0");
                if let Some(invariants) = constitution["invariants"].as_object() {
                    for (key, val) in invariants {
                        let rule = val["rule"].as_str().unwrap_or_default();
                        conn.execute(
                            "INSERT INTO memories (id, category, content, provenance, confidence, is_speculation, tags, created_at)
                             VALUES (?1, 'shared', ?2, ?3, 1.0, 0, ?4, ?5)",
                            params![
                                Uuid::new_v4().to_string(),
                                format!("Constitution v{} ({}): {}", version, key.replace('_', " "), rule),
                                format!("constitution.json#{}", key),
                                format!("[\"constitution\",\"{}\"]", key),
                                now
                            ],
                        )?;
                    }
                }
            }
        }


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
                    "Hey, welcome to #agency-hq. This is the group chat for all 12 of us. Tag anyone with @NAME, DM us from the sidebar, or type `/help` to see the slash commands. Tool status (Google, Obsidian, the model provider) lives in Settings, and we'll tell you straight if something isn't connected.",
                    now
                ],
            )?;
        }

        let settings = vec![
            ("audio_muted", "true"),
            ("agency_state", "running"),
            ("theme", "dark"),
            ("font", "segoe"),
            ("font_size", "14"),
            ("line_height", "standard"),
            ("focus_mode", "false"),
            ("sensory_mode", "silent"),
            ("purchase_cooling_hours", "48"),
            ("auto_start_windows", "false"),
        ];

        for (k, v) in settings {
            conn.execute(
                "INSERT OR IGNORE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
                params![k, v, now],
            )?;
        }

        Ok(())
    }

    fn apply_migrations(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let applied: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = 2",
            [],
            |r| r.get(0),
        )?;
        if applied > 0 {
            return Ok(());
        }
        let now = Utc::now().to_rfc3339();
        conn.execute_batch(
            "DELETE FROM memories WHERE provenance IN ('LINK_audit', 'agency_retrospective', 'VISION_journal');
             DELETE FROM relationships WHERE notes IN (
                'Strategic sprint decomposition into 5-minute action chunks',
                'Continuous build & regression verification pipeline',
                'Research synthesis indexed into Obsidian knowledge graph',
                'Minimalist Borg design token implementation',
                'CLI bridge allowlist & credential isolation audit',
                'Debated experimental feature velocity vs stability window; resolved via preview sandbox',
                'Guided R&D prototype alignment with Application Constitution'
             );
             UPDATE tasks SET status = 'failed', resume_checkpoint = 'Retired: this task was seeded with a pre-checked step that never actually ran'
              WHERE title = 'Agency Boot & Ecosystem Synchronization' AND status = 'active';
             UPDATE ui_deployments SET is_active = 0, verification_log = 'Retired: legacy manifest-only record with no real bundle snapshot'
              WHERE sha256_hash = 'baseline' OR directory_path NOT LIKE '%bundle';",
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at) VALUES (2, 'V2__retire_fabricated_seed_data', ?1)",
            params![now],
        )?;
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

    fn map_agent(row: &rusqlite::Row) -> rusqlite::Result<Agent> {
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
    }

    pub fn get_agent(&self, codename: &str) -> Result<Option<Agent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT codename, display_name, role, model_tier, status, avatar_config, personality, skills, tools, permissions, updated_at
             FROM agents WHERE codename = ?1",
        )?;
        let mut rows = stmt.query(params![codename])?;
        match rows.next()? {
            Some(row) => Ok(Some(Self::map_agent(row)?)),
            None => Ok(None),
        }
    }

    pub fn get_channel_agent(&self, channel_id: i64) -> Result<Option<Agent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT a.codename, a.display_name, a.role, a.model_tier, a.status, a.avatar_config, a.personality, a.skills, a.tools, a.permissions, a.updated_at
             FROM channels c JOIN agents a ON a.codename = c.agent_codename
             WHERE c.id = ?1",
        )?;
        let mut rows = stmt.query(params![channel_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(Self::map_agent(row)?)),
            None => Ok(None),
        }
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

    // Relationships
    pub fn get_relationships(&self) -> Result<Vec<RelationshipEdge>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT source_agent, target_agent, edge_type, weight, notes, updated_at
             FROM relationships ORDER BY weight DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(RelationshipEdge {
                source_agent: row.get(0)?,
                target_agent: row.get(1)?,
                edge_type: row.get(2)?,
                weight: row.get(3)?,
                notes: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        let mut edges = Vec::new();
        for r in rows {
            edges.push(r?);
        }
        Ok(edges)
    }

    pub fn record_relationship(
        &self,
        source: &str,
        target: &str,
        edge_type: &str,
        delta: f64,
        notes: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let current: f64 = conn
            .query_row(
                "SELECT weight FROM relationships WHERE source_agent = ?1 AND target_agent = ?2 AND edge_type = ?3",
                params![source, target, edge_type],
                |r| r.get(0),
            )
            .unwrap_or(0.5);
        let new_weight = (current + delta).clamp(0.1, 1.0);
        conn.execute(
            "INSERT INTO relationships (source_agent, target_agent, edge_type, weight, notes, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(source_agent, target_agent, edge_type) DO UPDATE SET
                weight = excluded.weight,
                notes = COALESCE(excluded.notes, relationships.notes),
                updated_at = excluded.updated_at",
            params![source, target, edge_type, new_weight, notes, now],
        )?;
        Ok(())
    }

    pub fn decay_relationships(&self, half_life_days: f64) -> Result<usize> {
        self.decay_relationships_at(half_life_days, Utc::now())
    }

    pub fn decay_relationships_at(&self, half_life_days: f64, now: chrono::DateTime<Utc>) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let now_str = now.to_rfc3339();
        let last: Option<String> = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'relationships_last_decay'",
                [],
                |r| r.get(0),
            )
            .ok();
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES ('relationships_last_decay', ?1, ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![now_str],
        )?;
        let Some(last_ts) = last.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok()) else {
            return Ok(0);
        };
        let elapsed_days = (now - last_ts.with_timezone(&Utc)).num_seconds() as f64 / 86_400.0;
        if elapsed_days <= 0.0 || half_life_days <= 0.0 {
            return Ok(0);
        }
        let factor = 0.5f64.powf(elapsed_days / half_life_days);
        let n = conn.execute(
            "UPDATE relationships SET weight = MAX(0.1, weight * ?1) WHERE updated_at < ?2",
            params![factor, last_ts.with_timezone(&Utc).to_rfc3339()],
        )?;
        Ok(n)
    }

    // Temporary Specialists (strictly prefixed with temp_ and auto-retired after TTL)
    pub fn get_temp_specialists(&self) -> Result<Vec<TempSpecialist>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT codename, display_name, role, sponsor_agent, permissions, expires_at, status, created_at
             FROM temp_specialists ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TempSpecialist {
                codename: row.get(0)?,
                display_name: row.get(1)?,
                role: row.get(2)?,
                sponsor_agent: row.get(3)?,
                permissions: row.get(4)?,
                expires_at: row.get(5)?,
                status: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn spawn_temp_specialist(
        &self,
        name: &str,
        role: &str,
        sponsor: &str,
        ttl_minutes: i64,
    ) -> Result<TempSpecialist> {
        let clean = name.trim().to_lowercase().replace(' ', "_");
        let codename = if clean.starts_with("temp_") {
            clean
        } else {
            format!("temp_{}", clean)
        };
        let now = Utc::now();
        let expires = (now + ChronoDuration::minutes(ttl_minutes.max(5))).to_rfc3339();
        let now_str = now.to_rfc3339();
        let perms = serde_json::json!({
            "read": ["assigned_task_only"],
            "write": ["temp_scratch"],
            "sponsor": sponsor,
            "permanent_promotion": false
        })
        .to_string();

        let spec = TempSpecialist {
            codename: codename.clone(),
            display_name: format!("{} [TEMP]", name.trim()),
            role: role.to_string(),
            sponsor_agent: sponsor.to_uppercase(),
            permissions: perms.clone(),
            expires_at: expires.clone(),
            status: "active".to_string(),
            created_at: now_str.clone(),
        };

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO temp_specialists (codename, display_name, role, sponsor_agent, permissions, expires_at, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7)",
            params![
                spec.codename,
                spec.display_name,
                spec.role,
                spec.sponsor_agent,
                spec.permissions,
                spec.expires_at,
                spec.created_at
            ],
        )?;
        Ok(spec)
    }

    pub fn retire_expired_specialists(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let count = conn.execute(
            "UPDATE temp_specialists SET status = 'retired' WHERE status = 'active' AND expires_at <= ?1",
            params![now],
        )?;
        Ok(count)
    }

    // External Managed Attachments
    pub fn save_attachment(
        &self,
        filename: &str,
        mime_type: &str,
        data_base64: &str,
    ) -> Result<AttachmentMeta> {
        let clean_b64 = match data_base64.find(",") {
            Some(idx) => &data_base64[idx + 1..],
            None => data_base64,
        };
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(clean_b64.trim())
            .context("Attachment data is not valid base64")?;

        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash = format!("{:x}", hasher.finalize());

        let ext = std::path::Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");

        let attach_dir = Self::app_root_dir().join("attachments");
        std::fs::create_dir_all(&attach_dir)?;
        let file_path = attach_dir.join(format!("{}.{}", hash, ext));
        if !file_path.exists() {
            std::fs::write(&file_path, &bytes)?;
        }

        let now = Utc::now().to_rfc3339();
        let meta = AttachmentMeta {
            hash: hash.clone(),
            filename: filename.to_string(),
            mime_type: mime_type.to_string(),
            size_bytes: bytes.len() as u64,
            file_path: file_path.to_string_lossy().to_string(),
            data_url: if mime_type.starts_with("image/") && bytes.len() < 2_000_000 {
                Some(format!("data:{};base64,{}", mime_type, clean_b64.trim()))
            } else {
                None
            },
            created_at: now.clone(),
        };

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO attachments (hash, filename, mime_type, size_bytes, file_path, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                meta.hash,
                meta.filename,
                meta.mime_type,
                meta.size_bytes as i64,
                meta.file_path,
                meta.created_at
            ],
        )?;

        Ok(meta)
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

    pub fn get_message_by_id(&self, id: &str) -> Result<Option<Message>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, channel_id, sender, sender_role, content, attachments, reactions, reply_to_id, thread_root_id, is_pinned, is_edited, is_deleted, created_at, updated_at
             FROM messages WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_message(row)?))
        } else {
            Ok(None)
        }
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

    pub fn toggle_reaction(&self, id: &str, emoji: &str, user_id: &str) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        let existing: Option<String> = conn
            .query_row("SELECT reactions FROM messages WHERE id = ?1", params![id], |r| r.get(0))
            .ok()
            .flatten();

        let mut map: HashMap<String, Vec<String>> = existing
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let entry = map.entry(emoji.to_string()).or_default();
        if let Some(pos) = entry.iter().position(|u| u == user_id) {
            entry.remove(pos);
            if entry.is_empty() {
                map.remove(emoji);
            }
        } else {
            entry.push(user_id.to_string());
        }

        let reactions_json = serde_json::to_string(&map)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE messages SET reactions = ?1, updated_at = ?2 WHERE id = ?3",
            params![reactions_json, now, id],
        )?;
        Ok(reactions_json)
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

    pub fn forward_message(&self, message_id: &str, target_channel_id: i64) -> Result<Message> {
        let orig = self
            .get_message_by_id(message_id)?
            .context("Original message not found")?;
        let now = Utc::now().to_rfc3339();
        let new_msg = Message {
            id: Uuid::new_v4().to_string(),
            channel_id: target_channel_id,
            sender: "user".to_string(),
            sender_role: Some("Owner (Forwarded)".to_string()),
            content: format!("↪ *Forwarded from {}*:\n{}", orig.sender, orig.content),
            attachments: orig.attachments,
            reactions: None,
            reply_to_id: None,
            thread_root_id: None,
            is_pinned: false,
            is_edited: false,
            is_deleted: false,
            created_at: now.clone(),
            updated_at: now,
        };
        self.insert_message(&new_msg)?;
        Ok(new_msg)
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

    pub fn set_task_status(&self, task_id: &str, status: &str, checkpoint: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET status = ?1, resume_checkpoint = COALESCE(?2, resume_checkpoint), updated_at = ?3 WHERE id = ?4",
            params![status, checkpoint, now, task_id],
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

    pub fn get_agent_memories(&self, codename: &str, limit: i64) -> Result<Vec<MemoryItem>> {
        let conn = self.conn.lock().unwrap();
        let own_tagged = format!("agent:{}:%", codename);
        let own_prefixed = format!("{}%", codename);
        let mut stmt = conn.prepare(
            "SELECT id, category, content, provenance, confidence, is_speculation, tags, created_at
             FROM memories
             WHERE provenance LIKE ?1 OR provenance LIKE ?2 OR category = 'shared'
             ORDER BY (provenance LIKE ?1 OR provenance LIKE ?2) DESC, created_at DESC
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![own_tagged, own_prefixed, limit], Self::map_memory)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
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

    pub fn update_memory(&self, id: &str, content: &str, is_speculation: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let conf = if is_speculation { 0.5 } else { 1.0 };
        conn.execute(
            "UPDATE memories SET content = ?1, is_speculation = ?2, confidence = ?3 WHERE id = ?4",
            params![content, is_speculation, conf, id],
        )?;
        Ok(())
    }

    pub fn delete_memory(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        Ok(())
    }

    // Unified Search across messages, tasks, memories
    pub fn search(&self, query: &str) -> Result<serde_json::Value> {
        let conn = self.conn.lock().unwrap();
        let escaped = query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("%{}%", escaped);

        // Messages
        let mut msg_stmt = conn.prepare(
            "SELECT id, channel_id, sender, content, created_at FROM messages
             WHERE content LIKE ?1 ESCAPE '\\' AND is_deleted = 0
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
             WHERE title LIKE ?1 ESCAPE '\\' OR description LIKE ?1 ESCAPE '\\'
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
             WHERE content LIKE ?1 ESCAPE '\\' OR tags LIKE ?1 ESCAPE '\\'
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

    pub fn get_audit_log(&self, limit: i64) -> Result<Vec<AuditEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, event_type, actor, details, status, timestamp
             FROM audit_log ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(AuditEntry {
                id: row.get(0)?,
                event_type: row.get(1)?,
                actor: row.get(2)?,
                details: row.get(3)?,
                status: row.get(4)?,
                timestamp: row.get(5)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn save_decision(&self, d: &DecisionRow) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO decisions (id, title, description, level, proposer, status, votes_json, dissent_summary, created_at, resolved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                votes_json = excluded.votes_json,
                dissent_summary = excluded.dissent_summary,
                resolved_at = excluded.resolved_at",
            params![
                d.id,
                d.title,
                d.description,
                d.level,
                d.proposer,
                d.status,
                d.votes_json,
                d.dissent_summary,
                d.created_at,
                d.resolved_at
            ],
        )?;
        Ok(())
    }

    pub fn resolve_decision(&self, id: &str, status: &str) -> Result<Option<DecisionRow>> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE decisions SET status = ?1, resolved_at = ?2 WHERE id = ?3",
            params![status, now, id],
        )?;
        let mut stmt = conn.prepare(
            "SELECT id, title, description, level, proposer, status, votes_json, dissent_summary, created_at, resolved_at
             FROM decisions WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(DecisionRow {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                level: row.get(3)?,
                proposer: row.get(4)?,
                status: row.get(5)?,
                votes_json: row.get(6)?,
                dissent_summary: row.get(7)?,
                created_at: row.get(8)?,
                resolved_at: row.get(9)?,
            })),
            None => Ok(None),
        }
    }

    // UI Deployments
    pub fn record_ui_deployment(&self, dep: &UiDeployment) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        if dep.is_active {
            conn.execute("UPDATE ui_deployments SET is_active = 0", [])?;
        }
        conn.execute(
            "INSERT OR REPLACE INTO ui_deployments (version, directory_path, sha256_hash, author_agent, verification_log, is_active, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                dep.version,
                dep.directory_path,
                dep.sha256_hash,
                dep.author_agent,
                dep.verification_log,
                dep.is_active,
                dep.created_at
            ],
        )?;
        Ok(())
    }

    pub fn get_ui_deployments(&self) -> Result<Vec<UiDeployment>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT version, directory_path, sha256_hash, author_agent, verification_log, is_active, created_at
             FROM ui_deployments ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(UiDeployment {
                version: row.get(0)?,
                directory_path: row.get(1)?,
                sha256_hash: row.get(2)?,
                author_agent: row.get(3)?,
                verification_log: row.get(4)?,
                is_active: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
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
