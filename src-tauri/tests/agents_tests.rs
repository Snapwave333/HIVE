use chrono::{Duration as ChronoDuration, Utc};
use hive_lib::agents::{self, PromptContext};
use hive_lib::bridges::{EcosystemBridges, NimFailure};
use hive_lib::daemon::autonomous_delay;
use hive_lib::db::models::{MemoryItem, Message};
use hive_lib::db::Database;
use std::time::Duration;
use uuid::Uuid;

fn msg(sender: &str, content: &str) -> Message {
    let now = Utc::now().to_rfc3339();
    Message {
        id: Uuid::new_v4().to_string(),
        channel_id: 0,
        sender: sender.to_string(),
        sender_role: None,
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
    }
}

#[test]
fn agent_definitions_from_json_override_stale_db_values() {
    let db = Database::new_in_memory().unwrap();
    let prime = db.get_agent("PRIME").unwrap().unwrap();
    assert!(prime.personality.as_deref().unwrap().contains("contractions"));

    db.update_agent_status("PRIME", "thinking").unwrap();
    db.seed_defaults().unwrap();
    let prime = db.get_agent("PRIME").unwrap().unwrap();
    assert_eq!(prime.status, "available", "Stuck 'thinking' status resets on startup");
}

#[test]
fn seed_data_contains_no_fabricated_history() {
    let db = Database::new_in_memory().unwrap();
    assert!(db.get_relationships().unwrap().is_empty());
    assert!(db.get_tasks(None).unwrap().is_empty());
    let mems = db.get_memories(None).unwrap();
    assert!(!mems.is_empty());
    assert!(mems.iter().all(|m| m.provenance.starts_with("constitution.json#")));
    let welcome = db.get_messages(0, 5, None).unwrap();
    assert!(!welcome[0].content.contains("@gmail.com"));
}

#[test]
fn system_prompt_carries_persona_and_house_style() {
    let db = Database::new_in_memory().unwrap();
    let spark = db.get_agent("SPARK").unwrap().unwrap();
    let memories = vec!["Tried caching embeddings, it was 3x faster".to_string()];
    let prompt = agents::build_system_prompt(
        &spark,
        &PromptContext {
            setting: "a private DM with the Owner.",
            live_data: Some("Relationship graph: 0 edges."),
            memories: &memories,
            instructions: "Reply briefly.",
        },
    );
    assert!(prompt.contains("@SPARK"));
    assert!(prompt.contains("Innovation / Experiments"));
    assert!(prompt.contains("How you talk:"));
    assert!(prompt.contains("prototype evaluation"), "skills are humanized");
    assert!(prompt.contains("Talk like a real coworker"));
    assert!(prompt.contains("Never make up results"));
    assert!(prompt.contains("3x faster"));
    assert!(prompt.contains("Relationship graph: 0 edges."));
    assert!(!prompt.contains("@SPARK, @"), "teammate list excludes self");
}

#[test]
fn mentions_are_word_bounded_and_ordered() {
    assert_eq!(agents::find_mentions("hey @forge and @Sage, thoughts?"), vec!["FORGE", "SAGE"]);
    assert_eq!(agents::find_mentions("@SAGE then @FORGE then @SAGE"), vec!["SAGE", "FORGE"]);
    assert!(agents::find_mentions("mail me at someone@gmail.com").is_empty());
    assert!(agents::find_mentions("@PRIMEtime is not a mention").is_empty());
    assert!(agents::find_mentions("x@LINK is part of a word").is_empty());
    assert_eq!(agents::find_mentions("(@LINK)"), vec!["LINK"]);
}

#[test]
fn transcript_maps_roles_merges_and_labels() {
    let history = vec![
        msg("FORGE", "I'll go first"),
        msg("user", "What's the plan?"),
        msg("SAGE", "Research first."),
        msg("FORGE", "Agreed, then build."),
        msg("user", "Ok @FORGE go"),
    ];
    let turns = agents::transcript_turns("FORGE", &history, 500);
    assert_eq!(turns.len(), 3, "leading assistant turn dropped, consecutive user turns merged");
    assert_eq!(turns[0].role, "user");
    assert!(turns[0].content.contains("Owner: What's the plan?"));
    assert!(turns[0].content.contains("@SAGE: Research first."));
    assert_eq!(turns[1].role, "assistant");
    assert_eq!(turns[1].content, "Agreed, then build.");
    assert_eq!(turns[2].role, "user");

    let mut deleted = msg("user", "secret");
    deleted.is_deleted = true;
    assert!(agents::transcript_turns("FORGE", &[deleted], 500).is_empty());

    let long = agents::transcript_turns("FORGE", &[msg("user", &"a".repeat(50))], 10);
    assert!(long[0].content.ends_with('…'));
}

#[test]
fn replies_are_cleaned_and_passes_detected() {
    assert_eq!(agents::clean_reply("PRIME", "**@PRIME:** sounds good"), "sounds good");
    assert_eq!(agents::clean_reply("PRIME", "@PRIME (Command / Strategy): ok"), "ok");
    assert_eq!(agents::clean_reply("PRIME", "PRIME: fine"), "fine");
    assert_eq!(agents::clean_reply("PRIME", "@FORGE: nope"), "@FORGE: nope");
    assert_eq!(agents::clean_reply("PRIME", "Primed and ready: yes"), "Primed and ready: yes");
    assert!(agents::is_pass("[PASS]"));
    assert!(agents::is_pass("  **[pass]** "));
    assert!(agents::is_pass("PASS"));
    assert!(!agents::is_pass("Passing thought: we should cache that"));
}

#[test]
fn task_plans_and_step_lists_parse_robustly() {
    let plan = agents::parse_task_plan(
        "**TITLE:** Speed up search\n**ASSIGNEE:** @sage\nSTEP1: Profile LIKE queries\nStep 2: Try FTS5\n- STEP3: `Benchmark`",
        "FORGE",
    )
    .unwrap();
    assert_eq!(plan.title, "Speed up search");
    assert_eq!(plan.assignee, "SAGE");
    assert_eq!(plan.steps, vec!["Profile LIKE queries", "Try FTS5", "Benchmark"]);

    let fallback = agents::parse_task_plan("TITLE: X\nASSIGNEE: nobody\nSTEP1: a", "FORGE").unwrap();
    assert_eq!(fallback.assignee, "FORGE");
    assert!(agents::parse_task_plan("just some prose", "FORGE").is_none());

    let steps = agents::parse_step_lines("1. Open logs\n2) Grep errors\n- **Fix the 3D bug**\n\n", 3);
    assert_eq!(steps, vec!["Open logs", "Grep errors", "Fix the 3D bug"]);
}

#[test]
fn agent_memory_recall_prefers_own_memories() {
    let db = Database::new_in_memory().unwrap();
    let now = Utc::now();
    for (i, (prov, cat, content)) in [
        ("agent:FORGE:note", "individual", "FORGE likes small PRs"),
        ("agent:SAGE:note", "individual", "SAGE private note"),
    ]
    .iter()
    .enumerate()
    {
        db.insert_memory(&MemoryItem {
            id: Uuid::new_v4().to_string(),
            category: cat.to_string(),
            content: content.to_string(),
            provenance: prov.to_string(),
            confidence: 1.0,
            is_speculation: false,
            tags: None,
            created_at: (now + ChronoDuration::seconds(i as i64)).to_rfc3339(),
        })
        .unwrap();
    }
    let forge = db.get_agent_memories("FORGE", 3).unwrap();
    assert_eq!(forge[0].content, "FORGE likes small PRs");
    assert!(forge.iter().all(|m| m.content != "SAGE private note"));
}

#[test]
fn relationship_weights_decay_without_reinforcement() {
    let db = Database::new_in_memory().unwrap();
    db.record_relationship("SAGE", "FORGE", "collaborated", 0.3, None).unwrap();
    let t0 = Utc::now() + ChronoDuration::seconds(1);
    assert_eq!(db.decay_relationships_at(30.0, t0).unwrap(), 0, "first run only records a baseline");

    let touched = db.decay_relationships_at(30.0, t0 + ChronoDuration::days(30)).unwrap();
    assert_eq!(touched, 1);
    let w = db.get_relationships().unwrap()[0].weight;
    assert!((w - 0.4).abs() < 1e-6, "one half-life halves the weight, got {}", w);

    db.set_setting("relationships_last_decay", &(Utc::now() - ChronoDuration::days(1)).to_rfc3339()).unwrap();
    db.record_relationship("SAGE", "FORGE", "collaborated", 0.1, None).unwrap();
    let touched = db.decay_relationships_at(30.0, Utc::now() + ChronoDuration::days(29)).unwrap();
    assert_eq!(touched, 0, "edges reinforced since the last decay are exempt");

    for _ in 0..20 {
        let _ = db.decay_relationships_at(1.0, Utc::now() + ChronoDuration::days(400));
    }
    assert!(db.get_relationships().unwrap()[0].weight >= 0.1 - 1e-9, "weights never drop below the floor");
}

#[test]
fn autonomous_pacing_backs_off_when_idle_or_failing() {
    assert_eq!(autonomous_delay(20, 0, 0), Duration::from_secs(20));
    assert_eq!(autonomous_delay(20, 900, 0), Duration::from_secs(80));
    assert_eq!(autonomous_delay(20, 7_200, 0), Duration::from_secs(240));
    assert_eq!(autonomous_delay(20, 0, 2), Duration::from_secs(80));
    assert_eq!(autonomous_delay(20, 7_200, 9), Duration::from_secs(900), "capped at 15 minutes");
    assert_eq!(autonomous_delay(1, 0, 0), Duration::from_secs(5), "floor protects the API");
}

#[test]
fn nim_router_orders_cascade_and_classifies_failures() {
    let order = EcosystemBridges::cascade_order("openai/gpt-oss-20b", &|m: &str| m == "nvidia/nemotron-3.5-lightning-30b-a3b");
    assert_eq!(order[0], "openai/gpt-oss-20b");
    assert_eq!(order.last().unwrap(), "nvidia/nemotron-3.5-lightning-30b-a3b", "cooling models go last");
    assert_eq!(order.iter().filter(|m| *m == "openai/gpt-oss-20b").count(), 1, "no duplicates");

    let all_cool = EcosystemBridges::cascade_order("openai/gpt-oss-20b", &|_| true);
    assert_eq!(all_cool[0], "openai/gpt-oss-20b", "still tries in order when everything is cooling");

    assert!(matches!(EcosystemBridges::classify_http(401, "{}"), NimFailure::Auth(_)));
    assert!(matches!(
        EcosystemBridges::classify_http(429, r#"{"error":{"message":"slow down"}}"#),
        NimFailure::RateLimited(ref d) if d.contains("slow down")
    ));
    assert!(matches!(EcosystemBridges::classify_http(404, "{}"), NimFailure::ModelMissing(_)));
    assert!(matches!(EcosystemBridges::classify_http(503, "busy"), NimFailure::Unavailable(_)));
    assert!(NimFailure::Auth(String::new()).cooldown().is_none());
    assert!(NimFailure::RateLimited(String::new()).cooldown().is_some());
    assert_eq!(EcosystemBridges::strip_think("<think>hmm</think> Hi there"), "Hi there");
}

#[test]
fn search_escapes_like_wildcards() {
    let db = Database::new_in_memory().unwrap();
    db.insert_message(&msg("user", "progress is 100% done")).unwrap();
    db.insert_message(&msg("user", "progress is 100 done")).unwrap();
    let res = db.search("100%").unwrap();
    assert_eq!(res["messages"].as_array().unwrap().len(), 1);
    let res = db.search("_").unwrap();
    assert_eq!(res["messages"].as_array().unwrap().len(), 0);
}
