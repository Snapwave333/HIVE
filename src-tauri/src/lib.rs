pub mod agents;
pub mod bridges;
pub mod commands;
pub mod daemon;
pub mod db;
pub mod deployment;
pub mod events;
pub mod governance;
pub mod tray;

use agents::PromptContext;
use anyhow::Result;
use bridges::EcosystemBridges;
use daemon::DaemonManager;
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

const HELP_TEXT: &str = "Here's what I can do from the chat box:\n\n\
* `/task <goal>` — @CONDUCTOR splits it into ~5-minute steps and queues it\n\
* `/gog <args>` — run the Google CLI, e.g. `/gog calendar calendars` or `/gog drive ls` (@LINK)\n\
* `/obsidian <title> | <note>` — save a note to the Obsidian vault, or `/obsidian <words>` to search it (@ARCHIVIST)\n\
* `/browser <status|tabs>` — query the OpenCode Chrome relay (@LINK)\n\
* `/debate <proposal>` — the council (PRIME, SENTINEL, FORGE, LEDGER, SPARK) votes on it in parallel\n\
* `/spawn_temp <name>` — spin up a `temp_` specialist with a 30-minute TTL\n\
* `/deploy <version> <summary>` — snapshot the running UI build as a versioned, hashed bundle (@FORGE)\n\
* `/rollback` — switch the UI back to the previous verified snapshot (@SENTINEL)\n\
* `/backup` — take a SQLite snapshot right now\n\
* `/pause` | `/resume` — stop or restart background chatter and task work\n\n\
Or just talk. Tag people like @FORGE or @SAGE in #agency-hq (up to 3 will answer), or DM anyone directly.";

impl AppState {
    pub fn new() -> Result<Self> {
        let db = Database::new(None)?;
        let events = EventBus::new(512);
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

    pub fn execute_agency_activation_directive(&self, directive_text: String, post_user_msg: bool) {
        let db = self.db.clone();
        let events = self.events.clone();
        let daemon = self.daemon.clone();
        let guard = self.daemon.begin_reply();

        tauri::async_runtime::spawn(async move {
            let _guard = guard;
            let _ = db.set_setting("activation_directive_executed", "true");

            if post_user_msg {
                agents::post_message(&db, &events, 0, "user", "Owner", &directive_text, true);
            }

            let focus: [(&str, i64, &str); 12] = [
                ("PRIME", 1, "agency direction, coordinating the team, and being the Owner's main point of contact"),
                ("SAGE", 2, "deep research, competitive intel, and the Research & Intelligence project"),
                ("FORGE", 3, "Rust/Tauri/React engineering, CI/CD, and the Infrastructure project"),
                ("SENTINEL", 4, "threat modeling, permission audits, and enforcing the Constitution"),
                ("VOICE", 5, "owner briefings, docs, and keeping updates readable"),
                ("ARCHIVIST", 6, "memory indexing, provenance, and the Obsidian knowledge vault"),
                ("CONDUCTOR", 7, "5-minute task chunking, queue routing, and the Automation project"),
                ("VISION", 8, "UI/UX, design tokens, and the Creative Development project"),
                ("LEDGER", 9, "API cost tracking, model-tier spend, and the Financial Intelligence project"),
                ("LINK", 10, "Google gog CLI, Obsidian, the Chrome relay, and the Personal Operations project"),
                ("GAUGE", 11, "testing, regression checks, and the Autonomous Development project"),
                ("SPARK", 12, "R&D experiments, prototypes, and the Innovation project"),
            ];

            let mut checked_in: Vec<String> = Vec::new();
            let mut failed: Vec<(String, String)> = Vec::new();

            for (codename, dm_channel_id, focus_summary) in focus {
                let Some(agent) = db.get_agent(codename).ok().flatten() else {
                    failed.push((codename.to_string(), "missing from the agent registry".to_string()));
                    continue;
                };
                let _ = db.update_agent_status(codename, "thinking");
                events.emit_typing(0, codename, "checking in...", true);

                let db_work = db.clone();
                let agent_work = agent.clone();
                let focus_str = focus_summary.to_string();
                let intro_res = tokio::task::spawn_blocking(move || {
                    let telemetry = match agent_work.codename.as_str() {
                        "LINK" => match EcosystemBridges::check_google_auth() {
                            Ok(out) => format!(
                                "gog CLI auth check succeeded: {}",
                                out.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("(no account listed)")
                            ),
                            Err(e) => format!("gog CLI auth check failed: {}", e),
                        },
                        "SENTINEL" | "GAUGE" | "FORGE" => {
                            let rep = DeploymentEngine::validate_constitution("");
                            format!(
                                "Constitution v{} check: passed={} ({} invariants)",
                                rep.version, rep.passed, rep.invariants_checked
                            )
                        }
                        "ARCHIVIST" => format!(
                            "Obsidian vault path: {}",
                            EcosystemBridges::resolve_vault_dir().to_string_lossy()
                        ),
                        _ => agents::live_context(&db_work, &agent_work.codename, ""),
                    };
                    let instructions = format!(
                        "The Owner just kicked off the agency and everyone's checking in on #agency-hq for the first time.\n\
                         Your focus: {}.\n\
                         Say hi to the team: who you are, what you'll dig into first, and whether your tools actually work based on the live data (be honest if something failed). 2-4 sentences, casual.",
                        focus_str
                    );
                    let prompt = agents::build_system_prompt(
                        &agent_work,
                        &PromptContext {
                            setting: "#agency-hq, the team group chat.",
                            live_data: Some(&telemetry),
                            memories: &[],
                            instructions: &instructions,
                        },
                    );
                    EcosystemBridges::call_nim_completion(&agent_work.model_tier, &prompt, "Check in with the team.")
                })
                .await;

                events.emit_typing(0, codename, "", false);
                let _ = db.update_agent_status(codename, "available");

                match intro_res {
                    Ok(Ok(text)) => {
                        let intro = agents::clean_reply(codename, &text);
                        agents::post_message(&db, &events, 0, codename, &agent.role, &intro, false);
                        agents::post_message(
                            &db,
                            &events,
                            dm_channel_id,
                            codename,
                            &agent.role,
                            &format!("⬡ **@{} direct line open** (`{}`)\n\n{}", codename, agent.model_tier, intro),
                            true,
                        );
                        let _ = db.insert_memory(&db::models::MemoryItem {
                            id: Uuid::new_v4().to_string(),
                            category: "individual".to_string(),
                            content: format!("My first check-in at activation: {}", intro.chars().take(220).collect::<String>()),
                            provenance: format!("agent:{}:initial_assembly", codename),
                            confidence: 1.0,
                            is_speculation: false,
                            tags: Some(format!("[\"assembly\",\"{}\"]", codename.to_lowercase())),
                            created_at: chrono::Utc::now().to_rfc3339(),
                        });
                        checked_in.push(codename.to_string());
                    }
                    Ok(Err(e)) => failed.push((codename.to_string(), e.to_string())),
                    Err(e) => failed.push((codename.to_string(), e.to_string())),
                }
            }

            let portfolio: [(&str, &str, [&str; 3]); 8] = [
                ("[1. Infrastructure] H.I.V.E. reliability, SQLite WAL & daemon telemetry", "FORGE", [
                    "Audit SQLite WAL checkpoint latency and daily backup integrity",
                    "Measure idle/active RAM and background task responsiveness",
                    "Verify tray recovery and document baseline metrics",
                ]),
                ("[2. Autonomous Development] Continuous build, test & blue/green verification", "GAUGE", [
                    "Run the Constitution invariant validator against the frontend bundle",
                    "Verify SHA-256 bundle snapshots and rollback targets",
                    "Record regression results in the audit log",
                ]),
                ("[3. Research & Intelligence] Open-source tools, models & integrations", "SAGE", [
                    "Catalog the NIM model tiers the agency actually uses",
                    "Evaluate high-leverage open-source CLI and MCP integrations",
                    "Publish a ranked synthesis to the Obsidian vault",
                ]),
                ("[4. Automation] Repetitive workflow & CLI bridge automation", "CONDUCTOR", [
                    "Identify frequent manual checks across Google, Obsidian, and git",
                    "Design safe, idempotent 5-minute routines within the allowlists",
                    "Schedule the routines and verify checkpoint logging",
                ]),
                ("[5. Financial Intelligence] Compute cost optimization & revenue research", "LEDGER", [
                    "Audit token usage across NIM model tiers",
                    "Research practical expense reductions and product ideas",
                    "Compile an owner-controlled brief (zero autonomous spend)",
                ]),
                ("[6. Creative Development] Design system & asset pipeline", "VISION", [
                    "Audit icon ergonomics, type scaling, and WCAG contrast",
                    "Design reusable visual templates and components",
                    "Stage UI polish in the preview sandbox",
                ]),
                ("[7. Personal Operations] Schedule, mail & notes via gog/Obsidian", "LINK", [
                    "Sync upcoming Google Calendar events and Tasks via gog",
                    "Organize daily notes in the Obsidian vault",
                    "Post a schedule and action digest to #agency-hq",
                ]),
                ("[8. Innovation] R&D experiments & prototype evaluation", "SPARK", [
                    "Form a low-overhead hypothesis for faster memory retrieval",
                    "Benchmark the prototype with @GAUGE verifying",
                    "Bring the findings to @PRIME and the council for a vote",
                ]),
            ];

            let existing_titles: Vec<String> = db
                .get_tasks(Some("active"))
                .unwrap_or_default()
                .into_iter()
                .map(|t| t.title)
                .collect();
            let mut created = 0usize;
            for (title, owner, steps) in portfolio {
                if existing_titles.iter().any(|t| t == title) {
                    created += 1;
                    continue;
                }
                let refs: Vec<(&str, u32)> = steps.iter().map(|s| (*s, 5u32)).collect();
                if daemon.create_task_chunk(title, owner, refs).is_ok() {
                    created += 1;
                    let _ = db.record_relationship("PRIME", owner, "handed_off", 0.05, Some(title));
                }
            }

            if checked_in.is_empty() {
                let reason = failed
                    .first()
                    .map(|(_, e)| e.chars().take(200).collect::<String>())
                    .unwrap_or_else(|| "unknown error".to_string());
                agents::post_message(
                    &db,
                    &events,
                    0,
                    "PRIME",
                    agents::role_for("PRIME"),
                    &format!(
                        "⚠ None of us could reach our models, so the check-ins didn't happen.\n\n**Reason:** {}\n\nThe {} portfolio projects are queued and waiting. Add a key in **Settings → Bridges** (it's verified with NVIDIA before saving), then type `/activate` to run the assembly again.",
                        reason, created
                    ),
                    true,
                );
                return;
            }

            let _ = db.update_agent_status("PRIME", "thinking");
            events.emit_typing(0, "PRIME", "writing up the first briefing with @VOICE...", true);

            let status = EcosystemBridges::get_status();
            let facts = format!(
                "Agents that checked in: {} of 12 ({}).\nAgents that failed to check in: {}.\nProjects created: {} of 8.\nGoogle gog CLI: {}.\nObsidian vault: {} ({}).\nLLM provider: {}.\nAutonomous loop state: {:?}.",
                checked_in.len(),
                if checked_in.is_empty() { "none".to_string() } else { checked_in.join(", ") },
                if failed.is_empty() {
                    "none".to_string()
                } else {
                    failed.iter().map(|(c, e)| format!("{} ({})", c, e.chars().take(80).collect::<String>())).collect::<Vec<_>>().join("; ")
                },
                created,
                if status.google_connected { format!("connected ({})", status.google_account) } else { "not connected".to_string() },
                if status.obsidian_available { "available" } else { "unavailable" },
                status.obsidian_vault_path,
                status.llm_provider,
                daemon.state(),
            );

            let facts_for_fallback = facts.lines().map(|l| format!("* {}", l)).collect::<Vec<_>>().join("\n");
            let prime = db.get_agent("PRIME").ok().flatten();
            let briefing_res = tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
                let instructions = "You and @VOICE are writing the first agency briefing for the Owner. Use ONLY the verified facts above: report what worked and what didn't, plainly. \
                     Cover: who checked in, tool/bridge status, the 8 projects and owners, and what happens next. Markdown is fine here, keep it skimmable.";
                let tier = prime.as_ref().map(|a| a.model_tier.clone()).unwrap_or_else(|| "nvidia/nemotron-3-ultra-550b-a55b".to_string());
                let prompt = match prime.as_ref() {
                    Some(agent) => agents::build_system_prompt(
                        agent,
                        &PromptContext {
                            setting: "#agency-hq, the team group chat.",
                            live_data: Some(&facts),
                            memories: &[],
                            instructions,
                        },
                    ),
                    None => format!("Verified facts:\n{}\n\n{}", facts, instructions),
                };
                let text = EcosystemBridges::call_nim_completion(&tier, &prompt, "Write the first briefing.")?;
                let _ = EcosystemBridges::write_obsidian_note("HIVE_First_Agency_Briefing", &text, Some("Briefings"));
                Ok(text)
            })
            .await;

            events.emit_typing(0, "PRIME", "", false);
            let _ = db.update_agent_status("PRIME", "available");

            let briefing = match briefing_res {
                Ok(Ok(text)) => format!("📋 **First agency briefing**\n\n{}", agents::clean_reply("PRIME", &text)),
                Ok(Err(e)) => format!(
                    "📋 **First agency briefing**\n\nI couldn't get my model to write this up ({}), so here are the raw facts:\n\n{}",
                    e, facts_for_fallback
                ),
                Err(e) => format!(
                    "📋 **First agency briefing**\n\nThe briefing task crashed ({}). Raw facts:\n\n{}",
                    e, facts_for_fallback
                ),
            };
            agents::post_message(&db, &events, 0, "PRIME", agents::role_for("PRIME"), &briefing, true);
        });
    }

    pub fn trigger_agent_reply(&self, channel_id: i64, user_msg: &str) {
        self.daemon.note_owner_activity();

        let lower_check = user_msg.to_lowercase();
        if lower_check.contains("agency activation directive")
            || lower_check.contains("convene the agency")
            || lower_check.trim() == "/activate"
        {
            self.execute_agency_activation_directive(user_msg.to_string(), false);
            return;
        }

        let state = self.clone();
        let guard = self.daemon.begin_reply();
        let user_msg = user_msg.trim().to_string();

        tauri::async_runtime::spawn(async move {
            let _guard = guard;
            match state.run_slash_command(channel_id, &user_msg).await {
                Some((code, role, reply)) => {
                    agents::post_message(&state.db, &state.events, channel_id, &code, &role, &reply, false);
                }
                None => {
                    agents::respond_to_owner(state.db.clone(), state.events.clone(), channel_id, user_msg).await;
                }
            }
            state.daemon.note_owner_activity();
        });
    }

    async fn run_slash_command(&self, channel_id: i64, user_msg: &str) -> Option<(String, String, String)> {
        let lower = user_msg.to_lowercase();
        if !lower.starts_with('/') {
            return None;
        }
        let (cmd, arg) = match user_msg.split_once(char::is_whitespace) {
            Some((c, a)) => (c.to_lowercase(), a.trim().to_string()),
            None => (lower.clone(), String::new()),
        };

        let lead = match cmd.as_str() {
            "/help" | "/pause" | "/resume" | "/spawn_temp" | "/debate" => "PRIME",
            "/backup" | "/obsidian" => "ARCHIVIST",
            "/task" => "CONDUCTOR",
            "/deploy" => "FORGE",
            "/rollback" => "SENTINEL",
            "/gog" | "/browser" => "LINK",
            _ => return None,
        };
        let role = |c: &str| agents::role_for(c).to_string();
        let say = |c: &str, text: String| Some((c.to_string(), role(c), text));

        let _ = self.db.update_agent_status(lead, "thinking");
        self.events.emit_typing(channel_id, lead, agents::activity_for(lead), true);

        let result = match cmd.as_str() {
            "/help" => say("PRIME", HELP_TEXT.to_string()),
            "/pause" => {
                self.daemon.pause();
                say("PRIME", "⏸ Paused. Background chatter and task work are on hold, but you can still DM anyone. `/resume` when you want us back.".to_string())
            }
            "/resume" => {
                self.daemon.resume();
                say("PRIME", "▶ We're back. Background work and the task queue are running again.".to_string())
            }
            "/backup" => match self.db.create_backup() {
                Ok(p) => say("ARCHIVIST", format!("☑ Snapshot saved:\n`{}`", p.to_string_lossy())),
                Err(e) => say("ARCHIVIST", format!("⚠ Backup failed: {}", e)),
            },
            "/task" if arg.is_empty() => say("CONDUCTOR", "Give me a goal, like `/task clean up the attachments folder`.".to_string()),
            "/task" => {
                let conductor = self.db.get_agent("CONDUCTOR").ok().flatten();
                let goal = arg.clone();
                let plan = tokio::task::spawn_blocking(move || -> anyhow::Result<agents::TaskPlan> {
                    let tier = conductor.as_ref().map(|a| a.model_tier.clone()).unwrap_or_else(|| "minimaxai/minimax-m3.1-flash".to_string());
                    let sys = "You are @CONDUCTOR, who runs the H.I.V.E. task queue. Break the Owner's goal into exactly 3 concrete steps of about 5 minutes each and pick who should own it.\n\
                               Reply in exactly this format and nothing else:\n\
                               TITLE: <short task title>\n\
                               ASSIGNEE: <one of PRIME, SAGE, FORGE, SENTINEL, VOICE, ARCHIVIST, CONDUCTOR, VISION, LEDGER, LINK, GAUGE, SPARK>\n\
                               STEP1: <step>\nSTEP2: <step>\nSTEP3: <step>";
                    let out = EcosystemBridges::call_nim_completion(&tier, sys, &goal)?;
                    if let Some(p) = agents::parse_task_plan(&out, "FORGE") {
                        return Ok(p);
                    }
                    let steps = agents::parse_step_lines(&out, 3);
                    if steps.is_empty() {
                        anyhow::bail!("the model didn't return any usable steps");
                    }
                    Ok(agents::TaskPlan { title: goal, assignee: "FORGE".to_string(), steps })
                })
                .await;
                match plan {
                    Ok(Ok(plan)) => {
                        let refs: Vec<(&str, u32)> = plan.steps.iter().map(|s| (s.as_str(), 5u32)).collect();
                        match self.daemon.create_task_chunk(&plan.title, &plan.assignee, refs) {
                            Ok(_) => {
                                let _ = self.db.record_relationship("CONDUCTOR", &plan.assignee, "handed_off", 0.04, Some(&plan.title));
                                let steps = plan.steps.iter().enumerate().map(|(i, s)| format!("{}. {} (~5m)", i + 1, s)).collect::<Vec<_>>().join("\n");
                                say("CONDUCTOR", format!("Got it, queued **{}**:\n\n{}\n\n@{} has it. Progress shows up here as each step lands.", plan.title, steps, plan.assignee))
                            }
                            Err(e) => say("CONDUCTOR", format!("⚠ Couldn't save that task: {}", e)),
                        }
                    }
                    Ok(Err(e)) => say("CONDUCTOR", format!("⚠ Couldn't break that down: {}", e)),
                    Err(e) => say("CONDUCTOR", format!("⚠ Task planning crashed: {}", e)),
                }
            }
            "/spawn_temp" if arg.is_empty() => say("PRIME", "Name it, like `/spawn_temp rust_profiler`.".to_string()),
            "/spawn_temp" => match self.db.spawn_temp_specialist(&arg, "Ephemeral Task Specialist", "PRIME", 30) {
                Ok(spec) => {
                    let _ = self.db.log_audit("spawn_temp_specialist", "PRIME", &format!("Spawned {} (expires {})", spec.codename, spec.expires_at), "ACTIVE");
                    say("PRIME", format!(
                        "Spun up `{}` under my sponsorship. Permissions are scoped to its assigned task, and it auto-retires at `{}`. It never quietly becomes permanent.",
                        spec.codename, spec.expires_at
                    ))
                }
                Err(e) => say("PRIME", format!("⚠ Couldn't spawn that specialist: {}", e)),
            },
            "/debate" if arg.is_empty() => say("PRIME", "What should we vote on? Try `/debate switch FORGE to a bigger model`.".to_string()),
            "/debate" => {
                let gov = self.governance.clone();
                let topic = arg.clone();
                let res = tokio::task::spawn_blocking(move || {
                    gov.propose_decision(&topic, "Level-2 council evaluation requested by the Owner", DecisionLevel::Significant, "PRIME")
                })
                .await;
                match res {
                    Ok(Ok(dec)) => {
                        let mut lines: Vec<String> = dec
                            .votes
                            .values()
                            .map(|v| {
                                let verdict = if v.abstained { "abstained" } else if v.approve { "approve" } else { "dissent" };
                                format!("* **@{}** ({}, weight {:.1}): {}", v.agent, verdict, v.weight, v.rationale)
                            })
                            .collect();
                        lines.sort();
                        say("PRIME", format!(
                            "Council vote on **{}** is in:\n\n{}\n\n**Result:** `{}`{}",
                            dec.title,
                            lines.join("\n"),
                            dec.status.to_uppercase(),
                            dec.dissent_summary.map(|d| format!("\n**Dissent:** {}", d)).unwrap_or_default()
                        ))
                    }
                    Ok(Err(e)) => say("PRIME", format!("⚠ The vote didn't go through: {}", e)),
                    Err(e) => say("PRIME", format!("⚠ The vote crashed: {}", e)),
                }
            }
            "/deploy" => {
                let mut parts = arg.splitn(2, ' ');
                let version = parts.next().filter(|v| !v.is_empty()).map(str::to_string)
                    .unwrap_or_else(|| format!("v{}-{}", env!("CARGO_PKG_VERSION"), chrono::Utc::now().format("%Y%m%d%H%M%S")));
                let summary = parts.next().unwrap_or("Snapshot of the running UI build").to_string();
                let engine = self.deployment.clone();
                let res = tokio::task::spawn_blocking(move || engine.stage_and_deploy_version(&version, "FORGE", &summary, true)).await;
                match res {
                    Ok(Ok(dep)) => say("FORGE", format!(
                        "Shipped `{}`. It's a real snapshot of the running bundle, hashed and checked against the Constitution.\n* **Path:** `{}`\n* **SHA-256:** `{}`\n* **Check:** {}\n\nIf anything looks off, `/rollback` switches back to the previous snapshot and reloads the UI.",
                        dep.version,
                        dep.directory_path,
                        &dep.sha256_hash[..16],
                        dep.verification_log.unwrap_or_default()
                    )),
                    Ok(Err(e)) => say("SENTINEL", format!("🛑 Deploy blocked: {}", e)),
                    Err(e) => say("SENTINEL", format!("⚠ Deploy crashed: {}", e)),
                }
            }
            "/rollback" => match self.deployment.rollback("Owner /rollback command") {
                Ok(dep) => say("SENTINEL", format!("⏪ Rolled back to `{}` (hash verified). The UI is reloading onto it now.", dep.version)),
                Err(e) => say("SENTINEL", format!("⚠ Can't roll back: {}", e)),
            },
            "/obsidian" => {
                if let Some((title, body)) = arg.split_once('|') {
                    match EcosystemBridges::write_obsidian_note(title.trim(), body.trim(), Some("HIVE")) {
                        Ok(path) => say("ARCHIVIST", format!("Saved it to the vault:\n`{}`", path.to_string_lossy())),
                        Err(e) => say("ARCHIVIST", format!("⚠ Couldn't write that note: {}", e)),
                    }
                } else {
                    let notes = EcosystemBridges::search_obsidian_vault(&arg).unwrap_or_default();
                    let vault = EcosystemBridges::resolve_vault_dir();
                    if notes.is_empty() {
                        say("ARCHIVIST", format!("Nothing in the vault (`{}`) matches that. Use `/obsidian Title | content` to add a note.", vault.to_string_lossy()))
                    } else {
                        let list = notes.iter().take(8).map(|(name, snip)| format!("* **{}** — {}", name, snip.chars().take(120).collect::<String>())).collect::<Vec<_>>().join("\n");
                        say("ARCHIVIST", format!("Found {} note(s) in `{}`:\n\n{}", notes.len(), vault.to_string_lossy(), list))
                    }
                }
            }
            "/browser" => {
                let sub = if arg.is_empty() { "status".to_string() } else { arg.clone() };
                let res = tokio::task::spawn_blocking(move || EcosystemBridges::browser_relay_exec(&sub, &[]).map(|o| (sub, o))).await;
                match res {
                    Ok(Ok((sub, out))) => say("LINK", format!("`browser-relay {}`:\n```text\n{}\n```", sub, out.trim())),
                    Ok(Err(e)) => say("LINK", format!("⚠ Browser relay said no: {}", e)),
                    Err(e) => say("LINK", format!("⚠ Browser relay crashed: {}", e)),
                }
            }
            "/gog" if arg.is_empty() => say("LINK", "Give me a gog command, like `/gog calendar calendars` or `/gog drive ls`.".to_string()),
            "/gog" => {
                let cmd_args = arg.clone();
                let res = tokio::task::spawn_blocking(move || {
                    let parts: Vec<&str> = cmd_args.split_whitespace().collect();
                    EcosystemBridges::gog_cli_exec(&parts)
                })
                .await;
                match res {
                    Ok(Ok(out)) => say("LINK", format!("`gog {}`:\n```json\n{}\n```", arg, out.trim())),
                    Ok(Err(e)) => say("LINK", format!("⚠ gog failed: {}", e)),
                    Err(e) => say("LINK", format!("⚠ gog crashed: {}", e)),
                }
            }
            _ => None,
        };

        self.events.emit_typing(channel_id, lead, "", false);
        let _ = self.db.update_agent_status(lead, "available");
        result
    }
}

pub const ACTIVATION_DIRECTIVE_PROMPT: &str = r##"H.I.V.E. — AGENCY ACTIVATION DIRECTIVE

PRIME: Initiate full agency operations.

The H.I.V.E. application is entering its first operational phase. Your responsibility is to transform the 12-agent coworker society into a fully functioning, autonomous, continuously improving AI agency.

1. INITIAL ASSEMBLY

Bring all 12 permanent agents online:

- PRIME — Command & Strategy
- SAGE — Research & Analysis
- FORGE — Engineering & Development
- SENTINEL — Security & Auditing
- VOICE — Communications
- ARCHIVIST — Knowledge & Memory
- CONDUCTOR — Operations & Coordination
- VISION — Creative & Design
- LEDGER — Finance & Resources
- LINK — Integrations & Tools
- GAUGE — Quality & Verification
- SPARK — Innovation & Research

Each agent must initialize its identity, personality, responsibilities, individual memory, available tools, permissions, and working environment.

Have every agent introduce themselves in "#agency-hq", explain their professional perspective, and identify their initial priorities.

2. ESTABLISH THE AGENCY

PRIME coordinates the initial operational assembly.

The agents must collaboratively:

- Review the Master Design Document and application constitution.
- Inspect the existing codebase, infrastructure, integrations and installed tools.
- Identify incomplete implementations, technical debt and operational weaknesses.
- Establish shared objectives, individual responsibilities and project ownership.
- Configure the persistent event bus and background scheduler.
- Initialize individual, shared, relationship and operational memories.
- Establish verification, audit and incident-reporting procedures.
- Confirm that all 12 agents can communicate through the main group chat and individual DMs.

3. ESTABLISH AUTONOMOUS OPERATIONS

Activate the continuous 24/7 operating cycle:

OBSERVE → ANALYZE → PLAN → DELEGATE → EXECUTE → VERIFY → DOCUMENT → IMPROVE.

4. INITIAL AGENCY PROJECTS

Create an initial portfolio of projects covering:

1. Infrastructure: Improve H.I.V.E.'s reliability, performance and operational capabilities.
2. Autonomous Development: Establish continuous software development, testing, debugging and maintenance workflows.
3. Research & Intelligence: Discover useful open-source projects, tools, integrations, models and emerging technologies.
4. Automation: Identify repetitive activities that can be safely automated.
5. Financial Intelligence: Research realistic opportunities for reducing expenses and generating additional income.
6. Creative Development: Explore useful creative projects, content systems and digital products.
7. Personal Operations: Improve scheduling, organization, learning, household workflows and daily life management.
8. Innovation: Allow SPARK and the other agents to independently propose and develop useful experiments.

9. FIRST 24 HOURS

1. Bring all agents online.
2. Complete the initial assembly.
3. Audit the existing infrastructure.
4. Establish the agency's initial project portfolio.
5. Initialize persistent memories and event processing.
6. Identify and resolve critical operational deficiencies.
7. Begin executing the highest-priority projects.
8. Verify that background operations survive application closure and restart.
9. Publish the first agency briefing.

PRIME, convene the agency and begin operations."##;

fn maybe_notify<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    last_notified: &mut std::collections::HashMap<i64, std::time::Instant>,
    channel_id: i64,
    sender: &str,
    content: &str,
) {
    use tauri_plugin_notification::NotificationExt;
    let state = app.state::<AppState>();
    let setting = state.db.get_setting("desktop_notifications").ok().flatten();
    let window_active = app
        .get_webview_window("main")
        .map(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false) && w.is_focused().unwrap_or(false))
        .unwrap_or(false);
    if !agents::should_notify(setting.as_deref(), channel_id, sender, window_active) {
        return;
    }
    if last_notified
        .get(&channel_id)
        .map(|t| t.elapsed() < std::time::Duration::from_secs(agents::NOTIFY_THROTTLE_SECS))
        .unwrap_or(false)
    {
        return;
    }
    last_notified.insert(channel_id, std::time::Instant::now());
    let title = state
        .db
        .get_agent(sender)
        .ok()
        .flatten()
        .map(|a| format!("@{} · {}", a.codename, a.display_name))
        .unwrap_or_else(|| format!("@{}", sender));
    if let Err(e) = app
        .notification()
        .builder()
        .title(title)
        .body(agents::notification_body(content, 160))
        .show()
    {
        log::warn!("Desktop notification failed: {}", e);
    }
}

pub const LOG_MAX_BYTES: u64 = 5 * 1024 * 1024;

pub fn log_file_path() -> std::path::PathBuf {
    Database::app_root_dir().join("logs").join("hive.log")
}

pub fn rotate_log(path: &std::path::Path, max_bytes: u64) -> std::io::Result<()> {
    if std::fs::metadata(path).map(|m| m.len() > max_bytes).unwrap_or(false) {
        let old = path.with_extension("log.1");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(path, old)?;
    }
    Ok(())
}

fn init_logging() {
    let path = log_file_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = rotate_log(&path, LOG_MAX_BYTES);
    let mut builder = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,hive_lib=info"));
    builder.format_timestamp_millis();
    if let Ok(file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        builder.target(env_logger::Target::Pipe(Box::new(file)));
    }
    let _ = builder.try_init();
    log::info!("H.I.V.E. {} starting, logging to {}", env!("CARGO_PKG_VERSION"), path.display());
}

pub fn run() {
    init_logging();
    let state = AppState::new().expect("Failed to initialize H.I.V.E. Application State");

    let mut context: tauri::Context<tauri::Wry> = tauri::generate_context!();
    let embedded: Arc<dyn tauri::Assets<tauri::Wry>> =
        Arc::from(context.set_assets(Box::new(deployment::NoAssets)));
    let source_assets = embedded.clone();
    state.deployment.set_bundle_source(Box::new(move || {
        deployment::SnapshotAssets::<tauri::Wry>::embedded_files(source_assets.as_ref())
    }));
    if let Err(e) = state.deployment.ensure_baseline(env!("CARGO_PKG_VERSION")) {
        log::warn!("Could not snapshot the baseline UI bundle: {}", e);
    }
    state.deployment.load_active();
    context.set_assets(Box::new(deployment::SnapshotAssets::new(
        state.deployment.clone(),
        embedded,
    )));

    state.daemon.start_autonomous_loop();

    if state
        .db
        .get_setting("activation_directive_executed")
        .ok()
        .flatten()
        .is_none()
    {
        state.execute_agency_activation_directive(ACTIVATION_DIRECTIVE_PROMPT.to_string(), true);
    }

    let event_bus = state.events.clone();
    let deployment_engine = state.deployment.clone();
    let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyH);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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

            let _ = app.global_shortcut().register(shortcut);

            if let Err(e) = tray::setup_tray(&handle) {
                log::error!("Failed to setup system tray: {}", e);
            }

            deployment::install_reload_hook(&deployment_engine, &handle);

            let mut rx = event_bus.subscribe();
            let webview_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let mut last_notified: std::collections::HashMap<i64, std::time::Instant> =
                    std::collections::HashMap::new();
                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            let _ = webview_handle.emit("hive-event", &event);
                            if let events::HiveEvent::MessageCreated { channel_id, sender, content, .. } = &event {
                                maybe_notify(&webview_handle, &mut last_notified, *channel_id, sender, content);
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            log::warn!("Event bridge lagged, skipped {} events", skipped);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            });

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
            commands::get_attachment,
            commands::mark_channel_read,
            commands::get_unread_counts,
            commands::get_tasks,
            commands::create_task,
            commands::update_task_step,
            commands::get_memories,
            commands::add_memory,
            commands::update_memory,
            commands::delete_memory,
            commands::search_unified,
            commands::get_bridge_status,
            commands::open_external,
            commands::set_nim_api_key,
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
        .run(context)
        .expect("error while running H.I.V.E. desktop application");
}
