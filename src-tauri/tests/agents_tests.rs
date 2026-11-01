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

#[test]
fn unread_counts_track_agent_messages_since_last_read() {
    let db = Database::new_in_memory().unwrap();
    let base = db.get_unread_counts().unwrap();
    assert_eq!(base.get(&0).copied(), Some(1), "the welcome message starts unread");

    db.mark_channel_read(0).unwrap();
    assert_eq!(db.get_unread_counts().unwrap().get(&0), None);

    std::thread::sleep(Duration::from_millis(5));
    let mut agent_msg = msg("FORGE", "build is green");
    agent_msg.created_at = (Utc::now() + ChronoDuration::seconds(1)).to_rfc3339();
    db.insert_message(&agent_msg).unwrap();
    let mut own = msg("user", "nice");
    own.created_at = (Utc::now() + ChronoDuration::seconds(1)).to_rfc3339();
    db.insert_message(&own).unwrap();
    assert_eq!(db.get_unread_counts().unwrap().get(&0).copied(), Some(1), "owner's own messages never count");
}

#[test]
fn attachments_are_described_and_text_excerpted_for_agents() {
    use base64::Engine;
    let db = Database::new_in_memory().unwrap();
    let body = "name,score\nforge,9\nsage,8\n";
    let meta = db
        .save_attachment(
            &format!("hive-test-{}.csv", Uuid::new_v4()),
            "text/csv",
            &base64::engine::general_purpose::STANDARD.encode(body),
        )
        .unwrap();
    let image = db
        .save_attachment("pic.png", "image/png", &base64::engine::general_purpose::STANDARD.encode([137u8, 80, 78, 71]))
        .unwrap();
    let raw = serde_json::json!([
        { "hash": meta.hash, "filename": "scores.csv", "mime_type": "text/csv", "size_bytes": meta.size_bytes },
        { "hash": image.hash, "filename": "pic.png", "mime_type": "image/png", "size_bytes": 4 }
    ])
    .to_string();

    assert_eq!(agents::parse_attachments(Some(&raw)).len(), 2);
    assert!(agents::parse_attachments(Some("legacy.txt (C:/x)")).is_empty());

    let full = agents::describe_attachments(&db, Some(&raw), true);
    assert!(full.contains("scores.csv"));
    assert!(full.contains("forge,9"), "text contents are excerpted for the model");
    assert!(full.contains("You can't see images"));

    let short = agents::describe_attachments(&db, Some(&raw), false);
    assert!(!short.contains("forge,9"), "older messages only get a one-line note");

    let mut m = msg("user", "");
    m.attachments = Some(raw);
    let noted = agents::with_attachment_notes(&db, &[m]);
    assert!(noted[0].content.starts_with("[Attached file: scores.csv"));

    assert!(agents::is_text_like("", "notes.MD"));
    assert!(!agents::is_text_like("application/pdf", "doc.pdf"));
    assert!(db.save_attachment("bad.bin", "application/octet-stream", "%%%not-base64%%%").is_err());
}

#[test]
fn full_text_search_stems_ranks_and_tracks_edits() {
    let db = Database::new_in_memory().unwrap();
    let a = msg("FORGE", "Deploying the new bundle now, snapshots look healthy");
    let b = msg("SAGE", "Unrelated note about research papers");
    let c = msg("GAUGE", "deploy deploy deploy: the deploy pipeline needs another deploy check");
    for m in [&a, &b, &c] {
        db.insert_message(m).unwrap();
    }

    let res = db.search("deploy").unwrap();
    let ids: Vec<&str> = res["messages"].as_array().unwrap().iter().map(|m| m["id"].as_str().unwrap()).collect();
    assert_eq!(ids.len(), 2, "prefix matching finds Deploying and deploy but not the unrelated note");
    assert_eq!(ids[0], c.id, "bm25 ranks the denser match first");

    db.update_message(&b.id, "Now this one mentions deployment too").unwrap();
    assert_eq!(db.search("deploy").unwrap()["messages"].as_array().unwrap().len(), 3, "edits are re-indexed");

    db.delete_message(&a.id).unwrap();
    let after = db.search("deploy").unwrap();
    assert!(after["messages"].as_array().unwrap().iter().all(|m| m["id"] != a.id.as_str()), "deleted messages never show up");

    let long = msg("VOICE", &format!("{} the needle is here {}", "filler ".repeat(200), "padding ".repeat(200)));
    db.insert_message(&long).unwrap();
    let hit = db.search("needle").unwrap();
    let snippet = hit["messages"][0]["snippet"].as_str().unwrap();
    assert!(snippet.contains("needle") && snippet.len() < 300, "long messages return an excerpt around the match");

    assert!(db.search("research papers").unwrap()["messages"].as_array().unwrap().is_empty(), "the edited-away text is gone from the index");
    assert_eq!(Database::fts_query("hi \" OR x"), Some("\"hi\"* \"OR\"* \"x\"*".to_string()), "user input can't inject FTS syntax");
    assert_eq!(Database::fts_query("%%"), None);
}

#[test]
#[ignore]
fn search_benchmark_fts_vs_like() {
    let db = Database::new_in_memory().unwrap();
    let words = ["deploy", "snapshot", "agent", "memory", "calendar", "budget", "security", "research", "design", "queue"];
    for i in 0..30_000 {
        let body = format!(
            "message {} about {} and {} with some extra words to make it realistic {}",
            i,
            words[i % words.len()],
            words[(i * 7) % words.len()],
            "lorem ".repeat(i % 20)
        );
        db.insert_message(&msg(if i % 2 == 0 { "FORGE" } else { "user" }, &body)).unwrap();
    }
    let t = std::time::Instant::now();
    for _ in 0..50 {
        db.search("snapshot budget").unwrap();
    }
    let fts = t.elapsed() / 50;
    let t = std::time::Instant::now();
    for _ in 0..50 {
        db.search("%snapshot").unwrap();
    }
    let like = t.elapsed() / 50;
    println!("30k messages: FTS {:?} per query, LIKE scan {:?} per query", fts, like);
}

#[test]
fn notifications_are_opt_in_dm_only_and_plain_text() {
    assert!(!agents::should_notify(None, 3, "FORGE", false), "off unless the owner opts in");
    assert!(!agents::should_notify(Some("off"), 3, "FORGE", false));
    assert!(agents::should_notify(Some("dms"), 3, "FORGE", false));
    assert!(!agents::should_notify(Some("dms"), 0, "FORGE", false), "#agency-hq chatter never pops up");
    assert!(!agents::should_notify(Some("dms"), 3, "user", false), "your own messages never notify you");
    assert!(!agents::should_notify(Some("dms"), 3, "FORGE", true), "no pop-ups while you're looking at the app");

    let body = agents::notification_body(
        "## Heads up\n**Build** is _green_ now.\n```rust\nfn main() {}\n```\n- next: `ship it`",
        200,
    );
    assert_eq!(body, "Heads up Build is green now. [code] next: ship it");
    assert!(agents::notification_body(&"word ".repeat(100), 40).ends_with('…'));

    let db = Database::new_in_memory().unwrap();
    assert_eq!(db.get_setting("desktop_notifications").unwrap().as_deref(), Some("off"));
}

#[test]
fn log_rotation_keeps_one_previous_file() {
    let dir = std::env::temp_dir().join(format!("hive-log-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let log = dir.join("hive.log");
    std::fs::write(&log, vec![b'x'; 100]).unwrap();
    hive_lib::rotate_log(&log, 1_000).unwrap();
    assert!(log.exists(), "small logs are left alone");

    std::fs::write(&log, vec![b'y'; 2_000]).unwrap();
    hive_lib::rotate_log(&log, 1_000).unwrap();
    assert!(!log.exists());
    assert_eq!(std::fs::read(dir.join("hive.log.1")).unwrap().len(), 2_000);

    std::fs::write(&log, vec![b'z'; 3_000]).unwrap();
    hive_lib::rotate_log(&log, 1_000).unwrap();
    assert_eq!(std::fs::read(dir.join("hive.log.1")).unwrap()[0], b'z', "only the newest rotated log is kept");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn only_safe_external_links_can_be_opened() {
    assert_eq!(agents::safe_external_url(" https://build.nvidia.com/models "), Some("https://build.nvidia.com/models".to_string()));
    assert!(agents::safe_external_url("http://example.com/a?b=c").is_some());
    assert!(agents::safe_external_url("mailto:owner@example.com").is_some());
    for bad in [
        "file:///C:/Windows/System32/calc.exe",
        "javascript:alert(1)",
        "C:\\Windows\\System32\\cmd.exe",
        "https://",
        "https:///etc/passwd",
        "https://evil.com/\nrm -rf",
        "https://exa mple.com",
        "ms-settings:privacy",
        "",
    ] {
        assert!(agents::safe_external_url(bad).is_none(), "should refuse {:?}", bad);
    }
}

fn test_state() -> hive_lib::AppState {
    let db = Database::new_in_memory().unwrap();
    let events = hive_lib::events::EventBus::new(64);
    hive_lib::AppState {
        db: db.clone(),
        events: events.clone(),
        daemon: hive_lib::daemon::DaemonManager::new(db.clone(), events.clone()),
        governance: hive_lib::governance::GovernanceEngine::new(db.clone(), events.clone()),
        deployment: hive_lib::deployment::DeploymentEngine::with_root(
            db,
            events,
            std::env::temp_dir().join(format!("hive-state-{}", Uuid::new_v4())),
        ),
        audio_muted: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
    }
}

#[test]
fn queued_tasks_use_the_owners_steps_or_a_real_planner() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let state = test_state();

    let plan = rt
        .block_on(state.queue_task(
            "Clean up attachments",
            Some("@archivist".to_string()),
            vec!["List orphaned files".into(), "  ".into(), "Delete the orphans".into()],
        ))
        .unwrap();
    assert_eq!(plan.assignee, "ARCHIVIST");
    assert_eq!(plan.steps, vec!["List orphaned files", "Delete the orphans"], "blank lines are dropped");
    let tasks = state.db.get_tasks(Some("active")).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].steps.len(), 2);
    assert!(tasks.iter().all(|t| !t.steps.iter().any(|s| s.title.contains("Scope and inspect"))));

    assert!(rt.block_on(state.queue_task("  ", None, vec!["x".into()])).is_err());
    assert!(rt.block_on(state.queue_task("x", Some("NOBODY".into()), vec!["x".into()])).is_err());

    if !EcosystemBridges::llm_configured() {
        let err = rt.block_on(state.queue_task("Plan a launch", None, vec![])).unwrap_err().to_string();
        assert!(err.contains("NVIDIA_API_KEY"), "without steps it needs the real planner, got: {}", err);
        assert_eq!(state.db.get_tasks(Some("active")).unwrap().len(), 1, "nothing canned gets queued");
    }

    let msg = hive_lib::AppState::queued_message(&plan);
    assert!(msg.contains("1. List orphaned files (~5m)") && msg.contains("@ARCHIVIST"));
}

#[test]
fn text_size_is_clamped_to_the_accessible_range() {
    use hive_lib::commands::{clamp_text_size, BASE_TEXT_SIZE};
    assert_eq!(clamp_text_size(16.0), 16.0);
    assert_eq!(clamp_text_size(4.0), 12.0);
    assert_eq!(clamp_text_size(99.0), 20.0);
    assert_eq!(clamp_text_size(f64::NAN), BASE_TEXT_SIZE);
}

#[test]
fn backup_retention_keeps_the_newest_dated_files_only() {
    let dir = std::env::temp_dir().join(format!("hive-backups-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    for day in 1..=20 {
        std::fs::write(dir.join(format!("hive_2026-10-{:02}.db", day)), b"x").unwrap();
    }
    std::fs::write(dir.join("hive_manual-copy.db"), b"keep me").unwrap();
    std::fs::write(dir.join("notes.txt"), b"keep me").unwrap();

    let removed = Database::prune_backups(&dir, 14).unwrap();
    assert_eq!(removed.len(), 6);
    assert!(!dir.join("hive_2026-10-06.db").exists());
    assert!(dir.join("hive_2026-10-07.db").exists());
    assert!(dir.join("hive_2026-10-20.db").exists());
    assert!(dir.join("hive_manual-copy.db").exists(), "files that aren't dated daily backups are never touched");
    assert!(dir.join("notes.txt").exists());
    assert!(Database::prune_backups(&dir, 14).unwrap().is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_databases_never_write_into_the_real_attachments_folder() {
    use base64::Engine;
    let db = Database::new_in_memory().unwrap();
    let meta = db
        .save_attachment("probe.txt", "text/plain", &base64::engine::general_purpose::STANDARD.encode("probe"))
        .unwrap();
    assert!(meta.file_path.starts_with(std::env::temp_dir().to_string_lossy().as_ref()));
    assert!(!meta.file_path.starts_with(Database::app_root_dir().to_string_lossy().as_ref()));
}

#[test]
fn cli_bridge_rejects_shell_metacharacters_before_spawning() {
    for bad in ["%NVIDIA_API_KEY%", "a&calc", "x|y", "^&", "\"quoted\"", "(group)", "!delayed!", "a;b", "$(x)", "`x`", "a\nb"] {
        let err = EcosystemBridges::gog_cli_exec(&["drive", "ls", bad]).unwrap_err().to_string();
        assert!(err.contains("unsafe shell character"), "{:?} should be rejected before running, got: {}", bad, err);
    }
    let err = EcosystemBridges::gog_cli_exec(&["format", "c:"]).unwrap_err().to_string();
    assert!(err.contains("allowlist"));
}

#[test]
fn oversized_attachments_are_refused_before_decoding() {
    let db = Database::new_in_memory().unwrap();
    let huge = "A".repeat((hive_lib::db::MAX_ATTACHMENT_BYTES / 3 + 10) * 4);
    let err = db.save_attachment("big.bin", "application/octet-stream", &huge).unwrap_err().to_string();
    assert!(err.contains("too large"), "got: {}", err);
    let ok = "A".repeat(4 * 1024);
    assert!(db.save_attachment("small.bin", "application/octet-stream", &ok).is_ok());
}

#[test]
fn exports_are_readable_markdown_without_deleted_messages() {
    let mut a = msg("user", "Can someone check the build?");
    a.created_at = "2026-11-01T08:00:00+00:00".to_string();
    let mut b = msg("FORGE", "On it, **green** now.");
    b.created_at = "2026-11-01T08:01:00+00:00".to_string();
    b.is_edited = true;
    let mut gone = msg("user", "oops");
    gone.is_deleted = true;
    let mut c = msg("user", "");
    c.attachments = Some(r#"[{"hash":"h","filename":"log.txt","mime_type":"text/plain","size_bytes":2048}]"#.to_string());

    let md = agents::export_markdown("agency-hq", &[a, b, gone, c]);
    assert!(md.starts_with("# agency-hq"));
    assert!(md.contains("3 messages"));
    assert!(md.contains("**Owner** · 2026-11-01 08:00"));
    assert!(md.contains("**@FORGE** · 2026-11-01 08:01 · edited"));
    assert!(md.contains("On it, **green** now."));
    assert!(md.contains("📎 log.txt (2 KB)"));
    assert!(!md.contains("oops"));
}
