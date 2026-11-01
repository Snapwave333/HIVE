use chrono::Utc;
use hive_lib::db::models::{MemoryItem, Message, TaskChunk, TaskStep};
use hive_lib::db::Database;
use hive_lib::deployment::DeploymentEngine;
use hive_lib::events::EventBus;
use hive_lib::governance::{DecisionLevel, GovernanceEngine};
use std::path::PathBuf;
use uuid::Uuid;

#[test]
fn test_database_initialization_and_seeding() {
    let db = Database::new_in_memory().expect("In-memory DB should initialize");

    // Verify channels: #agency-hq + 12 agent DMs = 13 channels
    let channels = db.get_channels().expect("Should retrieve channels");
    assert_eq!(channels.len(), 13, "Expected exactly 13 channels (#agency-hq + 12 agent DMs)");

    let hq = &channels[0];
    assert_eq!(hq.id, 0);
    assert_eq!(hq.name, "agency-hq");
    assert!(hq.pinned, "#agency-hq must be pinned");

    let agents = db.get_agents().expect("Should retrieve agents");
    assert_eq!(agents.len(), 12, "Expected exactly 12 permanent agents");

    let prime = agents.iter().find(|a| a.codename == "PRIME").expect("PRIME must exist");
    assert_eq!(prime.role, "Command / Strategy");
    assert_eq!(prime.model_tier, "nvidia/nemotron-3-ultra-550b-a55b");
    assert!(prime.personality.is_some(), "PRIME personality JSON should be seeded");
    assert!(prime.skills.is_some(), "PRIME skills JSON should be seeded");

    let forge = agents.iter().find(|a| a.codename == "FORGE").expect("FORGE must exist");
    assert_eq!(forge.role, "Engineering / Build");

    // Verify welcome message in #agency-hq
    let messages = db.get_messages(0, 10, None).expect("Should retrieve messages");
    assert_eq!(messages.len(), 1, "Expected welcome broadcast message");
    assert_eq!(messages[0].sender, "PRIME");
}

#[test]
fn test_message_crud_reactions_and_forwarding() {
    let db = Database::new_in_memory().expect("In-memory DB should initialize");
    let now = Utc::now().to_rfc3339();
    let msg_id = Uuid::new_v4().to_string();

    let msg = Message {
        id: msg_id.clone(),
        channel_id: 0,
        sender: "user".to_string(),
        sender_role: Some("Owner".to_string()),
        content: "Hello H.I.V.E.".to_string(),
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

    db.insert_message(&msg).expect("Message insert should succeed");

    // Toggle reactions (add then remove)
    let r1 = db.toggle_reaction(&msg_id, "(ง'̀-'́)ง", "user").expect("Add reaction");
    assert!(r1.contains("(ง'̀-'́)ง"));
    let r2 = db.toggle_reaction(&msg_id, "(ง'̀-'́)ง", "user").expect("Remove reaction");
    assert!(!r2.contains("user"));

    // Forward message to @PRIME DM (channel 1)
    let fwd = db.forward_message(&msg_id, 1).expect("Forward should succeed");
    assert_eq!(fwd.channel_id, 1);
    assert!(fwd.content.contains("Hello H.I.V.E."));

    // Edit message
    db.update_message(&msg_id, "Hello H.I.V.E. updated").expect("Edit should succeed");
    let msgs = db.get_messages(0, 10, None).expect("Retrieve messages");
    let updated = msgs.iter().find(|m| m.id == msg_id).expect("Message found");
    assert_eq!(updated.content, "Hello H.I.V.E. updated");
    assert!(updated.is_edited);

    // Delete message
    db.delete_message(&msg_id).expect("Delete should succeed");
    let msgs_after = db.get_messages(0, 10, None).expect("Retrieve messages");
    let deleted = msgs_after.iter().find(|m| m.id == msg_id).expect("Message found");
    assert!(deleted.is_deleted);
    assert_eq!(deleted.content, "[Message deleted]");
}

#[test]
fn test_5_minute_task_chunking() {
    let db = Database::new_in_memory().expect("In-memory DB should initialize");
    let now = Utc::now().to_rfc3339();
    let task_id = Uuid::new_v4().to_string();

    let steps = vec![
        TaskStep {
            index: 0,
            title: "Inspect target code".to_string(),
            estimated_minutes: 5,
            completed: false,
            completed_at: None,
        },
        TaskStep {
            index: 1,
            title: "Write failing unit test".to_string(),
            estimated_minutes: 5,
            completed: false,
            completed_at: None,
        },
    ];

    let task = TaskChunk {
        id: task_id.clone(),
        title: "Build Feature X".to_string(),
        description: Some("ADHD-friendly micro-actions".to_string()),
        assignee: "FORGE".to_string(),
        priority: "high".to_string(),
        status: "active".to_string(),
        steps,
        current_step_index: 0,
        resume_checkpoint: Some("Ready for Step 0".to_string()),
        created_at: now.clone(),
        updated_at: now,
    };

    db.insert_task(&task).expect("Insert task should succeed");

    // Step 0 completion
    db.update_task_step(&task_id, 0, true, Some("Step 0 verified"))
        .expect("Update step 0 should succeed");

    let tasks = db.get_tasks(None).expect("Retrieve tasks");
    let current_task = tasks.iter().find(|t| t.id == task_id).expect("Found task");
    assert_eq!(current_task.current_step_index, 1);
    assert_eq!(current_task.resume_checkpoint.as_deref(), Some("Step 0 verified"));
    assert_eq!(current_task.status, "active");

    // Step 1 completion -> task finishes
    db.update_task_step(&task_id, 1, true, Some("All steps complete"))
        .expect("Update step 1 should succeed");

    let tasks_done = db.get_tasks(None).expect("Retrieve tasks");
    let completed_task = tasks_done.iter().find(|t| t.id == task_id).expect("Found task");
    assert_eq!(completed_task.status, "completed");
}

#[test]
fn test_three_level_governance() {
    let db = Database::new_in_memory().expect("In-memory DB should initialize");
    let events = EventBus::new(16);
    let gov = GovernanceEngine::new(db.clone(), events);

    // Level 1: Routine -> Auto approved
    let l1 = gov.propose_decision("Fix typo in CSS", "Minor cosmetic update", DecisionLevel::Routine, "VISION")
        .expect("Level 1 should succeed");
    assert_eq!(l1.status, "approved");

    // Level 2: Significant -> Live NIM council debate & weighted vote
    let l2 = gov.propose_decision("Optimize SQLite WAL Checkpoint Interval", "Tune WAL autocheckpoint to 1000 pages for low-latency writes", DecisionLevel::Significant, "FORGE")
        .expect("Level 2 debate should succeed");
    assert_eq!(l2.votes.len(), 5, "All 5 council agents must be recorded");
    let cast = l2.votes.values().filter(|v| !v.abstained).count();
    if cast < hive_lib::governance::QUORUM {
        assert_eq!(l2.status, "inconclusive", "Without quorum the council must not fake a verdict");
        assert!(l2.votes.values().filter(|v| v.abstained).all(|v| !v.approve));
    } else {
        assert!(l2.status == "approved" || l2.status == "rejected");
    }

    // Level 3: Owner Controlled -> Requires user authorization
    let l3 = gov.propose_decision("Transfer System Authority", "Dangerous action", DecisionLevel::OwnerControlled, "PRIME")
        .expect("Level 3 proposal should succeed");
    assert_eq!(l3.status, "awaiting_owner");

    gov.owner_authorize(&l3.id, true).expect("Owner authorization should succeed");
}

#[test]
fn test_four_tier_memory() {
    let db = Database::new_in_memory().expect("In-memory DB should initialize");
    let now = Utc::now().to_rfc3339();
    let mem_id = Uuid::new_v4().to_string();

    let mem1 = MemoryItem {
        id: mem_id.clone(),
        category: "individual".to_string(),
        content: "FORGE prefers rust-analyzer with clippy strict checks.".to_string(),
        provenance: "agent_observation".to_string(),
        confidence: 0.95,
        is_speculation: false,
        tags: Some("engineering,preference".to_string()),
        created_at: now,
    };

    db.insert_memory(&mem1).expect("Insert individual memory");

    let individual_memories = db.get_memories(Some("individual")).expect("Query individual");
    assert!(individual_memories.iter().any(|m| m.id == mem_id));

    // Update memory
    db.update_memory(&mem_id, "Updated FORGE preference", true).expect("Update memory");
    let after_update = db.get_memories(Some("individual")).expect("Query after update");
    let found = after_update.iter().find(|m| m.id == mem_id).unwrap();
    assert_eq!(found.content, "Updated FORGE preference");
    assert!(found.is_speculation);

    // Delete memory
    db.delete_memory(&mem_id).expect("Delete memory");
    let after_del = db.get_memories(Some("individual")).expect("Query after delete");
    assert!(!after_del.iter().any(|m| m.id == mem_id));
}

#[test]
fn test_relationships_and_temp_specialists() {
    let db = Database::new_in_memory().expect("In-memory DB should initialize");

    let rels = db.get_relationships().expect("Get relationships");
    assert!(rels.is_empty(), "Relationship graph must start empty and only grow from real interactions");

    db.record_relationship("SAGE", "FORGE", "collaborated", 0.2, Some("Co-authored benchmark"))
        .expect("Record relationship");
    let rels = db.get_relationships().expect("Get relationships");
    assert_eq!(rels.len(), 1);
    assert!((rels[0].weight - 0.7).abs() < 1e-9);

    // Spawn temp specialist
    let spec = db
        .spawn_temp_specialist("rust_profiler", "Performance Specialist", "FORGE", 15)
        .expect("Spawn temp specialist");
    assert!(spec.codename.starts_with("temp_"), "Temp specialist must be prefixed with temp_");
    assert_eq!(spec.sponsor_agent, "FORGE");
}

#[test]
fn test_blue_green_deployment_and_rollback() {
    let db = Database::new_in_memory().expect("In-memory DB should initialize");
    let events = EventBus::new(16);
    let root = std::env::temp_dir().join(format!("hive-deploy-test-{}", Uuid::new_v4()));
    let engine = DeploymentEngine::with_root(db.clone(), events, root.clone());

    assert!(
        engine.stage_and_deploy_version("v9.9.9", "FORGE", "no bundle source yet", true).is_err(),
        "Deploying without a real running bundle must fail instead of faking a snapshot"
    );

    let good_v1: Vec<(String, Vec<u8>)> = vec![
        ("/index.html".to_string(), b"<html><script src=\"./assets/a.js\"></script></html>".to_vec()),
        ("/assets/a.js".to_string(), b"console.log('v1')".to_vec()),
    ];
    let good_v2: Vec<(String, Vec<u8>)> = vec![
        ("/index.html".to_string(), b"<html><script src=\"./assets/a.js\"></script></html>".to_vec()),
        ("/assets/a.js".to_string(), b"console.log('v2')".to_vec()),
    ];
    let bad: Vec<(String, Vec<u8>)> = vec![
        ("/index.html".to_string(), b"<html></html>".to_vec()),
        ("/assets/a.js".to_string(), b"render(VirtualWorld3D)".to_vec()),
    ];

    let v1 = engine.stage_bundle("v1.0.0", "FORGE", "Baseline", &good_v1, true).expect("v1 deploys");
    assert_eq!(v1.sha256_hash, hash_of(&good_v1));
    assert!(PathBuf::from(&v1.directory_path).join("assets/a.js").is_file());

    let v2 = engine.stage_bundle("v1.0.1", "FORGE", "Refined bubble padding", &good_v2, true).expect("v2 deploys");
    assert!(v2.is_active);
    assert_ne!(v1.sha256_hash, v2.sha256_hash, "Different bundles must hash differently");
    assert_eq!(engine.resolve_asset("/assets/a.js").as_deref(), Some(&b"console.log('v2')"[..]));
    assert_eq!(engine.resolve_asset("/__preview/v1.0.0/assets/a.js").as_deref(), Some(&b"console.log('v1')"[..]));
    assert!(engine.resolve_asset("/../../etc/passwd").is_none(), "Path traversal must be rejected");

    let blocked = engine.stage_bundle("v1.0.2", "SPARK", "Harmless sounding summary", &bad, true);
    assert!(blocked.is_err(), "Validator must scan the real bundle source, not just the summary");
    assert!(engine.stage_bundle("../evil", "SPARK", "x", &good_v1, true).is_err());

    let rolled = engine.rollback("Regression test rollback").expect("Rollback should succeed");
    assert_eq!(rolled.version, "v1.0.0");
    assert_eq!(engine.resolve_asset("/assets/a.js").as_deref(), Some(&b"console.log('v1')"[..]));

    std::fs::write(PathBuf::from(&v2.directory_path).join("assets/a.js"), b"tampered").unwrap();
    let after_tamper = engine.rollback("Try to roll onto tampered snapshot");
    assert!(after_tamper.is_err(), "Tampered snapshots must fail the integrity check");

    let _ = std::fs::remove_dir_all(root);
}

fn hash_of(files: &[(String, Vec<u8>)]) -> String {
    hive_lib::deployment::bundle_hash(files)
}

#[test]
fn test_unified_search() {
    let db = Database::new_in_memory().expect("In-memory DB should initialize");
    let search_res = db.search("agency-hq").expect("Search should succeed");
    assert!(!search_res["messages"].as_array().unwrap().is_empty(), "Should find the welcome message");
    let constitution_hits = db.search("Constitution").expect("Search should succeed");
    assert!(!constitution_hits["memories"].as_array().unwrap().is_empty(), "Should find constitution memories");
}

#[test]
fn test_constitution_invariants_preserved() {
    let raw_constitution = include_str!("../../app/constitution.json");
    let parsed: serde_json::Value = serde_json::from_str(raw_constitution).expect("Valid JSON");

    let invariants = &parsed["invariants"];
    assert!(invariants["user_sovereignty"]["immutable"].as_bool().unwrap());
    assert!(invariants["agency_chat_architecture"]["immutable"].as_bool().unwrap());
    assert!(invariants["agent_permanence"]["immutable"].as_bool().unwrap());
    assert!(invariants["messaging_interaction_model"]["immutable"].as_bool().unwrap());
    assert!(invariants["audio_and_notification_sovereignty"]["immutable"].as_bool().unwrap());
    assert!(invariants["accessibility_baseline"]["immutable"].as_bool().unwrap());
    assert!(invariants["zero_dashboard_bloat"]["immutable"].as_bool().unwrap());
}
