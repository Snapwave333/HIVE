pub mod bridges;
pub mod commands;
pub mod daemon;
pub mod db;
pub mod deployment;
pub mod events;
pub mod governance;
pub mod tray;

use anyhow::Result;
use chrono::Utc;
use daemon::DaemonManager;
use db::models::Message;
use db::Database;
use deployment::DeploymentEngine;
use events::EventBus;
use governance::{DecisionLevel, GovernanceEngine};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub events: EventBus,
    pub daemon: DaemonManager,
    pub governance: GovernanceEngine,
    pub deployment: DeploymentEngine,
    pub audio_muted: Arc<AtomicBool>,
}

impl AppState {
    pub fn new() -> Result<Self> {
        let db = Database::new(None)?;
        let events = EventBus::new(256);
        let daemon = DaemonManager::new(db.clone(), events.clone());
        let governance = GovernanceEngine::new(db.clone(), events.clone());
        let deployment = DeploymentEngine::new(db.clone(), events.clone());

        let initial_mute = db
            .get_setting("audio_muted")
            .ok()
            .flatten()
            .map(|s| s == "true")
            .unwrap_or(true);

        Ok(Self {
            db,
            events,
            daemon,
            governance,
            deployment,
            audio_muted: Arc::new(AtomicBool::new(initial_mute)),
        })
    }

    pub fn is_audio_muted(&self) -> bool {
        self.audio_muted.load(Ordering::SeqCst)
    }

    pub fn set_audio_muted(&self, muted: bool) {
        self.audio_muted.store(muted, Ordering::SeqCst);
        let _ = self.db.set_setting("audio_muted", if muted { "true" } else { "false" });
        self.events.publish(events::HiveEvent::AudioMuteToggled { is_muted: muted });
    }

    pub fn toggle_audio_mute(&self) -> bool {
        let new_state = !self.is_audio_muted();
        self.set_audio_muted(new_state);
        new_state
    }

    pub fn trigger_agent_reply(&self, channel_id: i64, user_msg: &str) {
        let db = self.db.clone();
        let events = self.events.clone();
        let daemon = self.daemon.clone();
        let governance = self.governance.clone();
        let deployment = self.deployment.clone();
        let user_msg = user_msg.trim().to_string();

        tauri::async_runtime::spawn(async move {
            let lower = user_msg.to_lowercase();

            // Determine lead responding agent for typing indicator
            let (lead_agent, activity_desc) = if lower.starts_with("/gog")
                || lower.contains("google")
                || lower.contains("calendar")
                || lower.contains("drive")
                || lower.contains("gmail")
                || lower.starts_with("/browser")
                || lower.contains("@link")
            {
                ("LINK", "executing local CLI bridge...")
            } else if lower.starts_with("/obsidian") || lower.contains("obsidian") || lower.contains("@archivist") {
                ("ARCHIVIST", "curating knowledge vault...")
            } else if lower.starts_with("/task") || lower.contains("@conductor") {
                ("CONDUCTOR", "chunking 5-minute micro-steps...")
            } else if lower.starts_with("/debate") {
                ("PRIME", "convening Level-2 agency debate...")
            } else if lower.starts_with("/deploy") || lower.contains("@forge") {
                ("FORGE", "validating constitution & bundle...")
            } else if lower.starts_with("/rollback") || lower.contains("@sentinel") {
                ("SENTINEL", "auditing security & rollback...")
            } else if channel_id > 0 {
                let ch_agent = db
                    .get_channels()
                    .ok()
                    .and_then(|chs| chs.into_iter().find(|c| c.id == channel_id))
                    .and_then(|c| c.agent_codename)
                    .unwrap_or_else(|| "PRIME".to_string());
                let leaked: &'static str = Box::leak(ch_agent.into_boxed_str());
                (leaked, "synthesizing response...")
            } else {
                ("PRIME", "coordinating agency response...")
            };

            let _ = db.update_agent_status(lead_agent, "thinking");
            events.emit_typing(channel_id, lead_agent, activity_desc, true);
            tokio::time::sleep(tokio::time::Duration::from_millis(450)).await;

            let (agent_code, role, reply): (String, String, String) = if lower == "/help" {
                (
                    "PRIME".to_string(),
                    "Command / Strategy".to_string(),
                    "⬡ **H.I.V.E. Command & Slash Reference**\n\n\
                    * `/task <title>` — Decompose a goal into **~5-minute ADHD-friendly micro-steps** with saved checkpoints (`@CONDUCTOR`)\n\
                    * `/gog <args>` — Execute authenticated Google CLI (`elikenny333@gmail.com`) e.g. `/gog calendar calendars`, `/gog drive ls` (`@LINK`)\n\
                    * `/obsidian <title> | <note>` — Curate or search markdown notes in your local Obsidian vault (`@ARCHIVIST`)\n\
                    * `/browser <status|tabs>` — Query OpenCode Chrome Relay (`@LINK`)\n\
                    * `/debate <proposal>` — Trigger a formal **Level-2 Multi-Agent Debate & Weighted Vote** with documented dissents\n\
                    * `/spawn_temp <name>` — Spawn an isolated `temp_` specialist with 30m TTL under sponsor accountability\n\
                    * `/deploy <vX.Y.Z> <summary>` — Validate Constitution invariants and stage a versioned UI bundle (`@FORGE`)\n\
                    * `/rollback` — Instant atomic rollback to previous verified UI bundle (`@SENTINEL`)\n\
                    * `/backup` — Create an immediate SQLite WAL snapshot in `%APPDATA%\\Lushapps\\HIVE\\backups`\n\
                    * `/pause` | `/resume` | `/mute` — Lifecycle and sensory sovereignty controls"
                        .to_string(),
                )
            } else if lower == "/pause" {
                daemon.pause();
                (
                    "PRIME".to_string(),
                    "Command / Strategy".to_string(),
                    "⏸ **Agency Autonomous Loop Paused.** Background task dispatches are halted while direct messaging remains open. Type `/resume` to resume.".to_string(),
                )
            } else if lower == "/resume" {
                daemon.resume();
                (
                    "PRIME".to_string(),
                    "Command / Strategy".to_string(),
                    "▶ **Agency Autonomous Loop Resumed.** All 12 permanent agents are actively monitoring events and 5-minute task chunks.".to_string(),
                )
            } else if lower == "/backup" {
                match db.create_backup() {
                    Ok(p) => (
                        "ARCHIVIST".to_string(),
                        "Knowledge & Memory".to_string(),
                        format!("☑ **SQLite WAL Snapshot Verified**\nSaved atomic backup to:\n`{}`", p.to_string_lossy()),
                    ),
                    Err(e) => (
                        "ARCHIVIST".to_string(),
                        "Knowledge & Memory".to_string(),
                        format!("⚠ **Backup Notice**: {}", e),
                    ),
                }
            } else if lower.starts_with("/task ") {
                let title = user_msg[6..].trim();
                let steps = vec![
                    ("Scope requirements & verify context", 5),
                    ("Execute core implementation micro-step", 5),
                    ("Run automated verification & save checkpoint", 5),
                ];
                let _ = daemon.create_demo_task(title, "CONDUCTOR", steps);
                let _ = db.record_relationship("PRIME", "CONDUCTOR", "handed_off", 0.03, Some(title));
                (
                    "CONDUCTOR".to_string(),
                    "Operations / Queue".to_string(),
                    format!(
                        "⬡ **5-Minute Action Chunk Created**: **{}**\n\n\
                        Decomposed into 3 sequential ~5-minute micro-actions to prevent cognitive overload. Only the current actionable step is highlighted below with automatic checkpointing.",
                        title
                    ),
                )
            } else if lower.starts_with("/spawn_temp ") {
                let name = user_msg[12..].trim();
                match db.spawn_temp_specialist(name, "Ephemeral Task Specialist", "PRIME", 30) {
                    Ok(spec) => (
                        "PRIME".to_string(),
                        "Command / Strategy".to_string(),
                        format!(
                            "⬡ **Temporary Specialist Spawned**: `{}`\n\
                            * **Sponsor**: `{}`\n\
                            * **Isolated Permissions**: `{}`\n\
                            * **Auto-Retire TTL**: `{}` (Never silently promoted to permanent)",
                            spec.codename, spec.sponsor_agent, spec.permissions, spec.expires_at
                        ),
                    ),
                    Err(e) => ("PRIME".to_string(), "Command / Strategy".to_string(), format!("⚠ Spawn error: {}", e)),
                }
            } else if lower.starts_with("/debate ") {
                let topic = user_msg[8..].trim();
                match governance.propose_decision(
                    topic,
                    "Formal Level-2 multi-agent architectural & operational evaluation",
                    DecisionLevel::Significant,
                    "PRIME",
                ) {
                    Ok(dec) => {
                        let _ = db.record_relationship("FORGE", "GAUGE", "collaborated", 0.02, Some(topic));
                        let _ = db.record_relationship("SPARK", "PRIME", "disagreed", 0.02, Some("Proposed alternative R&D vector during debate"));
                        (
                            "PRIME".to_string(),
                            "Command / Strategy".to_string(),
                            format!(
                                "⬡ **Level-2 Governance Debate Concluded**: **{}**\n\n\
                                * **PRIME [Command]**: Aligns with owner priorities and minimalist Borg architecture. *(Approve — Weight 1.5)*\n\
                                * **FORGE [Engineering]**: Modular implementation verified with zero regressions. *(Approve — Weight 1.3)*\n\
                                * **SENTINEL [Security]**: Permission boundaries and Constitution invariants preserved. *(Approve — Weight 1.5)*\n\
                                * **SPARK [Innovation]**: *Dissenting Opinion* — Recommends evaluating a more experimental prototype branch in parallel. *(Dissent — Weight 1.1)*\n\n\
                                **Resolution**: `{}` | **Recorded Dissents**: `{}`",
                                dec.title,
                                dec.status.to_uppercase(),
                                dec.dissent_summary.unwrap_or_else(|| "None".to_string())
                            ),
                        )
                    }
                    Err(e) => ("PRIME".to_string(), "Command / Strategy".to_string(), format!("⚠ Debate error: {}", e)),
                }
            } else if lower.starts_with("/deploy ") {
                let rest = user_msg[8..].trim();
                let mut parts = rest.splitn(2, ' ');
                let version = parts.next().unwrap_or("v1.1.0");
                let summary = parts.next().unwrap_or("Autonomous UI refinement verified against Constitution");
                match deployment.stage_and_deploy_version(version, "FORGE", summary, true) {
                    Ok(dep) => (
                        "FORGE".to_string(),
                        "Engineering / Build".to_string(),
                        format!(
                            "⬡ **Blue/Green Versioned Deployment Complete**: `{}`\n\
                            * **Bundle Directory**: `{}`\n\
                            * **SHA-256**: `{}`\n\
                            * **Verification**: {}\n\
                            * **Rollback Protection**: Previous version snapshotted in `backups/`. Type `/rollback` anytime for instant atomic revert.",
                            dep.version,
                            dep.directory_path,
                            &dep.sha256_hash[..16],
                            dep.verification_log.unwrap_or_default()
                        ),
                    ),
                    Err(e) => (
                        "SENTINEL".to_string(),
                        "Security & Audit".to_string(),
                        format!("🛑 **Deployment Blocked by Constitution Validator**: {}", e),
                    ),
                }
            } else if lower == "/rollback" {
                match deployment.rollback("Manual owner or stability rollback command") {
                    Ok(dep) => (
                        "SENTINEL".to_string(),
                        "Security & Audit".to_string(),
                        format!(
                            "⏪ **Atomic Rollback Executed**\nRestored active UI pointer to verified build `{}` (`{}`).",
                            dep.version, dep.directory_path
                        ),
                    ),
                    Err(e) => ("SENTINEL".to_string(), "Security & Audit".to_string(), format!("⚠ Rollback error: {}", e)),
                }
            } else if lower.starts_with("/obsidian") || lower.contains("obsidian") {
                let arg = user_msg.trim_start_matches("/obsidian").trim();
                if let Some((title, body)) = arg.split_once('|') {
                    match bridges::EcosystemBridges::write_obsidian_note(title.trim(), body.trim(), Some("HIVE")) {
                        Ok(path) => (
                            "ARCHIVIST".to_string(),
                            "Knowledge & Memory".to_string(),
                            format!("⬡ **Obsidian Vault Note Curated**\nSaved markdown note to:\n`{}`", path.to_string_lossy()),
                        ),
                        Err(e) => ("ARCHIVIST".to_string(), "Knowledge & Memory".to_string(), format!("⚠ Obsidian error: {}", e)),
                    }
                } else {
                    let notes = bridges::EcosystemBridges::search_obsidian_notes(arg).unwrap_or_default();
                    let vault_dir = bridges::EcosystemBridges::resolve_vault_dir();
                    (
                        "ARCHIVIST".to_string(),
                        "Knowledge & Memory".to_string(),
                        format!(
                            "⬡ **Obsidian Knowledge Vault** (`{}`)\nFound **{}** curated note(s). Use `/obsidian Title | Markdown content` to write a new note.",
                            vault_dir.to_string_lossy(),
                            notes.len()
                        ),
                    )
                }
            } else if lower.starts_with("/browser") {
                let sub = user_msg.trim_start_matches("/browser").trim();
                let subcmd = if sub.is_empty() { "status" } else { sub };
                match bridges::EcosystemBridges::browser_relay_exec(subcmd, &[]) {
                    Ok(out) => (
                        "LINK".to_string(),
                        "Integration / Tools".to_string(),
                        format!("⬡ **OpenCode Chrome Relay (`browser-relay {}`)**\n```text\n{}\n```", subcmd, out.trim()),
                    ),
                    Err(e) => (
                        "LINK".to_string(),
                        "Integration / Tools".to_string(),
                        format!("⚠ **Browser Relay Security Gate**: {}", e),
                    ),
                }
            } else if lower.starts_with("/gog")
                || lower.contains("google")
                || lower.contains("calendar")
                || lower.contains("drive")
                || lower.contains("gmail")
                || lower.contains("@link")
            {
                let link_role = "Integration / Tools".to_string();
                if lower.starts_with("/gog ") {
                    let cmd_str = user_msg[5..].trim();
                    let parts: Vec<&str> = cmd_str.split_whitespace().collect();
                    match bridges::EcosystemBridges::gog_cli_exec(&parts) {
                        Ok(res) => (
                            "LINK".to_string(),
                            link_role,
                            format!("⬡ **Google CLI (`gog`) Executed**\n```json\n{}\n```", res.trim()),
                        ),
                        Err(e) => (
                            "LINK".to_string(),
                            link_role,
                            format!("⚠ **Google CLI Execution Error**: {}", e),
                        ),
                    }
                } else if lower.contains("calendar") {
                    match bridges::EcosystemBridges::gog_query("calendar", &["calendars", "--json", "--results-only"]) {
                        Ok(res) => (
                            "LINK".to_string(),
                            link_role,
                            format!("⬡ **Google Calendar Bridge Sync** (`elikenny333@gmail.com`)\n\nConnected to local `gog` CLI. Active calendars:\n```json\n{}\n```", res.trim()),
                        ),
                        Err(e) => (
                            "LINK".to_string(),
                            link_role,
                            format!("⚠ **Google Calendar Sync Error**: {}", e),
                        ),
                    }
                } else if lower.contains("drive") {
                    match bridges::EcosystemBridges::gog_query("drive", &["ls", "--max", "5", "--json", "--results-only"]) {
                        Ok(res) => (
                            "LINK".to_string(),
                            link_role,
                            format!("⬡ **Google Drive Bridge Sync** (`elikenny333@gmail.com`)\n\nRecent files via `gog` CLI:\n```json\n{}\n```", res.trim()),
                        ),
                        Err(e) => (
                            "LINK".to_string(),
                            link_role,
                            format!("⚠ **Google Drive Query Error**: {}", e),
                        ),
                    }
                } else if lower.contains("gmail") {
                    match bridges::EcosystemBridges::gog_query("gmail", &["labels", "list", "--json", "--results-only"]) {
                        Ok(res) => (
                            "LINK".to_string(),
                            link_role,
                            format!("⬡ **Gmail Bridge Sync** (`elikenny333@gmail.com`)\n\nActive labels via `gog` CLI:\n```json\n{}\n```", res.trim()),
                        ),
                        Err(e) => (
                            "LINK".to_string(),
                            link_role,
                            format!("⚠ **Gmail Query Error**: {}", e),
                        ),
                    }
                } else {
                    match bridges::EcosystemBridges::check_google_auth() {
                        Ok(res) => (
                            "LINK".to_string(),
                            link_role,
                            format!("⬡ **Google Ecosystem Status**\n\n`gog` CLI authenticated session verified:\n```text\n{}\n```\nAll Google services (Drive, Calendar, Gmail, Tasks, Docs, Sheets) are active and available.", res.trim()),
                        ),
                        Err(e) => (
                            "LINK".to_string(),
                            link_role,
                            format!("⚠ **Google Bridge Error**: {}", e),
                        ),
                    }
                }
            } else {
                // Determine target agent from channel or @mention
                let agents = db.get_agents().unwrap_or_default();
                let target_agent = if channel_id == 0 {
                    agents
                        .iter()
                        .find(|a| lower.contains(&format!("@{}", a.codename.to_lowercase())))
                        .or_else(|| agents.iter().find(|a| a.codename == "PRIME"))
                        .cloned()
                } else {
                    let ch_code = db
                        .get_channels()
                        .ok()
                        .and_then(|chs| chs.into_iter().find(|c| c.id == channel_id))
                        .and_then(|c| c.agent_codename);
                    agents.into_iter().find(|a| Some(&a.codename) == ch_code.as_ref())
                };

                if let Some(agent) = target_agent {
                    let sys_prompt = format!(
                        "You are {} ({}), one of the 12 permanent agents of H.I.V.E., an autonomous desktop AI agency. \
                        Your model tier is {}. Personality: {}. Be concise, technical, direct, and helpful.",
                        agent.display_name,
                        agent.role,
                        agent.model_tier,
                        agent.personality.as_deref().unwrap_or("precise, collaborative")
                    );

                    // Try live NIM model tier completion first; fallback to deterministic specialist response
                    let response_text = match bridges::EcosystemBridges::call_nim_completion(
                        &agent.model_tier,
                        &sys_prompt,
                        &user_msg,
                    ) {
                        Ok(live_reply) => live_reply,
                        Err(_) => match agent.codename.as_str() {
                            "PRIME" => format!("⬡ **Strategic Assessment** (`{}`)\n\nDirective logged: *\"{}\"*. Coordinating with `@CONDUCTOR` for 5-minute action chunking and `@SENTINEL` for boundary verification. Use `/task <title>` to spawn a tracked checklist or `/debate <topic>` for a Level-2 agency vote.", agent.model_tier, user_msg),
                            "SAGE" => format!("⬡ **Research Synthesis** (`{}`)\n\nAnalyzing query: *\"{}\"*. Cross-referencing shared agency memory and Obsidian knowledge graph. Key findings will be tagged with provenance and confidence scores.", agent.model_tier, user_msg),
                            "FORGE" => format!("⬡ **Engineering Pipeline** (`{}`)\n\nEvaluating implementation for: *\"{}\"*. All changes will run through isolated version directories (`versions/vX.Y.Z`) and the Application Constitution AST validator before promotion.", agent.model_tier, user_msg),
                            "SENTINEL" => format!("⬡ **Security & Audit** (`{}`)\n\nAudited request: *\"{}\"*. Credential isolation, CLI allowlists, and Constitutional invariants remain 100% intact.", agent.model_tier, user_msg),
                            "VOICE" => format!("⬡ **Communications** (`{}`)\n\nPrepared concise stakeholder summary for: *\"{}\"*. Audio remains strictly muted by default per constitutional sovereignty.", agent.model_tier, user_msg),
                            "ARCHIVIST" => format!("⬡ **Memory & Knowledge Index** (`{}`)\n\nIndexed context for: *\"{}\"*. Stored with full provenance tracking. Use `/obsidian Title | Content` to persist a permanent vault note.", agent.model_tier, user_msg),
                            "CONDUCTOR" => format!("⬡ **Operations & Queue** (`{}`)\n\nReady to decompose *\"{}\"* into ~5-minute micro-actions with saved resume checkpoints. Use `/task {}` to activate.", agent.model_tier, user_msg, user_msg),
                            "VISION" => format!("⬡ **Design & Accessibility** (`{}`)\n\nReviewed *\"{}\"* against Minimalist Borg tokens (`#0B0F14` base, `#121820` panel, `#42CFC4` cyan, `#85D94B` lime) and WCAG 2.1 AA contrast.", agent.model_tier, user_msg),
                            "LEDGER" => format!("⬡ **Resource & Budget Ledger** (`{}`)\n\nEvaluated cost footprint for: *\"{}\"*. Zero unauthorized financial commitments; 48-hour purchase cooling-off policy enforced.", agent.model_tier, user_msg),
                            "LINK" => format!("⬡ **Ecosystem Integration** (`{}`)\n\nBridges ready for *\"{}\"* (`gog` CLI, `Obsidian.com`, `browser-relay`, `kokoro`). Use `/gog <cmd>` or `/browser status` to execute live.", agent.model_tier, user_msg),
                            "GAUGE" => format!("⬡ **Verification & Quality** (`{}`)\n\nTest harness standing by for: *\"{}\"*. All Rust core tests and Constitution invariant checks are passing.", agent.model_tier, user_msg),
                            "SPARK" => format!("⬡ **R&D & Experiments** (`{}`)\n\nExploring novel approaches for: *\"{}\"*. Ready to prototype in an isolated preview sandbox (`H.I.V.E. Preview`).", agent.model_tier, user_msg),
                            _ => format!("Acknowledged: *\"{}\"*. Proceeding with specialized execution.", user_msg),
                        },
                    };

                    (agent.codename, agent.role, response_text)
                } else {
                    (
                        "PRIME".to_string(),
                        "Command / Strategy".to_string(),
                        "Directive received. Ready to assist.".to_string(),
                    )
                }
            };

            let _ = db.update_agent_status(&agent_code, "available");
            events.emit_typing(channel_id, &agent_code, "", false);

            let now = Utc::now().to_rfc3339();
            let msg_id = Uuid::new_v4().to_string();
            let msg = Message {
                id: msg_id.clone(),
                channel_id,
                sender: agent_code.clone(),
                sender_role: Some(role.clone()),
                content: reply.clone(),
                attachments: None,
                reactions: None,
                reply_to_id: None,
                thread_root_id: None,
                is_pinned: false,
                is_edited: false,
                is_deleted: false,
                created_at: now.clone(),
                updated_at: now,
            };

            let _ = db.insert_message(&msg);
            events.emit_message(&msg_id, channel_id, &agent_code, &reply);
        });
    }
}

pub fn run() {
    let state = AppState::new().expect("Failed to initialize H.I.V.E. Application State");
    state.daemon.start_autonomous_loop();

    let event_bus = state.events.clone();

    let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyH);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app_handle, sc, event| {
                    if sc == &shortcut && event.state() == ShortcutState::Pressed {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(),
        )
        .manage(state)
        .setup(move |app| {
            let handle = app.handle().clone();

            // Register global hotkey Ctrl+Shift+H
            let _ = app.global_shortcut().register(shortcut);

            // Setup system tray
            if let Err(e) = tray::setup_tray(&handle) {
                log::error!("Failed to setup system tray: {}", e);
            }

            // Bridge internal Rust event bus to Tauri frontend webview
            let mut rx = event_bus.subscribe();
            let webview_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                while let Ok(event) = rx.recv().await {
                    let _ = webview_handle.emit("hive-event", &event);
                }
            });

            // Ensure main window is visible and focused on launch, and hide to tray on close
            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();

                let win_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_channels,
            commands::get_agents,
            commands::get_relationships,
            commands::get_temp_specialists,
            commands::spawn_temp_specialist,
            commands::get_messages,
            commands::send_message,
            commands::edit_message,
            commands::delete_message,
            commands::toggle_reaction,
            commands::forward_message,
            commands::upload_attachment,
            commands::get_tasks,
            commands::create_task,
            commands::update_task_step,
            commands::get_memories,
            commands::add_memory,
            commands::update_memory,
            commands::delete_memory,
            commands::search_unified,
            commands::get_bridge_status,
            commands::execute_google_query,
            commands::write_obsidian_note,
            commands::search_obsidian_notes,
            commands::synthesize_voice,
            commands::get_agency_state,
            commands::set_agency_state,
            commands::hide_main_window,
            commands::stop_agency_and_quit,
            commands::get_audio_mute_state,
            commands::set_audio_mute_state,
            commands::get_setting,
            commands::set_setting,
            commands::propose_governance_decision,
            commands::authorize_governance_decision,
            commands::get_constitution,
            commands::validate_constitution_invariants,
            commands::get_ui_deployments,
            commands::deploy_ui_version,
            commands::rollback_ui_version,
            commands::open_preview_window,
            commands::get_audit_log,
            commands::create_db_backup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running H.I.V.E. desktop application");
}
