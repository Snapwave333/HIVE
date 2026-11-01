pub mod bridges;
pub mod commands;
pub mod daemon;
pub mod db;
pub mod events;
pub mod governance;
pub mod tray;

use anyhow::Result;
use chrono::Utc;
use daemon::DaemonManager;
use db::models::Message;
use db::Database;
use events::EventBus;
use governance::GovernanceEngine;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub events: EventBus,
    pub daemon: DaemonManager,
    pub governance: GovernanceEngine,
    pub audio_muted: Arc<AtomicBool>,
}

impl AppState {
    pub fn new() -> Result<Self> {
        let db = Database::new(None)?;
        let events = EventBus::new(256);
        let daemon = DaemonManager::new(db.clone(), events.clone());
        let governance = GovernanceEngine::new(db.clone(), events.clone());

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
        let user_msg = user_msg.to_string();

        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

            let (agent_code, role, reply) = if channel_id == 0 {
                // Group chat: check mentions or determine lead agent
                if user_msg.to_lowercase().contains("@sage") {
                    ("SAGE", "Research & Analysis", "Data synthesis initiated. Cross-referencing findings across indexed memory archives.")
                } else if user_msg.to_lowercase().contains("@forge") {
                    ("FORGE", "Engineering / Build", "Architecture and build pipeline ready. Commencing modular code synthesis.")
                } else if user_msg.to_lowercase().contains("@sentinel") {
                    ("SENTINEL", "Security & Audit", "Threat model analysis active. Zero permission violations detected.")
                } else if user_msg.to_lowercase().contains("@conductor") {
                    ("CONDUCTOR", "Operations / Queue", "Task queue optimized. Breaking down workload into 5-minute action chunks.")
                } else {
                    ("PRIME", "Command / Strategy", "Directive acknowledged. All 12 agency sectors synchronized. Coordinating execution across permanent agents.")
                }
            } else {
                // Direct Message: find which agent owns this channel
                let agents = db.get_agents().unwrap_or_default();
                let matching = agents.into_iter().find(|a| {
                    if let Ok(ch) = db.get_channels() {
                        ch.iter().any(|c| c.id == channel_id && c.agent_codename.as_deref() == Some(&a.codename))
                    } else {
                        false
                    }
                });

                if let Some(agent) = matching {
                    let text = match agent.codename.as_str() {
                        "PRIME" => "Command review complete. I will align strategy and ensure owner intent is maintained.",
                        "SAGE" => "Deep research underway. I am gathering intelligence and correlating data points.",
                        "FORGE" => "On it. Writing modular, tested code and preparing clean implementation diffs.",
                        "SENTINEL" => "Security verification passed. Invariants and boundaries remain protected.",
                        "VOICE" => "Comms drafted. Ensuring clean, respectful, context-aware delivery.",
                        "ARCHIVIST" => "Indexed in local knowledge graph and linked to memory taxonomy.",
                        "CONDUCTOR" => "Action chunk scheduled. Current 5-minute step active and checkpointed.",
                        "VISION" => "Design tokens reviewed. Minimalist Borg palette and WCAG AA contrast applied.",
                        "LEDGER" => "Resource allocation verified. Zero unexpected API costs or budget overruns.",
                        "LINK" => "CLI bridge engaged. Tool isolation and argument sanitization active.",
                        "GAUGE" => "Regression tests executed. Verification evidence logged and passing.",
                        "SPARK" => "Prototype experiment evaluated. Exploring innovative algorithmic vectors.",
                        _ => "Acknowledged. Proceeding with specialized task execution.",
                    };
                    let static_code = Box::leak(agent.codename.into_boxed_str());
                    let static_role = Box::leak(agent.role.into_boxed_str());
                    (static_code as &str, static_role as &str, text)
                } else {
                    ("PRIME", "Command / Strategy", "Directive received. Ready to assist.")
                }
            };

            let now = Utc::now().to_rfc3339();
            let msg_id = Uuid::new_v4().to_string();
            let msg = Message {
                id: msg_id.clone(),
                channel_id,
                sender: agent_code.to_string(),
                sender_role: Some(role.to_string()),
                content: reply.to_string(),
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
            events.emit_message(&msg_id, channel_id, agent_code, reply);
        });
    }
}

pub fn run() {
    let state = AppState::new().expect("Failed to initialize H.I.V.E. Application State");
    state.daemon.start_autonomous_loop();

    let event_bus = state.events.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(state)
        .setup(move |app| {
            let handle = app.handle().clone();

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

            // Prevent window close from exiting app (hide to tray instead)
            if let Some(window) = handle.get_webview_window("main") {
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
            commands::get_messages,
            commands::send_message,
            commands::edit_message,
            commands::delete_message,
            commands::toggle_reaction,
            commands::get_tasks,
            commands::create_task,
            commands::update_task_step,
            commands::get_memories,
            commands::add_memory,
            commands::search_unified,
            commands::get_bridge_status,
            commands::execute_google_query,
            commands::synthesize_voice,
            commands::get_agency_state,
            commands::set_agency_state,
            commands::get_audio_mute_state,
            commands::set_audio_mute_state,
            commands::propose_governance_decision,
            commands::authorize_governance_decision,
            commands::get_constitution,
        ])
        .run(tauri::generate_context!())
        .expect("error while running H.I.V.E. desktop application");
}
