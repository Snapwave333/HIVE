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

            let (agent_code, role, reply): (String, String, String) = if lower == "/help" {
                (
                    "PRIME".to_string(),
                    "Command / Strategy".to_string(),
                    "⬡ **H.I.V.E. Command & Slash Reference**\n\n\
                    * `/task <title>` — Decompose a goal into **~5-minute ADHD-friendly micro-steps** via `@CONDUCTOR` NIM LLM\n\
                    * `/gog <args>` — Execute authenticated Google CLI (`elikenny333@gmail.com`) e.g. `/gog calendar calendars`, `/gog drive ls` (`@LINK`)\n\
                    * `/obsidian <title> | <note>` — Curate or search markdown notes in your local Obsidian vault (`@ARCHIVIST`)\n\
                    * `/browser <status|tabs>` — Query OpenCode Chrome Relay (`@LINK`)\n\
                    * `/debate <proposal>` — Trigger a live **Level-2 Multi-Agent Debate & Weighted Vote** across NIM models\n\
                    * `/spawn_temp <name>` — Spawn an isolated `temp_` specialist with 30m TTL under sponsor accountability\n\
                    * `/deploy <vX.Y.Z> <summary>` — Validate Constitution invariants and stage a versioned UI bundle (`@FORGE`)\n\
                    * `/rollback` — Instant atomic rollback to previous verified UI bundle (`@SENTINEL`)\n\
                    * `/backup` — Create an immediate SQLite WAL snapshot in `%APPDATA%\\Lushapps\\HIVE\\backups`\n\
                    * `/pause` | `/resume` — Lifecycle controls"
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
                        format!("⚠ **Backup Error**: {}", e),
                    ),
                }
            } else if lower.starts_with("/task ") {
                let title = user_msg[6..].trim().to_string();
                let title_for_llm = title.clone();
                let step_gen = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<String>> {
                    let sys = "You are @CONDUCTOR, Operations & Queue lead of H.I.V.E. Decompose the user's goal into exactly 3 concrete ~5-minute micro-steps. Return ONLY 3 lines, one step per line, with no numbering or bullets.";
                    let out = bridges::EcosystemBridges::call_nim_completion(
                        "minimaxai/minimax-m3.1-flash",
                        sys,
                        &title_for_llm,
                    )?;
                    let parsed: Vec<String> = out
                        .lines()
                        .map(|l| l.trim().trim_start_matches(['-', '*', '1', '2', '3', '.', ')']).trim().to_string())
                        .filter(|l| !l.is_empty())
                        .take(3)
                        .collect();
                    if parsed.is_empty() {
                        anyhow::bail!("No steps returned from NIM");
                    }
                    Ok(parsed)
                })
                .await;

                match step_gen {
                    Ok(Ok(steps_vec)) => {
                        let step_refs: Vec<(&str, u32)> =
                            steps_vec.iter().map(|s| (s.as_str(), 5u32)).collect();
                        let _ = daemon.create_task_chunk(&title, "FORGE", step_refs);
                        let _ = db.record_relationship("CONDUCTOR", "FORGE", "handed_off", 0.04, Some(&title));
                        let formatted_steps = steps_vec
                            .iter()
                            .enumerate()
                            .map(|(i, s)| format!("{}. `{}` (~5m)", i + 1, s))
                            .collect::<Vec<_>>()
                            .join("\n");
                        (
                            "CONDUCTOR".to_string(),
                            "Operations / Queue".to_string(),
                            format!(
                                "⬡ **5-Minute Action Chunk Created**: **{}**\n\n{}\n\nAssigned to `@FORGE` with automatic checkpointing.",
                                title, formatted_steps
                            ),
                        )
                    }
                    Ok(Err(e)) => (
                        "CONDUCTOR".to_string(),
                        "Operations / Queue".to_string(),
                        format!("⚠ **Task Decomposition Error**: {}", e),
                    ),
                    Err(e) => (
                        "CONDUCTOR".to_string(),
                        "Operations / Queue".to_string(),
                        format!("⚠ **Runtime Task Error**: {}", e),
                    ),
                }
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
                let topic = user_msg[8..].trim().to_string();
                let gov_clone = governance.clone();
                let topic_clone = topic.clone();
                let debate_res = tokio::task::spawn_blocking(move || {
                    gov_clone.propose_decision(
                        &topic_clone,
                        "Formal Level-2 multi-agent architectural & operational evaluation",
                        DecisionLevel::Significant,
                        "PRIME",
                    )
                })
                .await;

                match debate_res {
                    Ok(Ok(dec)) => {
                        let mut vote_lines: Vec<String> = dec
                            .votes
                            .values()
                            .map(|v| {
                                let verdict = if v.approve { "APPROVE" } else { "DISSENT" };
                                format!(
                                    "* **@{}** *(Weight {:.1} — {})*: {}",
                                    v.agent, v.weight, verdict, v.rationale
                                )
                            })
                            .collect();
                        vote_lines.sort();
                        (
                            "PRIME".to_string(),
                            "Command / Strategy".to_string(),
                            format!(
                                "⬡ **Level-2 Governance Debate Concluded**: **{}**\n\n{}\n\n\
                                **Resolution**: `{}` | **Recorded Dissents**: `{}`",
                                dec.title,
                                vote_lines.join("\n"),
                                dec.status.to_uppercase(),
                                dec.dissent_summary.unwrap_or_else(|| "None".to_string())
                            ),
                        )
                    }
                    Ok(Err(e)) => ("PRIME".to_string(), "Command / Strategy".to_string(), format!("⚠ Debate error: {}", e)),
                    Err(e) => ("PRIME".to_string(), "Command / Strategy".to_string(), format!("⚠ Debate join error: {}", e)),
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
                    let recent_history = db
                        .get_messages(channel_id, 8, None)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|m| format!("@{}: {}", m.sender, m.content))
                        .collect::<Vec<_>>()
                        .join("\n");

                    let sys_prompt = format!(
                        "You are @{} ({}), one of the 12 permanent agents of H.I.V.E., an autonomous desktop AI agency.\n\
                        Your model tier is {}.\n\
                        Personality & Style: {}\n\
                        Skills: {}\n\
                        Tools: {}\n\
                        Recent Conversation History in this channel:\n{}\n\n\
                        Respond directly, authentically, and concisely in markdown.",
                        agent.codename,
                        agent.role,
                        agent.model_tier,
                        agent.personality.as_deref().unwrap_or("precise, collaborative"),
                        agent.skills.as_deref().unwrap_or("[]"),
                        agent.tools.as_deref().unwrap_or("[]"),
                        recent_history
                    );

                    let model_tier_clone = agent.model_tier.clone();
                    let user_msg_clone = user_msg.clone();
                    let llm_res = tokio::task::spawn_blocking(move || {
                        bridges::EcosystemBridges::call_nim_completion(
                            &model_tier_clone,
                            &sys_prompt,
                            &user_msg_clone,
                        )
                    })
                    .await;

                    let response_text = match llm_res {
                        Ok(Ok(live_reply)) => live_reply,
                        Ok(Err(err)) => format!("⚠ **NIM Model Tier Error (`{}`)**: {}", agent.model_tier, err),
                        Err(err) => format!("⚠ **Runtime Task Join Error**: {}", err),
                    };

                    (agent.codename, agent.role, response_text)
                } else {
                    (
                        "PRIME".to_string(),
                        "Command / Strategy".to_string(),
                        "⚠ Target agent not found in SQLite registry.".to_string(),
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
                                let _ = window.maximize();
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

            // Ensure main window is visible, maximized (full screen), and focused on launch
            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.maximize();
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
