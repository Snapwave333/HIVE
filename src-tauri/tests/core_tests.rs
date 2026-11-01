use hive_lib::db::models::{MemoryItem, Message, TaskChunk, TaskStep};
use hive_lib::db::Database;
use hive_lib::events::EventBus;
use hive_lib::governance::{DecisionLevel, GovernanceEngine};
use chrono::Utc;
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

    // Verify 12 permanent agents
    let agents = db.get_agents().expect("Should retrieve agents");
    assert_eq!(agents.len(), 12, "Expected exactly 12 permanent agents");

    let prime = agents.iter().find(|a| a.codename == "PRIME").expect("PRIME must exist");
    assert_eq!(prime.role, "Command / Strategy");
    assert_eq!(prime.model_tier, "nvidia/nemotron-3-ultra-550b-a55b");

    let forge = agents.iter().find(|a| a.codename == "FORGE").expect("FORGE must exist");
    assert_eq!(forge.role, "Engineering / Build");

    // Verify welcome message in #agency-hq
    let messages = db.get_messages(0, 10, None).expect("Should retrieve messages");
    assert_eq!(messages.len(), 1, "Expected welcome broadcast message");
    assert_eq!(messages[0].sender, "PRIME");
}

#[test]
fn test_message_crud_operations() {
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

    // Level 2: Significant -> Agency debate & vote with dissents
    let l2 = gov.propose_decision("Update Navigation Rail Spacing", "Adjust rail width to 52px", DecisionLevel::Significant, "VISION")
        .expect("Level 2 debate should succeed");
    assert_eq!(l2.status, "approved");
    assert!(l2.dissent_summary.is_some(), "Dissenting opinions must be recorded");

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

    let mem1 = MemoryItem {
        id: Uuid::new_v4().to_string(),
        category: "individual".to_string(),
        content: "FORGE prefers rust-analyzer with clippy strict checks.".to_string(),
        provenance: "agent_observation".to_string(),
        confidence: 0.95,
        is_speculation: false,
        tags: Some("engineering,preference".to_string()),
        created_at: now.clone(),
    };

    let mem2 = MemoryItem {
        id: Uuid::new_v4().to_string(),
        category: "shared".to_string(),
        content: "The owner requires all audio to remain strictly muted by default.".to_string(),
        provenance: "owner_directive".to_string(),
        confidence: 1.0,
        is_speculation: false,
        tags: Some("invariant,audio".to_string()),
        created_at: now,
    };

    db.insert_memory(&mem1).expect("Insert individual memory");
    db.insert_memory(&mem2).expect("Insert shared memory");

    let individual_memories = db.get_memories(Some("individual")).expect("Query individual");
    assert_eq!(individual_memories.len(), 1);
    assert_eq!(individual_memories[0].content, mem1.content);

    let shared_memories = db.get_memories(Some("shared")).expect("Query shared");
    assert_eq!(shared_memories.len(), 1);
    assert_eq!(shared_memories[0].content, mem2.content);
}

#[test]
fn test_unified_search() {
    let db = Database::new_in_memory().expect("In-memory DB should initialize");
    let search_res = db.search("PRIME").expect("Search should succeed");

    assert!(search_res["messages"].as_array().unwrap().len() >= 1, "Should find welcome message from PRIME");
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
