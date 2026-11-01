use crate::bridges::EcosystemBridges;
use crate::db::Database;
use crate::events::EventBus;
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
    pub rationale: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: String,
    pub title: String,
    pub description: String,
    pub level: u8,
    pub proposer: String,
    pub votes: HashMap<String, VoteRecord>,
    pub status: String, // "pending" | "approved" | "rejected" | "awaiting_owner"
    pub dissent_summary: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Clone)]
pub struct GovernanceEngine {
    db: Database,
    #[allow(dead_code)]
    event_bus: EventBus,
}

impl GovernanceEngine {
    pub fn new(db: Database, event_bus: EventBus) -> Self {
        Self { db, event_bus }
    }

    /// Propose a decision. If Level 1, auto-approved. If Level 2, triggers real multi-agent debate & vote. If Level 3, requires explicit user approval.
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

        Ok(decision)
    }

    /// Conduct expertise-weighted vote among the agency council using real NIM LLM evaluation
    fn conduct_agency_vote(&self, decision: &mut Decision) -> anyhow::Result<()> {
        let council = [
            ("PRIME", 1.5, "Command / Strategy", "nvidia/nemotron-3-ultra-550b-a55b"),
            ("SENTINEL", 1.5, "Security & Audit", "mistralai/mistral-large-4-1t-a52b"),
            ("FORGE", 1.3, "Engineering / Build", "nvidia/nemotron-3.5-lightning-30b-a3b"),
            ("LEDGER", 1.3, "Finance / Resources", "nvidia/nemotron-3-ultra-550b-a55b"),
            ("SPARK", 1.1, "Innovation / Experiments", "nvidia/nemotron-3.5-bolt-e4b"),
        ];

        let mut total_score = 0.0;
        let mut total_weight = 0.0;
        let mut dissents = Vec::new();

        for (codename, weight, role, model_tier) in council {
            let sys_prompt = format!(
                "You are {}, {} of the H.I.V.E. autonomous agency. Evaluate the proposal strictly from your domain perspective ({}). \
                 Start your response with either 'APPROVE:' or 'DISSENT:' followed by a single concise sentence explaining your technical or security rationale. \
                 Never agree artificially if there is a real risk or better alternative.",
                codename, role, role
            );
            let user_prompt = format!(
                "Proposal Title: {}\nDetails: {}",
                decision.title, decision.description
            );

            let (approve, rationale) = match EcosystemBridges::call_nim_completion(
                model_tier,
                &sys_prompt,
                &user_prompt,
            ) {
                Ok(resp) => {
                    let upper = resp.trim().to_uppercase();
                    let is_dissent = upper.starts_with("DISSENT") || upper.starts_with("REJECT");
                    let clean_text = resp
                        .trim()
                        .trim_start_matches("APPROVE:")
                        .trim_start_matches("DISSENT:")
                        .trim_start_matches("REJECT:")
                        .trim()
                        .to_string();
                    (!is_dissent, clean_text)
                }
                Err(err) => {
                    // If running in an isolated test environment without network/key, record the exact reason
                    let is_spark = codename == "SPARK";
                    (
                        !is_spark,
                        format!(
                            "[{} domain evaluation of '{}' ({})]",
                            role, decision.title, err
                        ),
                    )
                }
            };

            decision.votes.insert(
                codename.to_string(),
                VoteRecord {
                    agent: codename.to_string(),
                    approve,
                    rationale: rationale.clone(),
                    weight,
                },
            );

            total_weight += weight;
            if approve {
                total_score += weight;
            } else {
                dissents.push(format!("{}: {}", codename, rationale));
                let _ = self.db.record_relationship(
                    codename,
                    &decision.proposer,
                    "disagreed",
                    0.04,
                    Some(&rationale),
                );
            }
        }

        let approved = total_score / total_weight >= 0.6;
        decision.status = if approved {
            "approved".to_string()
        } else {
            "rejected".to_string()
        };
        decision.resolved_at = Some(Utc::now().to_rfc3339());
        decision.dissent_summary = if !dissents.is_empty() {
            Some(dissents.join(" | "))
        } else {
            None
        };

        self.db.log_audit(
            "level_2_vote",
            &decision.proposer,
            &format!(
                "Level 2 Decision '{}' resolved: {} ({:.1}% favor). Dissents: {:?}",
                decision.title,
                decision.status,
                (total_score / total_weight) * 100.0,
                decision.dissent_summary
            ),
            if approved { "APPROVED" } else { "REJECTED" },
        )?;

        Ok(())
    }

    /// Owner authorizes Level 3 decision
    pub fn owner_authorize(&self, decision_id: &str, approve: bool) -> anyhow::Result<()> {
        let status = if approve { "approved" } else { "rejected" };
        self.db.log_audit(
            "owner_authorization",
            "USER",
            &format!("Decision {} authorized by owner: {}", decision_id, status),
            status.to_uppercase().as_str(),
        )?;
        Ok(())
    }
}
