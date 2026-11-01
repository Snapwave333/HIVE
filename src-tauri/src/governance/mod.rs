use crate::bridges::EcosystemBridges;
use crate::db::models::DecisionRow;
use crate::db::Database;
use crate::events::{EventBus, HiveEvent};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionLevel {
    Routine = 1,
    Significant = 2,
    OwnerControlled = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRecord {
    pub agent: String,
    pub approve: bool,
    #[serde(default)]
    pub abstained: bool,
    pub rationale: String,
    pub weight: f64,
}

pub const COUNCIL: [(&str, f64); 5] = [
    ("PRIME", 1.5),
    ("SENTINEL", 1.5),
    ("FORGE", 1.3),
    ("LEDGER", 1.3),
    ("SPARK", 1.1),
];

pub const QUORUM: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: String,
    pub title: String,
    pub description: String,
    pub level: u8,
    pub proposer: String,
    pub votes: HashMap<String, VoteRecord>,
    pub status: String,
    pub dissent_summary: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Clone)]
pub struct GovernanceEngine {
    db: Database,
    event_bus: EventBus,
}

impl GovernanceEngine {
    pub fn new(db: Database, event_bus: EventBus) -> Self {
        Self { db, event_bus }
    }

    pub fn propose_decision(
        &self,
        title: &str,
        description: &str,
        level: DecisionLevel,
        proposer: &str,
    ) -> anyhow::Result<Decision> {
        let decision_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let level_u8 = match level {
            DecisionLevel::Routine => 1,
            DecisionLevel::Significant => 2,
            DecisionLevel::OwnerControlled => 3,
        };

        let mut decision = Decision {
            id: decision_id,
            title: title.to_string(),
            description: description.to_string(),
            level: level_u8,
            proposer: proposer.to_string(),
            votes: HashMap::new(),
            status: match level_u8 {
                1 => "approved".to_string(),
                2 => "pending".to_string(),
                3 => "awaiting_owner".to_string(),
                _ => "pending".to_string(),
            },
            dissent_summary: None,
            created_at: now.clone(),
            resolved_at: if level_u8 == 1 { Some(now) } else { None },
        };

        self.event_bus.publish(HiveEvent::DecisionProposed {
            decision_id: decision.id.clone(),
            title: decision.title.clone(),
            level: level_u8,
            proposer: proposer.to_string(),
        });

        if level_u8 == 1 {
            self.db.log_audit(
                "level_1_decision",
                proposer,
                &format!("Routine action '{}' executed autonomously", title),
                "APPROVED",
            )?;
        } else if level_u8 == 2 {
            self.conduct_agency_vote(&mut decision)?;
        } else {
            self.db.log_audit(
                "level_3_decision_gate",
                proposer,
                &format!("Owner approval required for '{}': {}", title, description),
                "AWAITING_OWNER",
            )?;
        }

        self.persist(&decision);
        Ok(decision)
    }

    fn persist(&self, decision: &Decision) {
        let row = DecisionRow {
            id: decision.id.clone(),
            title: decision.title.clone(),
            description: decision.description.clone(),
            level: decision.level,
            proposer: decision.proposer.clone(),
            status: decision.status.clone(),
            votes_json: serde_json::to_string(&decision.votes).unwrap_or_else(|_| "{}".to_string()),
            dissent_summary: decision.dissent_summary.clone(),
            created_at: decision.created_at.clone(),
            resolved_at: decision.resolved_at.clone(),
        };
        if let Err(e) = self.db.save_decision(&row) {
            log::error!("Failed to persist decision {}: {}", decision.id, e);
        }
    }

    fn cast_vote(codename: &str, role: &str, model_tier: &str, persona: &str, decision: &Decision) -> (Option<bool>, String) {
        let sys_prompt = format!(
            "You are @{}, the {} agent at H.I.V.E.\n{}\n\
             Evaluate the proposal strictly from your own domain ({}).\n\
             Start your reply with either 'APPROVE:' or 'DISSENT:' followed by one plain-language sentence giving your real reason. \
             Don't agree just to be agreeable. If there's a real risk or a better option, dissent.",
            codename, role, persona, role
        );
        let user_prompt = format!(
            "Proposal: {}\nDetails: {}\nProposed by: @{}",
            decision.title, decision.description, decision.proposer
        );
        match EcosystemBridges::call_nim_completion(model_tier, &sys_prompt, &user_prompt) {
            Ok(resp) => {
                let cleaned = resp.trim().trim_start_matches(['*', '_', ' ']).to_string();
                let upper = cleaned.to_uppercase();
                let verdict = if upper.starts_with("APPROVE") {
                    Some(true)
                } else if upper.starts_with("DISSENT") || upper.starts_with("REJECT") {
                    Some(false)
                } else {
                    None
                };
                let rationale = cleaned
                    .split_once(':')
                    .filter(|(head, _)| head.len() <= 12)
                    .map(|(_, rest)| rest.trim().trim_start_matches('*').trim().to_string())
                    .unwrap_or_else(|| cleaned.clone());
                match verdict {
                    Some(v) => (Some(v), rationale),
                    None => (None, format!("Abstained: reply had no clear APPROVE/DISSENT ({})", rationale.chars().take(160).collect::<String>())),
                }
            }
            Err(err) => (None, format!("Abstained: couldn't reach model ({})", err)),
        }
    }

    fn conduct_agency_vote(&self, decision: &mut Decision) -> anyhow::Result<()> {
        let council: Vec<(String, f64, String, String, String)> = COUNCIL
            .iter()
            .map(|(code, weight)| {
                let agent = self.db.get_agent(code).ok().flatten();
                let role = agent
                    .as_ref()
                    .map(|a| a.role.clone())
                    .unwrap_or_else(|| crate::agents::role_for(code).to_string());
                let tier = agent
                    .as_ref()
                    .map(|a| a.model_tier.clone())
                    .unwrap_or_else(|| "nvidia/nemotron-3.5-lightning-30b-a3b".to_string());
                let persona = crate::agents::describe_personality(
                    agent.as_ref().and_then(|a| a.personality.as_deref()),
                );
                (code.to_string(), *weight, role, tier, persona)
            })
            .collect();

        let snapshot = decision.clone();
        let results: Vec<(String, f64, Option<bool>, String)> = std::thread::scope(|scope| {
            let handles: Vec<_> = council
                .iter()
                .map(|(code, weight, role, tier, persona)| {
                    let snap = &snapshot;
                    scope.spawn(move || {
                        let (verdict, rationale) = Self::cast_vote(code, role, tier, persona, snap);
                        (code.clone(), *weight, verdict, rationale)
                    })
                })
                .collect();
            handles
                .into_iter()
                .zip(council.iter())
                .map(|(h, (code, weight, ..))| {
                    h.join().unwrap_or_else(|_| {
                        (code.clone(), *weight, None, "Abstained: vote thread crashed".to_string())
                    })
                })
                .collect()
        });

        let mut total_score = 0.0;
        let mut total_weight = 0.0;
        let mut dissents = Vec::new();
        let mut abstentions = 0usize;

        for (codename, weight, verdict, rationale) in results {
            match verdict {
                Some(true) => {
                    total_weight += weight;
                    total_score += weight;
                }
                Some(false) => {
                    total_weight += weight;
                    dissents.push(format!("{}: {}", codename, rationale));
                    let _ = self.db.record_relationship(
                        &codename,
                        &decision.proposer,
                        "disagreed",
                        0.04,
                        Some(&rationale),
                    );
                }
                None => abstentions += 1,
            }
            decision.votes.insert(
                codename.clone(),
                VoteRecord {
                    agent: codename,
                    approve: verdict == Some(true),
                    abstained: verdict.is_none(),
                    rationale,
                    weight,
                },
            );
        }

        let quorum = COUNCIL.len() - abstentions >= QUORUM;
        let ratio = if total_weight > 0.0 { total_score / total_weight } else { 0.0 };
        decision.status = if !quorum {
            "inconclusive".to_string()
        } else if ratio >= 0.6 {
            "approved".to_string()
        } else {
            "rejected".to_string()
        };
        decision.resolved_at = Some(Utc::now().to_rfc3339());
        decision.dissent_summary = if dissents.is_empty() {
            None
        } else {
            Some(dissents.join(" | "))
        };

        self.db.log_audit(
            "level_2_vote",
            &decision.proposer,
            &format!(
                "Level 2 Decision '{}' resolved: {} ({:.1}% of cast weight in favor, {} abstained). Dissents: {:?}",
                decision.title,
                decision.status,
                ratio * 100.0,
                abstentions,
                decision.dissent_summary
            ),
            &decision.status.to_uppercase(),
        )?;

        self.event_bus.publish(HiveEvent::DecisionResolved {
            decision_id: decision.id.clone(),
            approved: decision.status == "approved",
            vote_summary: format!("{} ({} abstained)", decision.status, abstentions),
        });

        Ok(())
    }

    pub fn owner_authorize(&self, decision_id: &str, approve: bool) -> anyhow::Result<()> {
        let status = if approve { "approved" } else { "rejected" };
        let row = self.db.resolve_decision(decision_id, status)?;
        if row.is_none() {
            anyhow::bail!("Decision {} not found", decision_id);
        }
        self.db.log_audit(
            "owner_authorization",
            "USER",
            &format!("Decision {} authorized by owner: {}", decision_id, status),
            status.to_uppercase().as_str(),
        )?;
        self.event_bus.publish(HiveEvent::DecisionResolved {
            decision_id: decision_id.to_string(),
            approved: approve,
            vote_summary: format!("Owner {}", status),
        });
        Ok(())
    }
}
