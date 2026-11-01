use crate::agents::{self, PromptContext};
use crate::bridges::EcosystemBridges;
use crate::db::models::{Agent, MemoryItem, TaskChunk, TaskStep};
use crate::db::Database;
use crate::events::{EventBus, HiveEvent};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

pub const DEFAULT_INTERVAL_SECS: u64 = 20;
pub const MAX_STEP_FAILURES: u32 = 3;
pub const OWNER_QUIET_WINDOW_SECS: i64 = 8;
pub const RELATIONSHIP_HALF_LIFE_DAYS: f64 = 30.0;

const SOCIETY_ROTATION: [&str; 12] = [
    "LINK", "CONDUCTOR", "SENTINEL", "SPARK", "PRIME", "ARCHIVIST", "FORGE", "SAGE", "VISION",
    "LEDGER", "GAUGE", "VOICE",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgencyState {
    Running,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnOutcome {
    Posted,
    Passed,
    Failed,
}

pub fn autonomous_delay(base_secs: u64, owner_idle_secs: i64, consecutive_failures: u32) -> Duration {
    let base = base_secs.clamp(5, 600);
    let idle_factor = if owner_idle_secs < 600 {
        1
    } else if owner_idle_secs < 3_600 {
        4
    } else {
        12
    };
    let failure_factor = 2u64.pow(consecutive_failures.min(5));
    Duration::from_secs((base * idle_factor * failure_factor).min(900))
}

fn now_secs() -> i64 {
    Utc::now().timestamp()
}

pub struct ReplyGuard {
    counter: Arc<AtomicUsize>,
}

impl Drop for ReplyGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Clone)]
pub struct DaemonManager {
    db: Database,
    event_bus: EventBus,
    is_running: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    last_owner_activity: Arc<AtomicI64>,
    replies_in_flight: Arc<AtomicUsize>,
}

impl DaemonManager {
    pub fn new(db: Database, event_bus: EventBus) -> Self {
        let paused = db
            .get_setting("agency_state")
            .ok()
            .flatten()
            .map(|s| s == "paused")
            .unwrap_or(false);
        Self {
            db,
            event_bus,
            is_running: Arc::new(AtomicBool::new(true)),
            is_paused: Arc::new(AtomicBool::new(paused)),
            last_owner_activity: Arc::new(AtomicI64::new(now_secs())),
            replies_in_flight: Arc::new(AtomicUsize::new(0)),
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

    fn transition(&self, state: &str, audit: &str) {
        let _ = self.db.set_setting("agency_state", state);
        self.event_bus.publish(HiveEvent::SystemLifecycle {
            state: state.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        });
        let _ = self.db.log_audit("agency_state_change", "USER", audit, "SUCCESS");
    }

    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
        self.transition("paused", "Agency state changed to PAUSED");
    }

    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
        self.transition("running", "Agency state changed to RUNNING");
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        self.transition("stopped", "Agency state changed to STOPPED");
    }

    pub fn note_owner_activity(&self) {
        self.last_owner_activity.store(now_secs(), Ordering::SeqCst);
    }

    pub fn owner_idle_secs(&self) -> i64 {
        (now_secs() - self.last_owner_activity.load(Ordering::SeqCst)).max(0)
    }

    pub fn begin_reply(&self) -> ReplyGuard {
        self.replies_in_flight.fetch_add(1, Ordering::SeqCst);
        ReplyGuard {
            counter: self.replies_in_flight.clone(),
        }
    }

    fn owner_has_the_floor(&self) -> bool {
        self.replies_in_flight.load(Ordering::SeqCst) > 0
            || self.owner_idle_secs() < OWNER_QUIET_WINDOW_SECS
    }

    fn base_interval(&self) -> u64 {
        self.db
            .get_setting("daemon_interval_secs")
            .ok()
            .flatten()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(DEFAULT_INTERVAL_SECS)
    }

    fn load_agent(&self, codename: &str) -> Agent {
        self.db
            .get_agent(codename)
            .ok()
            .flatten()
            .unwrap_or_else(|| Agent {
                codename: codename.to_string(),
                display_name: codename.to_string(),
                role: agents::role_for(codename).to_string(),
                model_tier: "nvidia/nemotron-3.5-lightning-30b-a3b".to_string(),
                status: "available".to_string(),
                avatar_config: "{}".to_string(),
                personality: None,
                skills: None,
                tools: None,
                permissions: None,
                updated_at: Utc::now().to_rfc3339(),
            })
    }

    pub fn start_autonomous_loop(&self) {
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            log::info!("H.I.V.E. autonomous daemon started.");
            sleep(Duration::from_secs(3)).await;

            let mut turn_index: usize = 0;
            let mut consecutive_failures: u32 = 0;
            let mut step_failures: HashMap<String, u32> = HashMap::new();
            let mut last_decay: Option<Instant> = None;

            while this.is_running.load(Ordering::SeqCst) {
                if this.is_paused.load(Ordering::SeqCst) || this.owner_has_the_floor() {
                    sleep(Duration::from_secs(3)).await;
                    continue;
                }

                if !EcosystemBridges::llm_configured() {
                    sleep(Duration::from_secs(60)).await;
                    continue;
                }

                let _ = this.db.retire_expired_specialists();

                if last_decay.map(|t| t.elapsed() > Duration::from_secs(3_600)).unwrap_or(true) {
                    let _ = this.db.decay_relationships(RELATIONSHIP_HALF_LIFE_DAYS);
                    last_decay = Some(Instant::now());
                }

                let outcome = match this.run_task_step(&mut step_failures).await {
                    Some(o) => o,
                    None => {
                        let code = SOCIETY_ROTATION[turn_index % SOCIETY_ROTATION.len()];
                        turn_index += 1;
                        this.run_society_turn(code).await
                    }
                };

                consecutive_failures = match outcome {
                    TurnOutcome::Failed => consecutive_failures.saturating_add(1),
                    _ => 0,
                };

                let delay = if outcome == TurnOutcome::Passed {
                    Duration::from_secs(4)
                } else {
                    autonomous_delay(this.base_interval(), this.owner_idle_secs(), consecutive_failures)
                };
                sleep(delay).await;
            }
            log::info!("H.I.V.E. autonomous daemon stopped.");
        });
    }

    async fn run_task_step(&self, step_failures: &mut HashMap<String, u32>) -> Option<TurnOutcome> {
        let task = self
            .db
            .get_tasks(Some("active"))
            .ok()?
            .into_iter()
            .find(|t| t.current_step_index < t.steps.len())?;

        let step_index = task.current_step_index;
        let step = task.steps[step_index].clone();
        let step_num = step_index + 1;
        let total = task.steps.len();
        let agent = self.load_agent(&task.assignee);
        let code = agent.codename.clone();

        let _ = self.db.update_agent_status(&code, "thinking");
        self.event_bus.emit_typing(
            0,
            &code,
            &format!("working on step {}/{}: {}...", step_num, total, step.title),
            true,
        );

        let db = self.db.clone();
        let agent_work = agent.clone();
        let task_title = task.title.clone();
        let step_title = step.title.clone();
        let result = tokio::task::spawn_blocking(move || {
            let live = agents::live_context(&db, &agent_work.codename, &step_title);
            let memories = agents::memory_lines(&db, &agent_work.codename, 4);
            let instructions = format!(
                "You're working on step {}/{} of the task \"{}\": \"{}\".\n\
                 Write a quick update for #agency-hq about what you did and found for this step, based on the live data above. \
                 2-4 sentences, casual but specific. If the live data doesn't actually let you finish the step, say what's blocking you instead of pretending it's done.",
                step_num, total, task_title, step_title
            );
            let prompt = agents::build_system_prompt(
                &agent_work,
                &PromptContext {
                    setting: "#agency-hq, the team group chat.",
                    live_data: Some(&live),
                    memories: &memories,
                    instructions: &instructions,
                },
            );
            EcosystemBridges::call_nim_completion(
                &agent_work.model_tier,
                &prompt,
                &format!("Post your update for step {}/{}.", step_num, total),
            )
        })
        .await;

        self.event_bus.emit_typing(0, &code, "", false);
        let _ = self.db.update_agent_status(&code, "available");

        match result {
            Ok(Ok(report)) => {
                step_failures.remove(&task.id);
                let report = agents::clean_reply(&code, &report);
                let checkpoint: String = report
                    .lines()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("Step completed")
                    .chars()
                    .take(120)
                    .collect();
                let _ = self
                    .db
                    .update_task_step(&task.id, step_index, true, Some(&checkpoint));
                self.event_bus.publish(HiveEvent::TaskStepCompleted {
                    task_id: task.id.clone(),
                    step_index,
                    checkpoint: Some(checkpoint),
                });
                let content = format!(
                    "☑ Step {}/{} — **{}**\n_{}_\n\n{}",
                    step_num, total, task.title, step.title, report
                );
                agents::post_message(&self.db, &self.event_bus, 0, &code, &agent.role, &content, false);
                Some(TurnOutcome::Posted)
            }
            Ok(Err(e)) => Some(self.handle_step_failure(&task, &step, &agent, step_failures, &e.to_string())),
            Err(e) => Some(self.handle_step_failure(&task, &step, &agent, step_failures, &e.to_string())),
        }
    }

    fn handle_step_failure(
        &self,
        task: &TaskChunk,
        step: &TaskStep,
        agent: &Agent,
        step_failures: &mut HashMap<String, u32>,
        err: &str,
    ) -> TurnOutcome {
        let count = step_failures.entry(task.id.clone()).or_insert(0);
        *count += 1;
        if *count < MAX_STEP_FAILURES {
            return TurnOutcome::Failed;
        }
        step_failures.remove(&task.id);
        let short_err: String = err.chars().take(180).collect();
        let _ = self.db.set_task_status(
            &task.id,
            "failed",
            Some(&format!("Parked after {} failed attempts: {}", MAX_STEP_FAILURES, short_err)),
        );
        let _ = self.db.log_audit(
            "task_parked",
            &agent.codename,
            &format!("Task '{}' parked at step '{}': {}", task.title, step.title, short_err),
            "FAILED",
        );
        let content = format!(
            "Heads up, I'm stuck on **{}** (step: _{}_). My model call failed {} times in a row ({}), so I've parked it as failed instead of clogging the queue. Retry it with `/task` once things are back up.",
            task.title, step.title, MAX_STEP_FAILURES, short_err
        );
        agents::post_message(&self.db, &self.event_bus, 0, &agent.codename, &agent.role, &content, false);
        TurnOutcome::Failed
    }

    async fn run_society_turn(&self, codename: &str) -> TurnOutcome {
        let history = self.db.get_messages(0, 12, None).unwrap_or_default();
        if history.last().map(|m| m.sender == codename).unwrap_or(false) {
            return TurnOutcome::Passed;
        }

        let active_tasks = self.db.get_tasks(Some("active")).map(|t| t.len()).unwrap_or(0);
        if codename == "CONDUCTOR" && active_tasks == 0 {
            return self.plan_next_task().await;
        }

        let agent = self.load_agent(codename);
        let code = agent.codename.clone();
        let _ = self.db.update_agent_status(&code, "thinking");
        self.event_bus.emit_typing(0, &code, agents::activity_for(&code), true);

        let db = self.db.clone();
        let agent_work = agent.clone();
        let result = tokio::task::spawn_blocking(move || {
            let recent_hint: String = history
                .iter()
                .rev()
                .take(3)
                .map(|m| m.content.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let live = agents::live_context(&db, &agent_work.codename, &recent_hint);
            let memories = agents::memory_lines(&db, &agent_work.codename, 4);
            let instructions = format!(
                "Nobody asked you anything directly, this is just a quiet moment in the group chat. \
                 Only chime in if you have something genuinely useful, interesting, or fun to add: react to what someone said, flag a problem, share something from your live data, or ask a teammate a question (tag them with @CODENAME). \
                 Keep it to 1-3 sentences and don't repeat what's already been said. \
                 If you've got nothing worth saying right now, reply with exactly {}.",
                agents::PASS_TOKEN
            );
            let prompt = agents::build_system_prompt(
                &agent_work,
                &PromptContext {
                    setting: "#agency-hq, the team group chat with all 12 agents and the Owner.",
                    live_data: Some(&live),
                    memories: &memories,
                    instructions: &instructions,
                },
            );
            let mut turns = agents::transcript_turns(&agent_work.codename, &history, 400);
            agents::push_user_turn(
                &mut turns,
                &format!("(Quiet moment. Post your next message to #agency-hq, or {}.)", agents::PASS_TOKEN),
            );
            EcosystemBridges::call_nim_chat(&agent_work.model_tier, &prompt, &turns)
        })
        .await;

        self.event_bus.emit_typing(0, &code, "", false);
        let _ = self.db.update_agent_status(&code, "available");

        let reply = match result {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                log::warn!("Society turn for {} failed: {}", code, e);
                return TurnOutcome::Failed;
            }
            Err(e) => {
                log::warn!("Society turn for {} crashed: {}", code, e);
                return TurnOutcome::Failed;
            }
        };

        if agents::is_pass(&reply) {
            return TurnOutcome::Passed;
        }
        let reply = agents::clean_reply(&code, &reply);
        if reply.is_empty() {
            return TurnOutcome::Passed;
        }

        if code == "ARCHIVIST" {
            let _ = self.db.insert_memory(&MemoryItem {
                id: Uuid::new_v4().to_string(),
                category: "operational".to_string(),
                content: reply.chars().take(240).collect(),
                provenance: "agent:ARCHIVIST:hq_observation".to_string(),
                confidence: 0.9,
                is_speculation: false,
                tags: Some("[\"hq\",\"observation\"]".to_string()),
                created_at: Utc::now().to_rfc3339(),
            });
        }

        agents::record_mentions(&self.db, &code, &reply);
        agents::post_message(&self.db, &self.event_bus, 0, &code, &agent.role, &reply, false);
        TurnOutcome::Posted
    }

    async fn plan_next_task(&self) -> TurnOutcome {
        let agent = self.load_agent("CONDUCTOR");
        let _ = self.db.update_agent_status("CONDUCTOR", "thinking");
        self.event_bus
            .emit_typing(0, "CONDUCTOR", "figuring out what we should tackle next...", true);

        let db = self.db.clone();
        let agent_work = agent.clone();
        let result = tokio::task::spawn_blocking(move || {
            let history = db.get_messages(0, 8, None).unwrap_or_default();
            let transcript = history
                .iter()
                .filter(|m| !m.is_deleted)
                .map(|m| {
                    let who = if m.sender == "user" { "Owner".to_string() } else { format!("@{}", m.sender) };
                    let snippet: String = m.content.chars().take(220).collect();
                    format!("{}: {}", who, snippet.replace('\n', " "))
                })
                .collect::<Vec<_>>()
                .join("\n");
            let instructions = "The queue is empty. Based on the recent chat, pick ONE concrete, useful task for the team and break it into 3 small steps (about 5 minutes each).\n\
                 Reply in exactly this format and nothing else:\n\
                 TITLE: <short task title>\n\
                 ASSIGNEE: <one of FORGE, SAGE, SENTINEL, LINK, ARCHIVIST, GAUGE, VISION, LEDGER, SPARK, VOICE>\n\
                 STEP1: <first step>\n\
                 STEP2: <second step>\n\
                 STEP3: <third step>";
            let prompt = agents::build_system_prompt(
                &agent_work,
                &PromptContext {
                    setting: "planning the team's next task.",
                    live_data: None,
                    memories: &[],
                    instructions,
                },
            );
            let user = if transcript.is_empty() { "No recent messages.".to_string() } else { transcript };
            let out = EcosystemBridges::call_nim_completion(&agent_work.model_tier, &prompt, &user)?;
            agents::parse_task_plan(&out, "FORGE")
                .ok_or_else(|| anyhow::anyhow!("model didn't return a usable task plan"))
        })
        .await;

        self.event_bus.emit_typing(0, "CONDUCTOR", "", false);
        let _ = self.db.update_agent_status("CONDUCTOR", "available");

        let plan = match result {
            Ok(Ok(p)) => p,
            _ => return TurnOutcome::Failed,
        };

        let step_refs: Vec<(&str, u32)> = plan.steps.iter().map(|s| (s.as_str(), 5u32)).collect();
        if self.create_task_chunk(&plan.title, &plan.assignee, step_refs).is_err() {
            return TurnOutcome::Failed;
        }
        let _ = self
            .db
            .record_relationship("CONDUCTOR", &plan.assignee, "handed_off", 0.05, Some(&plan.title));
        let step_list = plan
            .steps
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}. {} (~5m)", i + 1, s))
            .collect::<Vec<_>>()
            .join("\n");
        let content = format!(
            "Queue was empty, so here's the next one: **{}**\n{}\n\n@{} you're up, I'll check in after step 1.",
            plan.title, step_list, plan.assignee
        );
        agents::post_message(&self.db, &self.event_bus, 0, "CONDUCTOR", &agent.role, &content, false);
        TurnOutcome::Posted
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
