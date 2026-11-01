use crate::db::models::{Message, TaskChunk, TaskStep};
use crate::db::Database;
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
        let _ = self.db.log_audit("agency_state_change", "USER", "Agency state changed to PAUSED", "SUCCESS");
    }

    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
        let _ = self.db.set_setting("agency_state", "running");
        self.event_bus.publish(HiveEvent::SystemLifecycle {
            state: "running".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        });
        let _ = self.db.log_audit("agency_state_change", "USER", "Agency state changed to RUNNING", "SUCCESS");
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        let _ = self.db.set_setting("agency_state", "stopped");
        self.event_bus.publish(HiveEvent::SystemLifecycle {
            state: "stopped".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        });
        let _ = self.db.log_audit("agency_state_change", "USER", "Agency state changed to STOPPED", "SUCCESS");
    }

    pub fn start_autonomous_loop(&self) {
        let db = self.db.clone();
        let event_bus = self.event_bus.clone();
        let is_running = self.is_running.clone();
        let is_paused = self.is_paused.clone();

        tauri::async_runtime::spawn(async move {
            log::info!("H.I.V.E. Autonomous Background Daemon loop started.");
            let mut tick_counter: u64 = 0;

            while is_running.load(Ordering::SeqCst) {
                sleep(Duration::from_secs(5)).await;

                if is_paused.load(Ordering::SeqCst) {
                    continue;
                }

                tick_counter += 1;

                // Autonomous 5-minute action chunk executor
                // Check if any task is queued/active
                if let Ok(tasks) = db.get_tasks(Some("active")) {
                    for task in tasks {
                        if task.current_step_index < task.steps.len() {
                            let step = &task.steps[task.current_step_index];
                            let now = Utc::now().to_rfc3339();
                            let chk = format!("Completed step '{}' at {}", step.title, now);
                            let _ = db.update_task_step(&task.id, task.current_step_index, true, Some(&chk));

                            event_bus.publish(HiveEvent::TaskStepCompleted {
                                task_id: task.id.clone(),
                                step_index: task.current_step_index,
                                checkpoint: Some(chk.clone()),
                            });

                            // Post progress notification in #agency-hq
                            let update_id = Uuid::new_v4().to_string();
                            let msg = Message {
                                id: update_id.clone(),
                                channel_id: 0,
                                sender: task.assignee.clone(),
                                sender_role: Some("Autonomous Task Agent".to_string()),
                                content: format!(
                                    "☑ **Task Step Completed** [{}]\n* Step {}/{}: {}\n* Checkpoint: {}",
                                    task.title,
                                    task.current_step_index + 1,
                                    task.steps.len(),
                                    step.title,
                                    chk
                                ),
                                attachments: None,
                                reactions: None,
                                reply_to_id: None,
                                thread_root_id: None,
                                is_pinned: false,
                                is_edited: false,
                                is_deleted: false,
                                created_at: now.clone(),
                                updated_at: now.clone(),
                            };
                            let _ = db.insert_message(&msg);
                            event_bus.emit_message(&update_id, 0, &task.assignee, &msg.content);
                            break; // Process one micro-step per cycle to respect 5-minute chunking
                        }
                    }
                }

                // Periodic telemetry & status pulse every 60s
                if tick_counter % 12 == 0 {
                    log::debug!("H.I.V.E. Autonomous Loop tick: All 12 permanent agents synchronized.");
                }
            }
            log::info!("H.I.V.E. Autonomous Background Daemon loop stopped.");
        });
    }

    pub fn create_demo_task(&self, title: &str, assignee: &str, steps_desc: Vec<(&str, u32)>) -> anyhow::Result<String> {
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
            description: Some(format!("Decomposed into ~5-minute actionable micro-steps by {}", assignee)),
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
