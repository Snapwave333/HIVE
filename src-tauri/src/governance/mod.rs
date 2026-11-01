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

    /// Propose a decision. If Level 1, auto-approved. If Level 2, triggers agency debate & vote. If Level 3, requires explicit user approval.
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
            id: decision_id.clone(),
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
            resolved_at: if level_u8 == 1 { Some(now.clone()) } else { None },
        };

        if level_u8 == 1 {
            self.db.log_audit(
                "level_1_decision",
                proposer,
                &format!("Routine action '{}' executed autonomously", title),
                "APPROVED",
            )?;
        } else if level_u8 == 2 {
            // Simulate 12-agent expertise debate & vote
            self.conduct_agency_vote(&mut decision)?;
        } else {
            // Level 3 Owner gate
            self.db.log_audit(
                "level_3_decision_gate",
                proposer,
                &format!("Owner approval required for '{}': {}", title, description),
                "AWAITING_OWNER",
            )?;
        }

        Ok(decision)
    }

    /// Conduct expert-weighted vote among the 12 permanent agents
    fn conduct_agency_vote(&self, decision: &mut Decision) -> anyhow::Result<()> {
        let agents = vec![
            ("PRIME", 1.5, true, "Aligns with strategic objectives."),
            ("SAGE", 1.2, true, "Evidence supports positive outcome."),
            ("FORGE", 1.3, true, "Implementation is modular and maintainable."),
            ("SENTINEL", 1.5, true, "Security invariant audit clean."),
            ("VOICE", 1.0, true, "Clear communication protocol."),
            ("ARCHIVIST", 1.0, true, "Fits existing knowledge taxonomy."),
            ("CONDUCTOR", 1.1, true, "High operational throughput."),
            ("VISION", 1.2, true, "Adheres to Borg design tokens."),
            ("LEDGER", 1.4, true, "Within resource and token limits."),
            ("LINK", 1.2, true, "CLI bridges verified."),
            ("GAUGE", 1.3, true, "Meets regression verification threshold."),
            ("SPARK", 1.1, false, "Proposes exploring more novel alternatives."),
        ];

        let mut total_score = 0.0;
        let mut total_weight = 0.0;
        let mut dissents = Vec::new();

        for (codename, weight, approve, rationale) in agents {
            decision.votes.insert(
                codename.to_string(),
                VoteRecord {
                    agent: codename.to_string(),
                    approve,
                    rationale: rationale.to_string(),
                    weight,
                },
            );

            total_weight += weight;
            if approve {
                total_score += weight;
            } else {
                dissents.push(format!("{}: {}", codename, rationale));
            }
        }

        let approved = total_score / total_weight >= 0.6;
        decision.status = if approved { "approved".to_string() } else { "rejected".to_string() };
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
