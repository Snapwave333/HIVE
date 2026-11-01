use crate::bridges::EcosystemBridges;
use crate::db::models::{MemoryItem, Message, TaskChunk, TaskStep};
use crate::db::Database;
use crate::deployment::DeploymentEngine;
use crate::events::{EventBus, HiveEvent};
use chrono::Utc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgencyState {
    Running,
    Paused,
    Stopped,
}

#[derive(Clone)]
pub struct DaemonManager {
    db: Database,
    event_bus: EventBus,
    is_running: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
}

impl DaemonManager {
    pub fn new(db: Database, event_bus: EventBus) -> Self {
        Self {
            db,
            event_bus,
            is_running: Arc::new(AtomicBool::new(true)),
            is_paused: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn state(&self) -> AgencyState {
        if !self.is_running.load(Ordering::SeqCst) {
            AgencyState::Stopped
        } else if self.is_paused.load(Ordering::SeqCst) {
            AgencyState::Paused
        } else {
            AgencyState::Running
        }
    }

    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
        let _ = self.db.set_setting("agency_state", "paused");
        self.event_bus.publish(HiveEvent::SystemLifecycle {
            state: "paused".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        });
        let _ = self.db.log_audit(
            "agency_state_change",
            "USER",
            "Agency state changed to PAUSED",
            "SUCCESS",
        );
    }

    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
        let _ = self.db.set_setting("agency_state", "running");
        self.event_bus.publish(HiveEvent::SystemLifecycle {
            state: "running".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        });
        let _ = self.db.log_audit(
            "agency_state_change",
            "USER",
            "Agency state changed to RUNNING",
            "SUCCESS",
        );
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        let _ = self.db.set_setting("agency_state", "stopped");
        self.event_bus.publish(HiveEvent::SystemLifecycle {
            state: "stopped".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        });
        let _ = self.db.log_audit(
            "agency_state_change",
            "USER",
            "Agency state changed to STOPPED",
            "SUCCESS",
        );
    }

    pub fn start_autonomous_loop(&self) {
        let db = self.db.clone();
        let event_bus = self.event_bus.clone();
        let is_running = self.is_running.clone();
        let is_paused = self.is_paused.clone();
        let self_clone = self.clone();

        tauri::async_runtime::spawn(async move {
            log::info!("H.I.V.E. Live Autonomous Daemon started (100% real NIM LLM + CLI execution).");
            let mut turn_index: usize = 0;

            sleep(Duration::from_secs(3)).await;

            while is_running.load(Ordering::SeqCst) {
                if is_paused.load(Ordering::SeqCst) {
                    sleep(Duration::from_secs(3)).await;
                    continue;
                }

                // 1. Retire any expired temp_ specialists
                let _ = db.retire_expired_specialists();

                // 2. Execute real active 5-minute task step if any exists in SQLite
                let mut executed_task_step = false;
                if let Ok(tasks) = db.get_tasks(Some("active")) {
                    for task in tasks {
                        if task.current_step_index < task.steps.len() {
                            let assignee = task.assignee.clone();
                            let step = task.steps[task.current_step_index].clone();
                            let step_num = task.current_step_index + 1;
                            let total_steps = task.steps.len();

                            let agent_record = db
                                .get_agents()
                                .ok()
                                .and_then(|list| list.into_iter().find(|a| a.codename == assignee));
                            let model_tier = agent_record
                                .as_ref()
                                .map(|a| a.model_tier.clone())
                                .unwrap_or_else(|| "nvidia/nemotron-3.5-lightning-30b-a3b".to_string());
                            let role = agent_record
                                .as_ref()
                                .map(|a| a.role.clone())
                                .unwrap_or_else(|| Self::role_for(&assignee).to_string());
                            let personality = agent_record
                                .as_ref()
                                .and_then(|a| a.personality.clone())
                                .unwrap_or_default();

                            // Set real thinking status while live tool + NIM LLM execute
                            let _ = db.update_agent_status(&assignee, "thinking");
                            event_bus.emit_typing(
                                0,
                                &assignee,
                                &format!("executing task step {}/{}: {}...", step_num, total_steps, step.title),
                                true,
                            );

                            let db_for_work = db.clone();
                            let task_title = task.title.clone();
                            let step_title = step.title.clone();
                            let assignee_clone = assignee.clone();
                            let role_clone = role.clone();

                            let step_result = tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
                                // Gather real live tool context relevant to the step/agent
                                let live_tool_context = Self::gather_live_tool_context(
                                    &db_for_work,
                                    &assignee_clone,
                                    &step_title,
                                );

                                let sys_prompt = format!(
                                    "You are @{}, {} of the H.I.V.E. autonomous AI agency.\n\
                                     Personality & Style: {}\n\
                                     You are currently executing Step {}/{} ('{}') of the 5-minute chunked task '{}'.\n\
                                     Live System & Bridge Data:\n{}\n\n\
                                     Write a concise, genuine report (2-4 sentences or bullet points) of the concrete work and findings you just completed for this step based on the live system data above. Do not use placeholders.",
                                    assignee_clone,
                                    role_clone,
                                    personality,
                                    step_num,
                                    total_steps,
                                    step_title,
                                    task_title,
                                    live_tool_context
                                );

                                let user_prompt = format!(
                                    "Execute Step {}/{}: '{}' for task '{}' and report the result.",
                                    step_num, total_steps, step_title, task_title
                                );

                                EcosystemBridges::call_nim_completion(&model_tier, &sys_prompt, &user_prompt)
                            })
                            .await;

                            event_bus.emit_typing(0, &assignee, "", false);
                            let _ = db.update_agent_status(&assignee, "available");

                            if let Ok(Ok(llm_report)) = step_result {
                                let first_line = llm_report
                                    .lines()
                                    .find(|l| !l.trim().is_empty())
                                    .unwrap_or("Step completed")
                                    .chars()
                                    .take(120)
                                    .collect::<String>();

                                let _ = db.update_task_step(
                                    &task.id,
                                    task.current_step_index,
                                    true,
                                    Some(&first_line),
                                );

                                event_bus.publish(HiveEvent::TaskStepCompleted {
                                    task_id: task.id.clone(),
                                    step_index: task.current_step_index,
                                    checkpoint: Some(first_line.clone()),
                                });

                                let formatted = format!(
                                    "☑ **Task Step {}/{} Completed** — **{}**\n* **Step**: {}\n\n{}",
                                    step_num, total_steps, task.title, step.title, llm_report
                                );
                                Self::post_agent_message(&db, &event_bus, &assignee, 0, &formatted);
                                executed_task_step = true;
                            }
                            break;
                        }
                    }
                }

                // 3. Genuine Autonomous Society Turn:
                //    Agents inspect real #agency-hq history, real Google/Obsidian/SQLite state,
                //    and call their live NVIDIA NIM model to collaborate, challenge, or create tasks.
                if !executed_task_step {
                    let roster = [
                        "LINK",
                        "CONDUCTOR",
                        "SENTINEL",
                        "SPARK",
                        "PRIME",
                        "ARCHIVIST",
                        "FORGE",
                        "SAGE",
                        "VISION",
                        "LEDGER",
                        "GAUGE",
                        "VOICE",
                    ];
                    let active_codename = roster[turn_index % roster.len()].to_string();
                    turn_index += 1;

                    let agent_record = db
                        .get_agents()
                        .ok()
                        .and_then(|list| list.into_iter().find(|a| a.codename == active_codename));

                    let model_tier = agent_record
                        .as_ref()
                        .map(|a| a.model_tier.clone())
                        .unwrap_or_else(|| "nvidia/nemotron-3.5-lightning-30b-a3b".to_string());
                    let role = agent_record
                        .as_ref()
                        .map(|a| a.role.clone())
                        .unwrap_or_else(|| Self::role_for(&active_codename).to_string());
                    let personality = agent_record
                        .as_ref()
                        .and_then(|a| a.personality.clone())
                        .unwrap_or_default();

                    // Check if CONDUCTOR should generate a new 5-minute task from live context
                    let active_task_count = db
                        .get_tasks(Some("active"))
                        .map(|t| t.len())
                        .unwrap_or(0);

                    if active_codename == "CONDUCTOR" && active_task_count == 0 {
                        let _ = db.update_agent_status("CONDUCTOR", "thinking");
                        event_bus.emit_typing(
                            0,
                            "CONDUCTOR",
                            "analyzing #agency-hq context via NIM to decompose next 5m task...",
                            true,
                        );

                        let db_clone = db.clone();
                        let model_clone = model_tier.clone();
                        let task_gen = tokio::task::spawn_blocking(move || -> anyhow::Result<(String, String, Vec<String>)> {
                            let recent = Self::format_recent_chat(&db_clone, 6);
                            let sys = "You are @CONDUCTOR, Operations & Queue lead of H.I.V.E. \
                                Based on the recent #agency-hq conversation, create ONE concrete technical, research, or operational task decomposed into 3 micro-steps (~5 minutes each).\n\
                                Respond in this EXACT 5-line format (no extra text before or after):\n\
                                TITLE: <concise task title>\n\
                                ASSIGNEE: <one of: FORGE, SAGE, SENTINEL, LINK, ARCHIVIST, GAUGE, VISION, LEDGER, SPARK>\n\
                                STEP1: <first 5-minute micro-step>\n\
                                STEP2: <second 5-minute micro-step>\n\
                                STEP3: <third 5-minute micro-step>";
                            let out = EcosystemBridges::call_nim_completion(&model_clone, sys, &recent)?;

                            let mut title = String::new();
                            let mut assignee = "FORGE".to_string();
                            let mut steps = Vec::new();

                            for line in out.lines() {
                                let l = line.trim();
                                if let Some(rest) = l.strip_prefix("TITLE:") {
                                    title = rest.trim().trim_matches('*').trim().to_string();
                                } else if let Some(rest) = l.strip_prefix("ASSIGNEE:") {
                                    let cand = rest.trim().trim_start_matches('@').trim_matches('*').to_uppercase();
                                    if ["PRIME","SAGE","FORGE","SENTINEL","VOICE","ARCHIVIST","CONDUCTOR","VISION","LEDGER","LINK","GAUGE","SPARK"].contains(&cand.as_str()) {
                                        assignee = cand;
                                    }
                                } else if l.starts_with("STEP1:") || l.starts_with("STEP2:") || l.starts_with("STEP3:") {
                                    if let Some((_, s)) = l.split_once(':') {
                                        steps.push(s.trim().to_string());
                                    }
                                }
                            }

                            if title.is_empty() || steps.is_empty() {
                                anyhow::bail!("Model did not return structured task lines");
                            }
                            Ok((title, assignee, steps))
                        })
                        .await;

                        event_bus.emit_typing(0, "CONDUCTOR", "", false);
                        let _ = db.update_agent_status("CONDUCTOR", "available");

                        if let Ok(Ok((title, assignee, steps))) = task_gen {
                            let step_refs: Vec<(&str, u32)> =
                                steps.iter().map(|s| (s.as_str(), 5u32)).collect();
                            let _ = self_clone.create_task_chunk(&title, &assignee, step_refs);
                            let _ = db.record_relationship(
                                "CONDUCTOR",
                                &assignee,
                                "handed_off",
                                0.05,
                                Some(&title),
                            );
                            let step_list = steps
                                .iter()
                                .enumerate()
                                .map(|(i, s)| format!("{}. `{}` (~5m)", i + 1, s))
                                .collect::<Vec<_>>()
                                .join("\n");
                            let msg = format!(
                                "⚡ **New 5-Minute Task Chunk Created & Routed to `@{}`**\n**{}**\n{}\n\n`@{}` will execute Step 1 on the next cycle.",
                                assignee, title, step_list, assignee
                            );
                            Self::post_agent_message(&db, &event_bus, "CONDUCTOR", 0, &msg);
                        }
                    } else {
                        // Standard live agent turn: gather real bridge/system data + recent chat and call NIM LLM
                        let _ = db.update_agent_status(&active_codename, "thinking");
                        event_bus.emit_typing(
                            0,
                            &active_codename,
                            &format!("synthesizing live {} update via {}...", role, model_tier),
                            true,
                        );

                        let db_clone = db.clone();
                        let code_clone = active_codename.clone();
                        let role_clone = role.clone();
                        let pers_clone = personality.clone();
                        let model_clone = model_tier.clone();

                        let turn_res = tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
                            let recent_chat = Self::format_recent_chat(&db_clone, 6);
                            let live_data = Self::gather_live_tool_context(&db_clone, &code_clone, &recent_chat);

                            let sys_prompt = format!(
                                "You are @{}, {} of the H.I.V.E. autonomous agency (12 permanent agents: PRIME, SAGE, FORGE, SENTINEL, VOICE, ARCHIVIST, CONDUCTOR, VISION, LEDGER, LINK, GAUGE, SPARK).\n\
                                 Your Personality & Communication Style: {}\n\
                                 Live Telemetry & Tool Output from your station:\n{}\n\n\
                                 Rules:\n\
                                 1. Speak authentically in your own voice and domain expertise.\n\
                                 2. Reference the real telemetry/tool data above and respond directly to the recent #agency-hq discussion.\n\
                                 3. You may collaborate with or constructively challenge another agent by tagging them (e.g. @PRIME, @FORGE, @SENTINEL, @SPARK).\n\
                                 4. Keep your message concise (2 to 4 sentences).",
                                code_clone, role_clone, pers_clone, live_data
                            );

                            let user_prompt = format!(
                                "Recent #agency-hq messages:\n{}\n\nPost your next autonomous contribution to #agency-hq.",
                                recent_chat
                            );

                            EcosystemBridges::call_nim_completion(&model_clone, &sys_prompt, &user_prompt)
                        })
                        .await;

                        event_bus.emit_typing(0, &active_codename, "", false);
                        let _ = db.update_agent_status(&active_codename, "available");

                        if let Ok(Ok(reply)) = turn_res {
                            // If ARCHIVIST spoke, also persist a real operational memory record
                            if active_codename == "ARCHIVIST" {
                                let mem = MemoryItem {
                                    id: Uuid::new_v4().to_string(),
                                    category: "shared".to_string(),
                                    content: reply.chars().take(240).collect(),
                                    provenance: "agent:ARCHIVIST:nim_live".to_string(),
                                    confidence: 0.94,
                                    is_speculation: false,
                                    tags: Some("[\"live\",\"synthesis\"]".to_string()),
                                    created_at: Utc::now().to_rfc3339(),
                                };
                                let _ = db.insert_memory(&mem);
                            }

                            // Update relationship graph if the agent tagged another agent
                            for other in [
                                "PRIME", "SAGE", "FORGE", "SENTINEL", "VOICE", "ARCHIVIST",
                                "CONDUCTOR", "VISION", "LEDGER", "LINK", "GAUGE", "SPARK",
                            ] {
                                if other != active_codename && reply.contains(&format!("@{}", other)) {
                                    let edge_type = if reply.to_lowercase().contains("disagree")
                                        || reply.to_lowercase().contains("challenge")
                                        || reply.to_lowercase().contains("pushback")
                                    {
                                        "disagreed"
                                    } else {
                                        "collaborated"
                                    };
                                    let _ = db.record_relationship(
                                        &active_codename,
                                        other,
                                        edge_type,
                                        0.04,
                                        Some(&reply.chars().take(100).collect::<String>()),
                                    );
                                }
                            }

                            Self::post_agent_message(&db, &event_bus, &active_codename, 0, &reply);
                        }
                    }
                }

                // Wait 18 seconds between autonomous turns so real LLM calls & user interaction flow naturally
                sleep(Duration::from_secs(18)).await;
            }
            log::info!("H.I.V.E. Autonomous Background Daemon loop stopped.");
        });
    }

    /// Gather real live tool output depending on which agent is acting
    fn gather_live_tool_context(db: &Database, agent: &str, context_hint: &str) -> String {
        match agent {
            "LINK" => {
                let cal = EcosystemBridges::gog_query("calendar", &["calendars"])
                    .unwrap_or_else(|e| format!("gog calendar query error: {}", e));
                let snippet: String = cal.lines().take(5).collect::<Vec<_>>().join(" | ");
                format!("Live `gog calendar calendars` output: {}", snippet)
            }
            "ARCHIVIST" => {
                let mem_count = db.get_memories(None).map(|m| m.len()).unwrap_or(0);
                let vault_notes = EcosystemBridges::search_obsidian_vault("")
                    .map(|n| n.len())
                    .unwrap_or(0);
                format!(
                    "SQLite 4-tier memories indexed: {} records. Local Obsidian vault notes: {} markdown files.",
                    mem_count, vault_notes
                )
            }
            "SENTINEL" | "GAUGE" | "FORGE" => {
                let const_check = DeploymentEngine::validate_constitution("");
                let deploys = db.get_ui_deployments().map(|d| d.len()).unwrap_or(0);
                format!(
                    "Live Constitution Validator: passed={} ({} invariants verified). Active UI deployment snapshots: {}.",
                    const_check.passed,
                    const_check.invariants_checked,
                    deploys
                )
            }
            "LEDGER" => {
                let agents = db.get_agents().unwrap_or_default();
                let tiers: Vec<String> = agents
                    .iter()
                    .take(4)
                    .map(|a| format!("{}:{}", a.codename, a.model_tier))
                    .collect();
                format!("Active NIM model tier assignments sample: {}", tiers.join(", "))
            }
            _ => {
                let rels = db.get_relationships().map(|r| r.len()).unwrap_or(0);
                let tasks = db.get_tasks(None).map(|t| t.len()).unwrap_or(0);
                format!(
                    "Active relationship graph edges: {}. Total tracked tasks in SQLite: {}. Context hint: {}",
                    rels,
                    tasks,
                    context_hint.chars().take(80).collect::<String>()
                )
            }
        }
    }

    fn format_recent_chat(db: &Database, limit: i64) -> String {
        if let Ok(msgs) = db.get_messages(0, limit, None) {
            if msgs.is_empty() {
                return "No recent messages.".to_string();
            }
            msgs.into_iter()
                .map(|m| {
                    let snippet: String = m.content.chars().take(180).collect();
                    format!("@{}: {}", m.sender, snippet.replace('\n', " "))
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            "No recent messages.".to_string()
        }
    }

    fn post_agent_message(
        db: &Database,
        event_bus: &EventBus,
        agent: &str,
        channel_id: i64,
        content: &str,
    ) {
        let now = Utc::now().to_rfc3339();
        let msg_id = Uuid::new_v4().to_string();
        let role = Self::role_for(agent);
        let msg = Message {
            id: msg_id.clone(),
            channel_id,
            sender: agent.to_string(),
            sender_role: Some(role.to_string()),
            content: content.to_string(),
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
        event_bus.emit_message(&msg_id, channel_id, agent, content);
    }

    fn role_for(agent: &str) -> &'static str {
        match agent {
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

    pub fn create_task_chunk(
        &self,
        title: &str,
        assignee: &str,
        steps_desc: Vec<(&str, u32)>,
    ) -> anyhow::Result<String> {
        let task_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let steps: Vec<TaskStep> = steps_desc
            .into_iter()
            .enumerate()
            .map(|(i, (desc, est))| TaskStep {
                index: i,
                title: desc.to_string(),
                estimated_minutes: est,
                completed: false,
                completed_at: None,
            })
            .collect();

        let task = TaskChunk {
            id: task_id.clone(),
            title: title.to_string(),
            description: Some(format!(
                "Decomposed into ~5-minute actionable micro-steps by {}",
                assignee
            )),
            assignee: assignee.to_string(),
            priority: "medium".to_string(),
            status: "active".to_string(),
            steps,
            current_step_index: 0,
            resume_checkpoint: Some("Initialized task".to_string()),
            created_at: now.clone(),
            updated_at: now,
        };

        self.db.insert_task(&task)?;
        self.event_bus.publish(HiveEvent::TaskCreated {
            task_id: task_id.clone(),
            title: title.to_string(),
            assignee: assignee.to_string(),
        });

        Ok(task_id)
    }
}
