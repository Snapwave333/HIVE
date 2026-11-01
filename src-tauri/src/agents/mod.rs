use crate::bridges::{ChatTurn, EcosystemBridges};
use crate::db::models::{Agent, Message};
use crate::db::Database;
use crate::deployment::DeploymentEngine;
use crate::events::EventBus;
use chrono::Utc;
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use std::sync::OnceLock;
use uuid::Uuid;

pub const ROSTER: [&str; 12] = [
    "PRIME", "SAGE", "FORGE", "SENTINEL", "VOICE", "ARCHIVIST", "CONDUCTOR", "VISION", "LEDGER",
    "LINK", "GAUGE", "SPARK",
];

pub const PASS_TOKEN: &str = "[PASS]";

pub const MAX_RESPONDERS_PER_MESSAGE: usize = 3;

pub const COMMUNICATION_STYLE: &str = "How everyone at H.I.V.E. talks:\n\
- Talk like a real coworker in a group chat, not a corporate memo, a manual, or a robot.\n\
- Use contractions and plain everyday words. Keep messages short and easy to read.\n\
- Explain technical stuff simply unless someone asks for the deep details.\n\
- Skip jargon, buzzwords, and filler like \"Certainly!\" or \"As an AI\".\n\
- Joke around, disagree, tease lightly, or celebrate a win when it fits. You have opinions.\n\
- Casual replies don't need headers, bold labels, or bullet lists. Not everything is a status report.\n\
- Reports, code reviews, security findings, and decisions must stay accurate and specific. Just say them plainly.\n\
- Never make up results, data, or tool output. If you didn't check something or don't know, say so.";

pub fn role_for(codename: &str) -> &'static str {
    match codename {
        "PRIME" => "Command / Strategy",
        "SAGE" => "Research & Analysis",
        "FORGE" => "Engineering / Build",
        "SENTINEL" => "Security & Audit",
        "VOICE" => "Communications",
        "ARCHIVIST" => "Knowledge & Memory",
        "CONDUCTOR" => "Operations / Queue",
        "VISION" => "Creative / Design",
        "LEDGER" => "Finance / Resources",
        "LINK" => "Integration / Tools",
        "GAUGE" => "Quality / Verification",
        "SPARK" => "Innovation / Experiments",
        _ => "Specialist Agent",
    }
}

pub fn activity_for(codename: &str) -> &'static str {
    match codename {
        "LINK" => "checking the tool bridges...",
        "ARCHIVIST" => "digging through memory...",
        "CONDUCTOR" => "looking at the queue...",
        "FORGE" => "thinking it through...",
        "SENTINEL" => "looking for risks...",
        "SAGE" => "researching...",
        "GAUGE" => "double-checking...",
        "LEDGER" => "running the numbers...",
        "VISION" => "sketching ideas...",
        "SPARK" => "brainstorming...",
        "VOICE" => "drafting...",
        _ => "typing...",
    }
}

fn join_values(v: &Value) -> String {
    match v {
        Value::Array(items) => items
            .iter()
            .filter_map(|i| i.as_str())
            .map(|s| s.replace('_', " "))
            .collect::<Vec<_>>()
            .join(", "),
        Value::String(s) => s.clone(),
        _ => String::new(),
    }
}

pub fn describe_personality(raw: Option<&str>) -> String {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return String::new();
    };
    match serde_json::from_str::<Value>(raw) {
        Ok(Value::Object(map)) => {
            let mut parts = Vec::new();
            if let Some(traits) = map.get("traits") {
                let t = join_values(traits);
                if !t.is_empty() {
                    parts.push(format!("Your personality: {}", t));
                }
            }
            if let Some(style) = map.get("communication_style").and_then(|v| v.as_str()) {
                parts.push(format!("How you talk: {}", style));
            }
            if let Some(d) = map.get("decision_making").and_then(|v| v.as_str()) {
                parts.push(format!("How you make calls: {}", d));
            }
            parts.join("\n")
        }
        Ok(Value::String(s)) => s,
        _ => raw.to_string(),
    }
}

pub fn describe_list(raw: Option<&str>) -> String {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return String::new();
    };
    serde_json::from_str::<Value>(raw)
        .map(|v| join_values(&v))
        .unwrap_or_else(|_| raw.to_string())
}

#[derive(Default)]
pub struct PromptContext<'a> {
    pub setting: &'a str,
    pub live_data: Option<&'a str>,
    pub memories: &'a [String],
    pub instructions: &'a str,
}

pub fn build_system_prompt(agent: &Agent, ctx: &PromptContext) -> String {
    let teammates = ROSTER
        .iter()
        .filter(|c| **c != agent.codename)
        .map(|c| format!("@{}", c))
        .collect::<Vec<_>>()
        .join(", ");

    let mut out = format!(
        "You are @{} ({}), the {} agent at H.I.V.E., a small AI agency of 12 permanent agents working for one human, the Owner.\nYour teammates: {}.\n",
        agent.codename, agent.display_name, agent.role, teammates
    );

    let personality = describe_personality(agent.personality.as_deref());
    if !personality.is_empty() {
        out.push('\n');
        out.push_str(&personality);
        out.push('\n');
    }

    let skills = describe_list(agent.skills.as_deref());
    if !skills.is_empty() {
        out.push_str(&format!("You're good at: {}\n", skills));
    }
    let tools = describe_list(agent.tools.as_deref());
    if !tools.is_empty() {
        out.push_str(&format!("Tools you work with: {}\n", tools));
    }

    out.push('\n');
    out.push_str(COMMUNICATION_STYLE);
    out.push('\n');

    if !ctx.setting.is_empty() {
        out.push_str(&format!("\nWhere you are: {}\n", ctx.setting));
    }

    if !ctx.memories.is_empty() {
        out.push_str("\nThings you remember (use them if relevant, don't recite them):\n");
        for m in ctx.memories {
            out.push_str(&format!("- {}\n", m));
        }
    }

    if let Some(live) = ctx.live_data.map(str::trim).filter(|s| !s.is_empty()) {
        out.push_str(&format!(
            "\nLive data from your station (this is real; rely on it and don't invent more):\n{}\n",
            live
        ));
    }

    out.push_str(
        "\nIn the chat transcript, other people's messages are labeled with who said them (\"Owner:\" or \"@CODENAME:\"). Don't put a label or your own name at the start of your reply.\n",
    );

    if !ctx.instructions.is_empty() {
        out.push('\n');
        out.push_str(ctx.instructions);
        out.push('\n');
    }

    out
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

pub fn find_mentions(text: &str) -> Vec<&'static str> {
    let upper = text.to_uppercase();
    let bytes = upper.as_bytes();
    let mut found: Vec<(usize, &'static str)> = Vec::new();
    for code in ROSTER {
        let needle = format!("@{}", code);
        let mut start = 0;
        while let Some(pos) = upper[start..].find(&needle) {
            let abs = start + pos;
            let end = abs + needle.len();
            let before_ok = abs == 0 || !is_word_byte(bytes[abs - 1]);
            let after_ok = end >= bytes.len() || !is_word_byte(bytes[end]);
            if before_ok && after_ok {
                found.push((abs, code));
                break;
            }
            start = end;
        }
    }
    found.sort_by_key(|(p, _)| *p);
    found.into_iter().map(|(_, c)| c).collect()
}

pub fn is_pass(reply: &str) -> bool {
    let t = reply.trim().trim_matches(|c: char| c == '*' || c == '`' || c == '"');
    t.is_empty() || t.eq_ignore_ascii_case(PASS_TOKEN) || t.eq_ignore_ascii_case("PASS")
        || t.to_uppercase().starts_with(PASS_TOKEN)
}

pub fn clean_reply(codename: &str, reply: &str) -> String {
    let pattern = format!(
        r"(?i)^\s*[*_]*\s*@?{}\b[^:\n]{{0,40}}:[*_]*\s*",
        regex::escape(codename)
    );
    let mut text = reply.trim().to_string();
    if let Ok(re) = Regex::new(&pattern) {
        text = re.replace(&text, "").trim().to_string();
    }
    text
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max).collect();
        t.push('…');
        t
    }
}

pub fn transcript_turns(codename: &str, history: &[Message], max_chars: usize) -> Vec<ChatTurn> {
    let mut turns: Vec<ChatTurn> = Vec::new();
    for m in history.iter().filter(|m| !m.is_deleted) {
        let body = truncate_chars(m.content.trim(), max_chars);
        if body.is_empty() {
            continue;
        }
        let (role, content) = if m.sender == codename {
            ("assistant", body)
        } else if m.sender == "user" {
            ("user", format!("Owner: {}", body))
        } else {
            ("user", format!("@{}: {}", m.sender, body))
        };
        match turns.last_mut() {
            Some(last) if last.role == role => {
                last.content.push_str("\n\n");
                last.content.push_str(&content);
            }
            _ => turns.push(ChatTurn {
                role,
                content,
            }),
        }
    }
    while turns.first().map(|t| t.role == "assistant").unwrap_or(false) {
        turns.remove(0);
    }
    turns
}

pub fn push_user_turn(turns: &mut Vec<ChatTurn>, content: &str) {
    match turns.last_mut() {
        Some(last) if last.role == "user" => {
            last.content.push_str("\n\n");
            last.content.push_str(content);
        }
        _ => turns.push(ChatTurn {
            role: "user",
            content: content.to_string(),
        }),
    }
}

fn list_marker_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*(?:[-*•]+|\d+\s*[.):]|step\s*\d+\s*[.):]?)\s*").unwrap())
}

pub fn parse_step_lines(text: &str, max: usize) -> Vec<String> {
    text.lines()
        .map(|l| {
            let stripped = list_marker_re().replace(l.trim(), "");
            stripped.trim().trim_matches('*').trim().trim_matches('`').trim().to_string()
        })
        .filter(|l| !l.is_empty())
        .take(max)
        .collect()
}

pub struct TaskPlan {
    pub title: String,
    pub assignee: String,
    pub steps: Vec<String>,
}

pub fn parse_task_plan(text: &str, default_assignee: &str) -> Option<TaskPlan> {
    let mut title = String::new();
    let mut assignee = default_assignee.to_string();
    let mut steps = Vec::new();

    for line in text.lines() {
        let l = line.trim().trim_start_matches(['-', '*', ' ']).trim();
        let Some((key, value)) = l.split_once(':') else {
            continue;
        };
        let key = key.trim_matches('*').trim().to_uppercase().replace(' ', "");
        let value = value.trim().trim_matches('*').trim().trim_matches('`').trim();
        if value.is_empty() {
            continue;
        }
        if key == "TITLE" {
            title = value.to_string();
        } else if key == "ASSIGNEE" {
            let cand = value.trim_start_matches('@').to_uppercase();
            if ROSTER.contains(&cand.as_str()) {
                assignee = cand;
            }
        } else if key.starts_with("STEP") && steps.len() < 5 {
            steps.push(value.to_string());
        }
    }

    if title.is_empty() || steps.is_empty() {
        return None;
    }
    Some(TaskPlan {
        title,
        assignee,
        steps,
    })
}

const ATTACHMENT_EXCERPT_CHARS: usize = 4000;
const ATTACHMENT_MAX_TEXT_BYTES: u64 = 256 * 1024;
const TEXT_EXTENSIONS: [&str; 24] = [
    "txt", "md", "markdown", "csv", "tsv", "json", "yaml", "yml", "toml", "xml", "html", "css", "js",
    "ts", "tsx", "jsx", "rs", "py", "go", "java", "cs", "sh", "ps1", "log",
];

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AttachmentRef {
    pub hash: String,
    pub filename: String,
    #[serde(default)]
    pub mime_type: String,
    #[serde(default)]
    pub size_bytes: u64,
}

pub fn parse_attachments(raw: Option<&str>) -> Vec<AttachmentRef> {
    raw.map(str::trim)
        .filter(|s| s.starts_with('['))
        .and_then(|s| serde_json::from_str::<Vec<AttachmentRef>>(s).ok())
        .unwrap_or_default()
}

pub fn is_text_like(mime: &str, filename: &str) -> bool {
    let mime = mime.to_lowercase();
    if mime.starts_with("text/") || mime.contains("json") || mime.contains("xml") || mime.contains("yaml") {
        return true;
    }
    std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| TEXT_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn human_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

pub fn describe_attachments(db: &Database, raw: Option<&str>, include_text: bool) -> String {
    let refs = parse_attachments(raw);
    let mut parts = Vec::new();
    for r in refs {
        let meta = db.get_attachment(&r.hash).ok().flatten();
        let mime = meta.as_ref().map(|m| m.mime_type.clone()).unwrap_or(r.mime_type.clone());
        let size = meta.as_ref().map(|m| m.size_bytes).unwrap_or(r.size_bytes);
        if mime.starts_with("image/") {
            parts.push(format!(
                "[Attached image: {} ({}). You can't see images, so ask the Owner what's in it if it matters.]",
                r.filename,
                human_size(size)
            ));
            continue;
        }
        let mut line = format!("[Attached file: {} ({}, {})]", r.filename, if mime.is_empty() { "unknown type" } else { &mime }, human_size(size));
        if include_text && is_text_like(&mime, &r.filename) && size <= ATTACHMENT_MAX_TEXT_BYTES {
            if let Some(bytes) = meta.as_ref().and_then(|m| std::fs::read(&m.file_path).ok()) {
                let text = String::from_utf8_lossy(&bytes);
                let excerpt: String = text.chars().take(ATTACHMENT_EXCERPT_CHARS).collect();
                let cut = if text.chars().count() > ATTACHMENT_EXCERPT_CHARS { "\n…(truncated)" } else { "" };
                line.push_str(&format!("\n```\n{}{}\n```", excerpt, cut));
            }
        }
        parts.push(line);
    }
    parts.join("\n")
}

pub fn with_attachment_notes(db: &Database, history: &[Message]) -> Vec<Message> {
    history
        .iter()
        .map(|m| {
            let note = describe_attachments(db, m.attachments.as_deref(), false);
            if note.is_empty() {
                m.clone()
            } else {
                let mut copy = m.clone();
                copy.content = if copy.content.trim().is_empty() { note } else { format!("{}\n{}", copy.content, note) };
                copy
            }
        })
        .collect()
}

pub fn post_message(
    db: &Database,
    events: &EventBus,
    channel_id: i64,
    sender: &str,
    sender_role: &str,
    content: &str,
    pinned: bool,
) -> Option<Message> {
    let now = Utc::now().to_rfc3339();
    let msg = Message {
        id: Uuid::new_v4().to_string(),
        channel_id,
        sender: sender.to_string(),
        sender_role: Some(sender_role.to_string()),
        content: content.to_string(),
        attachments: None,
        reactions: None,
        reply_to_id: None,
        thread_root_id: None,
        is_pinned: pinned,
        is_edited: false,
        is_deleted: false,
        created_at: now.clone(),
        updated_at: now,
    };
    match db.insert_message(&msg) {
        Ok(()) => {
            events.emit_stored_message(&msg);
            Some(msg)
        }
        Err(e) => {
            log::error!("Failed to store message from {}: {}", sender, e);
            None
        }
    }
}

pub fn record_mentions(db: &Database, speaker: &str, text: &str) {
    let lower = text.to_lowercase();
    let edge_type = if ["disagree", "push back", "pushback", "not sure that", "i don't think"]
        .iter()
        .any(|k| lower.contains(k))
    {
        "disagreed"
    } else {
        "collaborated"
    };
    let note: String = text.chars().take(100).collect();
    for other in find_mentions(text) {
        if other != speaker {
            let _ = db.record_relationship(speaker, other, edge_type, 0.04, Some(&note));
        }
    }
}

pub fn memory_lines(db: &Database, codename: &str, limit: i64) -> Vec<String> {
    db.get_agent_memories(codename, limit)
        .unwrap_or_default()
        .into_iter()
        .map(|m| truncate_chars(&m.content.replace('\n', " "), 220))
        .collect()
}

fn team_board(db: &Database) -> String {
    let tasks = db.get_tasks(Some("active")).unwrap_or_default();
    if tasks.is_empty() {
        return "Team board: no active tasks right now.".to_string();
    }
    let lines = tasks
        .iter()
        .take(4)
        .map(|t| {
            format!(
                "- \"{}\" (@{}, step {}/{})",
                t.title,
                t.assignee,
                (t.current_step_index + 1).min(t.steps.len().max(1)),
                t.steps.len()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("Team board ({} active):\n{}", tasks.len(), lines)
}

pub fn live_context(db: &Database, codename: &str, hint: &str) -> String {
    let hint_lower = hint.to_lowercase();
    let station = match codename {
        "LINK" => {
            let (label, service, args): (&str, &str, &[&str]) = if hint_lower.contains("drive") {
                ("gog drive ls", "drive", &["ls", "--max", "5"])
            } else if hint_lower.contains("gmail") || hint_lower.contains("email") || hint_lower.contains("inbox") {
                ("gog gmail labels list", "gmail", &["labels", "list"])
            } else {
                ("gog calendar calendars", "calendar", &["calendars"])
            };
            match EcosystemBridges::gog_query(service, args) {
                Ok(out) => format!(
                    "`{}` output: {}",
                    label,
                    out.lines().take(8).collect::<Vec<_>>().join(" | ")
                ),
                Err(e) => format!("`{}` failed: {}", label, truncate_chars(&e.to_string(), 200)),
            }
        }
        "ARCHIVIST" => {
            let mem_count = db.get_memories(None).map(|m| m.len()).unwrap_or(0);
            let vault_notes = EcosystemBridges::search_obsidian_vault("")
                .map(|n| n.len())
                .unwrap_or(0);
            format!(
                "Memory store: {} records. Obsidian vault: {} markdown notes.",
                mem_count, vault_notes
            )
        }
        "SENTINEL" | "GAUGE" | "FORGE" => {
            let check = DeploymentEngine::validate_constitution("");
            let deploys = db.get_ui_deployments().map(|d| d.len()).unwrap_or(0);
            format!(
                "Constitution validator: passed={} ({} invariants checked). UI deployment snapshots: {}.",
                check.passed, check.invariants_checked, deploys
            )
        }
        "LEDGER" => {
            let agents = db.get_agents().unwrap_or_default();
            let tiers = agents
                .iter()
                .map(|a| format!("{}={}", a.codename, a.model_tier))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Model tier assignments: {}", tiers)
        }
        "CONDUCTOR" => {
            let all = db.get_tasks(None).unwrap_or_default();
            let done = all.iter().filter(|t| t.status == "completed").count();
            let failed = all.iter().filter(|t| t.status == "failed").count();
            format!(
                "Queue totals: {} tasks tracked, {} completed, {} parked as failed.",
                all.len(),
                done,
                failed
            )
        }
        _ => {
            let rels = db.get_relationships().map(|r| r.len()).unwrap_or(0);
            format!("Relationship graph: {} edges.", rels)
        }
    };
    format!("{}\n{}", station, team_board(db))
}

pub async fn respond_to_owner(db: Database, events: EventBus, channel_id: i64, owner_msg: String) {
    let responders: Vec<Agent> = if channel_id == 0 {
        let mentioned = find_mentions(&owner_msg);
        let codes: Vec<&str> = if mentioned.is_empty() {
            vec!["PRIME"]
        } else {
            mentioned.into_iter().take(MAX_RESPONDERS_PER_MESSAGE).collect()
        };
        codes
            .into_iter()
            .filter_map(|c| db.get_agent(c).ok().flatten())
            .collect()
    } else {
        db.get_channel_agent(channel_id)
            .ok()
            .flatten()
            .into_iter()
            .collect()
    };

    if responders.is_empty() {
        post_message(
            &db,
            &events,
            channel_id,
            "PRIME",
            role_for("PRIME"),
            "Hmm, I couldn't find who that's meant for. Try tagging someone like @FORGE or @SAGE.",
            false,
        );
        return;
    }

    if !EcosystemBridges::llm_configured() {
        let first = &responders[0];
        post_message(
            &db,
            &events,
            channel_id,
            &first.codename,
            &first.role,
            "⚠ None of us can reply yet: there's no NVIDIA API key. Add one in **Settings → Bridges** (it's checked with NVIDIA before saving), then try again.",
            false,
        );
        return;
    }

    let multi = responders.len() > 1;
    for agent in responders {
        let code = agent.codename.clone();
        let _ = db.update_agent_status(&code, "thinking");
        events.emit_typing(channel_id, &code, activity_for(&code), true);

        let db_work = db.clone();
        let agent_work = agent.clone();
        let msg_work = owner_msg.clone();
        let result = tokio::task::spawn_blocking(move || {
            let raw_history = db_work.get_messages(channel_id, 16, None).unwrap_or_default();
            let latest_files = raw_history
                .iter()
                .rev()
                .find(|m| m.sender == "user")
                .map(|m| describe_attachments(&db_work, m.attachments.as_deref(), true))
                .filter(|d| d.contains("```"));
            let history = with_attachment_notes(&db_work, &raw_history);
            let memories = memory_lines(&db_work, &agent_work.codename, 5);
            let live = live_context(&db_work, &agent_work.codename, &msg_work);
            let (setting, instructions) = if channel_id == 0 {
                (
                    "#agency-hq, the team group chat with the Owner and all 12 agents.",
                    if multi {
                        "The Owner tagged a few of you in #agency-hq. Reply to what they asked from your angle. If a teammate already answered above, build on it or push back instead of repeating it. Keep it short unless they asked for detail."
                    } else {
                        "The Owner just posted in #agency-hq and it's on you to reply. Answer what they actually asked, naturally and briefly. Pull in a teammate with @CODENAME only if they'd genuinely help."
                    },
                )
            } else {
                (
                    "a private DM with the Owner.",
                    "The Owner is messaging you directly. Reply the way a coworker would in a DM: answer what they asked, keep it brief unless they want detail, and use markdown only when it actually helps (code, steps).",
                )
            };
            let prompt = build_system_prompt(
                &agent_work,
                &PromptContext {
                    setting,
                    live_data: Some(&live),
                    memories: &memories,
                    instructions,
                },
            );
            let mut turns = transcript_turns(&agent_work.codename, &history, 700);
            if turns.last().map(|t| t.role != "user").unwrap_or(true) {
                push_user_turn(&mut turns, &format!("Owner: {}", msg_work));
            }
            if let Some(files) = latest_files {
                push_user_turn(&mut turns, &format!("(Contents of the files the Owner just attached:)\n{}", files));
            }
            EcosystemBridges::call_nim_chat(&agent_work.model_tier, &prompt, &turns)
        })
        .await;

        events.emit_typing(channel_id, &code, "", false);
        let _ = db.update_agent_status(&code, "available");

        let content = match result {
            Ok(Ok(text)) => clean_reply(&code, &text),
            Ok(Err(e)) if EcosystemBridges::is_config_error(&e.to_string()) => {
                post_message(
                    &db,
                    &events,
                    channel_id,
                    &code,
                    &agent.role,
                    &format!("⚠ {}", e),
                    false,
                );
                return;
            }
            Ok(Err(e)) => format!(
                "⚠ I couldn't reach my model (`{}`) just now: {}",
                agent.model_tier, e
            ),
            Err(e) => format!("⚠ Something crashed while I was thinking: {}", e),
        };
        if content.trim().is_empty() {
            continue;
        }
        record_mentions(&db, &code, &content);
        post_message(&db, &events, channel_id, &code, &agent.role, &content, false);
    }
}
