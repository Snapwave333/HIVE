use crate::bridges::{BridgeStatus, EcosystemBridges};
use crate::daemon::AgencyState;
use crate::db::models::{
    Agent, AttachmentMeta, AuditEntry, Channel, MemoryItem, Message, RelationshipEdge, TaskChunk,
    TempSpecialist, UiDeployment,
};
use crate::deployment::{ConstitutionReport, DeploymentEngine};
use crate::governance::{Decision, DecisionLevel};
use crate::AppState;
use chrono::Utc;
use serde_json::Value;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

#[tauri::command]
pub fn get_channels(state: State<'_, AppState>) -> Result<Vec<Channel>, String> {
    state.db.get_channels().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_agents(state: State<'_, AppState>) -> Result<Vec<Agent>, String> {
    state.db.get_agents().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_relationships(state: State<'_, AppState>) -> Result<Vec<RelationshipEdge>, String> {
    state.db.get_relationships().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_temp_specialists(state: State<'_, AppState>) -> Result<Vec<TempSpecialist>, String> {
    let _ = state.db.retire_expired_specialists();
    state.db.get_temp_specialists().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn spawn_temp_specialist(
    name: String,
    role: String,
    sponsor: String,
    ttl_minutes: Option<i64>,
    state: State<'_, AppState>,
) -> Result<TempSpecialist, String> {
    let spec = state
        .db
        .spawn_temp_specialist(&name, &role, &sponsor, ttl_minutes.unwrap_or(30))
        .map_err(|e| e.to_string())?;
    let _ = state.db.log_audit(
        "spawn_temp_specialist",
        &sponsor,
        &format!("Spawned temporary specialist {} (expires: {})", spec.codename, spec.expires_at),
        "ACTIVE",
    );
    Ok(spec)
}

#[tauri::command]
pub fn get_messages(
    channel_id: i64,
    limit: Option<i64>,
    before_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Message>, String> {
    state
        .db
        .get_messages(channel_id, limit.unwrap_or(80), before_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn send_message(
    channel_id: i64,
    content: String,
    attachments: Option<String>,
    reply_to_id: Option<String>,
    thread_root_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Message, String> {
    let now = Utc::now().to_rfc3339();
    let msg_id = Uuid::new_v4().to_string();

    let msg = Message {
        id: msg_id.clone(),
        channel_id,
        sender: "user".to_string(),
        sender_role: Some("Owner".to_string()),
        content: content.clone(),
        attachments,
        reactions: None,
        reply_to_id,
        thread_root_id,
        is_pinned: false,
        is_edited: false,
        is_deleted: false,
        created_at: now.clone(),
        updated_at: now,
    };

    state.db.insert_message(&msg).map_err(|e| e.to_string())?;
    state.events.emit_message(&msg_id, channel_id, "user", &content);

    // Trigger agent or slash-command response
    state.trigger_agent_reply(channel_id, &content);

    Ok(msg)
}

#[tauri::command]
pub fn edit_message(
    message_id: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .db
        .update_message(&message_id, &content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_message(message_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.db.delete_message(&message_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_reaction(
    message_id: String,
    emoji: String,
    user_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state
        .db
        .toggle_reaction(&message_id, &emoji, &user_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn forward_message(
    message_id: String,
    target_channel_id: i64,
    state: State<'_, AppState>,
) -> Result<Message, String> {
    let fwd = state
        .db
        .forward_message(&message_id, target_channel_id)
        .map_err(|e| e.to_string())?;
    state
        .events
        .emit_message(&fwd.id, target_channel_id, "user", &fwd.content);
    Ok(fwd)
}

#[tauri::command]
pub fn upload_attachment(
    filename: String,
    mime_type: String,
    data_base64: String,
    state: State<'_, AppState>,
) -> Result<AttachmentMeta, String> {
    state
        .db
        .save_attachment(&filename, &mime_type, &data_base64)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_tasks(
    status_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<TaskChunk>, String> {
    state
        .db
        .get_tasks(status_filter.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_task(
    title: String,
    assignee: String,
    steps: Vec<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let steps_tuple: Vec<(&str, u32)> = steps.iter().map(|s| (s.as_str(), 5)).collect();
    state
        .daemon
        .create_task_chunk(&title, &assignee, steps_tuple)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_task_step(
    task_id: String,
    step_index: usize,
    completed: bool,
    checkpoint: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .db
        .update_task_step(&task_id, step_index, completed, checkpoint.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_memories(
    category: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<MemoryItem>, String> {
    state
        .db
        .get_memories(category.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_memory(
    category: String,
    content: String,
    provenance: String,
    is_speculation: bool,
    tags: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let mem_id = Uuid::new_v4().to_string();
    let mem = MemoryItem {
        id: mem_id.clone(),
        category,
        content,
        provenance,
        confidence: if is_speculation { 0.5 } else { 1.0 },
        is_speculation,
        tags,
        created_at: Utc::now().to_rfc3339(),
    };
    state.db.insert_memory(&mem).map_err(|e| e.to_string())?;
    Ok(mem_id)
}

#[tauri::command]
pub fn update_memory(
    id: String,
    content: String,
    is_speculation: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .db
        .update_memory(&id, &content, is_speculation)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_memory(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.db.delete_memory(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_unified(query: String, state: State<'_, AppState>) -> Result<Value, String> {
    state.db.search(&query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_bridge_status() -> Result<BridgeStatus, String> {
    Ok(EcosystemBridges::get_status())
}

#[tauri::command]
pub fn execute_google_query(service: String, args: Vec<String>) -> Result<String, String> {
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    EcosystemBridges::gog_query(&service, &arg_refs).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_obsidian_note(
    title: String,
    content: String,
    folder: Option<String>,
) -> Result<String, String> {
    let path = EcosystemBridges::write_obsidian_note(&title, &content, folder.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn search_obsidian_notes(query: String) -> Result<Vec<Value>, String> {
    EcosystemBridges::search_obsidian_notes(&query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn synthesize_voice(text: String, agent_codename: String) -> Result<String, String> {
    EcosystemBridges::synthesize_speech_data_url(&text, &agent_codename)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_agency_state(state: State<'_, AppState>) -> Result<String, String> {
    let s = match state.daemon.state() {
        AgencyState::Running => "running",
        AgencyState::Paused => "paused",
        AgencyState::Stopped => "stopped",
    };
    Ok(s.to_string())
}

#[tauri::command]
pub fn set_agency_state(state_name: String, state: State<'_, AppState>) -> Result<(), String> {
    match state_name.as_str() {
        "running" => state.daemon.resume(),
        "paused" => state.daemon.pause(),
        "stopped" => state.daemon.stop(),
        _ => return Err("Invalid state".to_string()),
    }
    Ok(())
}

#[tauri::command]
pub fn hide_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn stop_agency_and_quit(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    state.daemon.stop();
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn get_audio_mute_state(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.is_audio_muted())
}

#[tauri::command]
pub fn set_audio_mute_state(is_muted: bool, state: State<'_, AppState>) -> Result<(), String> {
    state.set_audio_muted(is_muted);
    Ok(())
}

#[tauri::command]
pub fn get_setting(key: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    state.db.get_setting(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_setting(key: String, value: String, state: State<'_, AppState>) -> Result<(), String> {
    state.db.set_setting(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn propose_governance_decision(
    title: String,
    description: String,
    level: u8,
    proposer: String,
    state: State<'_, AppState>,
) -> Result<Decision, String> {
    let lvl = match level {
        1 => DecisionLevel::Routine,
        2 => DecisionLevel::Significant,
        3 => DecisionLevel::OwnerControlled,
        _ => return Err("Invalid decision level".to_string()),
    };
    state
        .governance
        .propose_decision(&title, &description, lvl, &proposer)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn authorize_governance_decision(
    decision_id: String,
    approve: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .governance
        .owner_authorize(&decision_id, approve)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_constitution() -> Result<Value, String> {
    let const_str = include_str!("../../app/constitution.json");
    serde_json::from_str(const_str).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn validate_constitution_invariants(candidate_source: String) -> Result<ConstitutionReport, String> {
    Ok(DeploymentEngine::validate_constitution(&candidate_source))
}

#[tauri::command]
pub fn get_ui_deployments(state: State<'_, AppState>) -> Result<Vec<UiDeployment>, String> {
    state.db.get_ui_deployments().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn deploy_ui_version(
    version: String,
    author_agent: String,
    summary: String,
    activate: bool,
    state: State<'_, AppState>,
) -> Result<UiDeployment, String> {
    state
        .deployment
        .stage_and_deploy_version(&version, &author_agent, &summary, activate)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rollback_ui_version(
    reason: String,
    state: State<'_, AppState>,
) -> Result<UiDeployment, String> {
    state.deployment.rollback(&reason).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_preview_window(app: AppHandle, version: String) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window("preview") {
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }
    WebviewWindowBuilder::new(&app, "preview", WebviewUrl::App("index.html?preview=1".into()))
        .title(format!("H.I.V.E. Isolated Preview Sandbox — {}", version))
        .inner_size(1020.0, 700.0)
        .center()
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_audit_log(limit: Option<i64>, state: State<'_, AppState>) -> Result<Vec<AuditEntry>, String> {
    state
        .db
        .get_audit_log(limit.unwrap_or(50))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_db_backup(state: State<'_, AppState>) -> Result<String, String> {
    let path = state.db.create_backup().map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}
