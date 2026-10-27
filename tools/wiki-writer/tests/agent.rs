use std::{fs, os::unix::fs::PermissionsExt, time::Duration};
use tempfile::tempdir;
use wiki_writer::{
    agent::{Agent, CallContext, FailureKind},
    config::Models,
    scheduler::{Job, JobOutcome, ScheduleOptions, ladder, run_jobs},
    types::{Provider, Stage, Tier},
};

fn context(attempt: u32) -> CallContext {
    CallContext {
        stage: Stage::Extract,
        item: "c0001".into(),
        attempt,
    }
}

fn fixture(root: &std::path::Path, item: &str, model: &str, attempt: u32, body: &str) {
    let path = root
        .join("extract")
        .join(item)
        .join(model)
        .join(format!("{attempt}.json"));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

fn options(root: &std::path::Path) -> ScheduleOptions {
    ScheduleOptions {
        provider: Provider::Fake,
        fake_root: Some(root.to_path_buf()),
        models: Models {
            small: "s".into(),
            mid: "m".into(),
            large: "l".into(),
        },
        ceiling: Tier::Large,
        budget_tokens: 1000,
        jobs: 2,
        timeout: Duration::from_secs(1),
        run: "run-1".into(),
    }
}

#[test]
fn scheduler_repairs_validation_then_escalates_and_records_every_attempt() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("fixtures");
    for (attempt, model, text) in [(1, "s", "bad"), (2, "s", "still bad"), (3, "m", "good")] {
        fixture(
            &root,
            "c0001",
            model,
            attempt,
            &format!(
                r#"{{"response":{{"text":"{text}","model":"{model}","input_tokens":2,"output_tokens":1,"cost_usd":null,"usage_reported":true}}}}"#
            ),
        );
    }
    let job = Job {
        stage: Stage::Extract,
        item: "c0001".into(),
        start: Tier::Small,
        prompt: "evidence".into(),
    };
    let report = run_jobs(
        dir.path(),
        vec![job],
        &options(&root),
        &|_, text| {
            if text == "good" {
                Ok(())
            } else {
                Err("needs good".into())
            }
        },
        &|_| {},
    )
    .unwrap();
    assert_eq!(
        report.outcomes[&(Stage::Extract, "c0001".into())],
        JobOutcome::Done("good".into())
    );
    assert_eq!(report.charged_tokens, 9);
    let ledger = fs::read_to_string(dir.path().join("state/ledger.jsonl")).unwrap();
    let rows: Vec<serde_json::Value> = ledger
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows.iter()
            .map(|row| row["attempt"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(
        rows.iter()
            .map(|row| row["model"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["s", "s", "m"]
    );
    assert_eq!(
        rows.iter()
            .map(|row| row["outcome"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["bad_output", "bad_output", "ok"]
    );
}

#[test]
fn scheduler_retries_rate_limits_and_shrinks_capacity() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("fixtures");
    for attempt in 1..=2 {
        fixture(
            &root,
            "c0001",
            "s",
            attempt,
            r#"{"error":{"kind":"rate_limit","message":"429","input_tokens":0,"output_tokens":0,"cost_usd":null,"usage_reported":true}}"#,
        );
    }
    fixture(
        &root,
        "c0001",
        "s",
        3,
        r#"{"response":{"text":"ok","model":"s","input_tokens":1,"output_tokens":1,"cost_usd":null,"usage_reported":true}}"#,
    );
    let delays = std::sync::Mutex::new(Vec::new());
    let job = Job {
        stage: Stage::Extract,
        item: "c0001".into(),
        start: Tier::Small,
        prompt: "p".into(),
    };
    let report = run_jobs(
        dir.path(),
        vec![job],
        &options(&root),
        &|_, _| Ok(()),
        &|delay| {
            delays.lock().unwrap().push(delay);
        },
    )
    .unwrap();
    assert_eq!(report.rate_limits, 2);
    assert_eq!(report.final_capacity, 1);
    assert_eq!(
        report.outcomes[&(Stage::Extract, "c0001".into())],
        JobOutcome::Done("ok".into())
    );
    let delays = delays.lock().unwrap();
    assert_eq!(delays.len(), 2);
    assert!(delays[0] > Duration::from_millis(900) && delays[0] <= Duration::from_secs(1));
    assert!(delays[1] > Duration::from_millis(1900) && delays[1] <= Duration::from_secs(2));
}

#[test]
fn rate_limited_job_does_not_block_other_ready_work() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("fixtures");
    fixture(
        &root,
        "c0001",
        "s",
        1,
        r#"{"error":{"kind":"rate_limit","message":"429","input_tokens":0,"output_tokens":0,"cost_usd":null,"usage_reported":true}}"#,
    );
    for (item, attempt) in [("c0001", 2), ("c0002", 1)] {
        fixture(
            &root,
            item,
            "s",
            attempt,
            r#"{"response":{"text":"ok","model":"s","input_tokens":1,"output_tokens":1,"cost_usd":null,"usage_reported":true}}"#,
        );
    }
    let jobs = ["c0001", "c0002"].map(|item| Job {
        stage: Stage::Extract,
        item: item.into(),
        start: Tier::Small,
        prompt: "p".into(),
    });
    let mut options = options(&root);
    options.jobs = 1;
    let report = run_jobs(dir.path(), jobs.into(), &options, &|_, _| Ok(()), &|_| {}).unwrap();
    assert_eq!(report.outcomes.len(), 2);
    let rows: Vec<serde_json::Value> = fs::read_to_string(dir.path().join("state/ledger.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        rows.iter()
            .map(|row| row["item"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["c0001", "c0002", "c0001"]
    );
}

#[test]
fn process_errors_retry_same_tier_then_skip_validation_repair_slot() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("fixtures");
    for attempt in 1..=2 {
        fixture(
            &root,
            "c0001",
            "s",
            attempt,
            r#"{"error":{"kind":"other","message":"process exited","input_tokens":0,"output_tokens":0,"cost_usd":null,"usage_reported":true}}"#,
        );
    }
    fixture(
        &root,
        "c0001",
        "m",
        3,
        r#"{"response":{"text":"ok","model":"m","input_tokens":1,"output_tokens":1,"cost_usd":null,"usage_reported":true}}"#,
    );
    let job = Job {
        stage: Stage::Extract,
        item: "c0001".into(),
        start: Tier::Small,
        prompt: "p".into(),
    };
    let report = run_jobs(
        dir.path(),
        vec![job],
        &options(&root),
        &|_, _| Ok(()),
        &|_| {},
    )
    .unwrap();
    assert_eq!(
        report.outcomes[&(Stage::Extract, "c0001".into())],
        JobOutcome::Done("ok".into())
    );
    let ledger = fs::read_to_string(dir.path().join("state/ledger.jsonl")).unwrap();
    let models: Vec<String> = ledger
        .lines()
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line).unwrap()["model"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(models, ["s", "s", "m"]);
}

#[test]
fn rate_exhaustion_stays_pending_and_resumed_attempts_continue() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("fixtures");
    for attempt in 1..=7 {
        fixture(
            &root,
            "c0001",
            "s",
            attempt,
            r#"{"error":{"kind":"rate_limit","message":"429","input_tokens":0,"output_tokens":0,"cost_usd":null,"usage_reported":true}}"#,
        );
    }
    let job = Job {
        stage: Stage::Extract,
        item: "c0001".into(),
        start: Tier::Small,
        prompt: "p".into(),
    };
    let report = run_jobs(
        dir.path(),
        vec![job.clone()],
        &options(&root),
        &|_, _| Ok(()),
        &|_| {},
    )
    .unwrap();
    assert_eq!(
        report.outcomes[&(Stage::Extract, "c0001".into())],
        JobOutcome::Pending
    );
    let ledger = fs::read_to_string(dir.path().join("state/ledger.jsonl")).unwrap();
    assert_eq!(ledger.lines().count(), 7);
    fixture(
        &root,
        "c0001",
        "s",
        8,
        r#"{"response":{"text":"ok","model":"s","input_tokens":1,"output_tokens":0,"cost_usd":null,"usage_reported":true}}"#,
    );
    let report = run_jobs(
        dir.path(),
        vec![job],
        &options(&root),
        &|_, _| Ok(()),
        &|_| {},
    )
    .unwrap();
    assert_eq!(
        report.outcomes[&(Stage::Extract, "c0001".into())],
        JobOutcome::Done("ok".into())
    );
    let ledger = fs::read_to_string(dir.path().join("state/ledger.jsonl")).unwrap();
    assert_eq!(ledger.lines().count(), 8);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(ledger.lines().last().unwrap()).unwrap()["attempt"],
        8
    );
}

#[test]
fn ledger_recovery_truncates_only_an_incomplete_final_line() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("fixtures");
    let ledger = dir.path().join("state/ledger.jsonl");
    fs::create_dir_all(ledger.parent().unwrap()).unwrap();
    let first = serde_json::json!({"run":"prior","stage":"extract","item":"c0001",
        "model":"s","attempt":1,"input_tokens":1,"output_tokens":1,"cost_usd":null,
        "outcome":"ok","duration_ms":1,"estimated_usage":false})
    .to_string();
    fs::write(&ledger, format!("{first}\n{{\"run\":\"interrupted\"")).unwrap();
    fixture(
        &root,
        "c0001",
        "s",
        2,
        r#"{"response":{"text":"ok","model":"s","input_tokens":1,"output_tokens":1,"cost_usd":null,"usage_reported":true}}"#,
    );
    let job = Job {
        stage: Stage::Extract,
        item: "c0001".into(),
        start: Tier::Small,
        prompt: "p".into(),
    };
    let report = run_jobs(
        dir.path(),
        vec![job.clone()],
        &options(&root),
        &|_, _| Ok(()),
        &|_| {},
    )
    .unwrap();
    assert_eq!(
        report.outcomes[&(Stage::Extract, "c0001".into())],
        JobOutcome::Done("ok".into())
    );
    let lines = fs::read_to_string(&ledger).unwrap();
    assert_eq!(lines.lines().count(), 2);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(lines.lines().last().unwrap()).unwrap()["attempt"],
        2
    );
    let malformed = format!("{first}\ninvalid complete line\n");
    fs::write(&ledger, &malformed).unwrap();
    let error = run_jobs(
        dir.path(),
        vec![job],
        &options(&root),
        &|_, _| Ok(()),
        &|_| {},
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("invalid ledger line"),
        "{error:#}"
    );
    assert_eq!(fs::read_to_string(&ledger).unwrap(), malformed);
}

#[test]
fn zero_budget_dispatches_nothing_and_missing_usage_is_estimated() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("fixtures");
    let job = Job {
        stage: Stage::Extract,
        item: "c0001".into(),
        start: Tier::Small,
        prompt: "12345678".into(),
    };
    let mut opts = options(&root);
    opts.budget_tokens = 0;
    let report = run_jobs(
        dir.path(),
        vec![job.clone()],
        &opts,
        &|_, _| Ok(()),
        &|_| {},
    )
    .unwrap();
    assert_eq!(
        report.outcomes[&(Stage::Extract, "c0001".into())],
        JobOutcome::Pending
    );
    assert!(!dir.path().join("state/ledger.jsonl").exists());
    opts.budget_tokens = 100;
    fixture(
        &root,
        "c0001",
        "s",
        1,
        r#"{"response":{"text":"abcd","model":"s","input_tokens":0,"output_tokens":0,"cost_usd":null,"usage_reported":false}}"#,
    );
    let report = run_jobs(dir.path(), vec![job], &opts, &|_, _| Ok(()), &|_| {}).unwrap();
    assert_eq!(report.charged_tokens, 3);
    let row: serde_json::Value = serde_json::from_str(
        fs::read_to_string(dir.path().join("state/ledger.jsonl"))
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(row["estimated_usage"], true);
    assert_eq!(row["input_tokens"], 2);
    assert_eq!(row["output_tokens"], 1);
}

#[test]
fn bad_output_uses_validation_ladder_and_ceiling_blocks() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("fixtures");
    for attempt in 1..=2 {
        fixture(
            &root,
            "c0001",
            "s",
            attempt,
            r#"{"error":{"kind":"bad_output","message":"invalid JSON","input_tokens":0,"output_tokens":0,"cost_usd":null,"usage_reported":false}}"#,
        );
    }
    let job = Job {
        stage: Stage::Extract,
        item: "c0001".into(),
        start: Tier::Small,
        prompt: "p".into(),
    };
    let mut opts = options(&root);
    opts.ceiling = Tier::Small;
    let report = run_jobs(dir.path(), vec![job], &opts, &|_, _| Ok(()), &|_| {}).unwrap();
    assert!(matches!(
        report.outcomes[&(Stage::Extract, "c0001".into())],
        JobOutcome::Blocked(_)
    ));
    let ledger = fs::read_to_string(dir.path().join("state/ledger.jsonl")).unwrap();
    assert_eq!(ledger.lines().count(), 2);
    assert!(
        ledger
            .lines()
            .all(|line| serde_json::from_str::<serde_json::Value>(line).unwrap()["model"] == "s")
    );
}

#[test]
fn fake_uses_per_attempt_fixtures_and_never_falls_through() {
    let dir = tempdir().unwrap();
    let fixture = dir.path().join("extract/c0001/test-model");
    fs::create_dir_all(&fixture).unwrap();
    fs::write(fixture.join("1.json"), r#"{"error":{"kind":"bad_output","message":"invalid schema","input_tokens":2,"output_tokens":1,"cost_usd":null,"usage_reported":true}}"#).unwrap();
    fs::write(fixture.join("2.json"), r#"{"response":{"text":"valid","model":"test-model","input_tokens":3,"output_tokens":2,"cost_usd":null,"usage_reported":true}}"#).unwrap();
    let first = Agent::fake(dir.path().to_path_buf(), context(1))
        .ask("test-model", "prompt")
        .unwrap_err();
    assert_eq!(first.kind, FailureKind::BadOutput);
    assert_eq!(first.input_tokens, 2);
    let second = Agent::fake(dir.path().to_path_buf(), context(2))
        .ask("test-model", "prompt")
        .unwrap();
    assert_eq!(second.text, "valid");
    assert_eq!(second.output_tokens, 2);
    let missing = Agent::fake(dir.path().to_path_buf(), context(3))
        .ask("test-model", "prompt")
        .unwrap_err();
    assert!(missing.message.contains("fixture"));
}

#[test]
fn ladder_respects_stage_start_and_ceiling() {
    assert_eq!(
        ladder(Tier::Small, Tier::Large),
        vec![Tier::Small, Tier::Small, Tier::Mid, Tier::Large]
    );
    assert_eq!(ladder(Tier::Mid, Tier::Large), vec![Tier::Mid, Tier::Large]);
    assert_eq!(ladder(Tier::Large, Tier::Large), vec![Tier::Large]);
    assert_eq!(
        ladder(Tier::Small, Tier::Small),
        vec![Tier::Small, Tier::Small]
    );
    assert_eq!(
        ladder(Tier::Mid, Tier::Small),
        vec![Tier::Small, Tier::Small]
    );
}

#[test]
fn fake_usage_flag_distinguishes_zero_from_missing() {
    let dir = tempdir().unwrap();
    let fixture = dir.path().join("extract/c0001/m/1.json");
    fs::create_dir_all(fixture.parent().unwrap()).unwrap();
    fs::write(&fixture, r#"{"response":{"text":"","model":"m","input_tokens":0,"output_tokens":0,"cost_usd":null,"usage_reported":false}}"#).unwrap();
    let response = Agent::fake(dir.path().to_path_buf(), context(1))
        .ask("m", "prompt")
        .unwrap();
    assert!(!response.usage_reported);
    assert_eq!(response.input_tokens, 0);
    let _ = Agent::new(Provider::Codex, Duration::from_millis(1), context(1));
}

#[test]
fn real_adapters_stream_json_and_send_prompt_on_stdin() {
    let dir = tempdir().unwrap();
    let bin = dir.path().join("bin");
    fs::create_dir(&bin).unwrap();
    for (name, stream) in [
        ("claude", include_str!("fixtures/agent/claude-result.jsonl")),
        ("codex", include_str!("fixtures/agent/codex-result.jsonl")),
    ] {
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CAPTURE_DIR/{name}.args\"\ncat > \"$CAPTURE_DIR/{name}.stdin\"\ncat <<'JSON'\n{stream}\nJSON\n"
        );
        let path = bin.join(name);
        fs::write(&path, script).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let original_path = std::env::var_os("PATH").unwrap_or_default();
    unsafe {
        std::env::set_var(
            "PATH",
            format!("{}:{}", bin.display(), original_path.to_string_lossy()),
        );
        std::env::set_var("CAPTURE_DIR", dir.path());
    }
    for provider in [Provider::Claude, Provider::Codex] {
        let response = Agent::new(provider, Duration::from_secs(2), context(1))
            .ask("model-id", "input $() `quoted`")
            .unwrap();
        assert_eq!(response.text, "answer");
        assert_eq!(response.output_tokens, 5);
        assert_eq!(
            response.input_tokens,
            if provider == Provider::Claude { 9 } else { 8 }
        );
        let name = if provider == Provider::Claude {
            "claude"
        } else {
            "codex"
        };
        assert_eq!(
            fs::read_to_string(dir.path().join(format!("{name}.stdin"))).unwrap(),
            "input $() `quoted`"
        );
        let args = fs::read_to_string(dir.path().join(format!("{name}.args"))).unwrap();
        assert!(args.lines().any(|a| a == "model-id"));
        assert!(!args.contains("input $()"));
        if provider == Provider::Claude {
            assert!(args.contains("--tools\n\n"));
        } else {
            assert!(args.contains("--sandbox\nread-only\n"));
            assert!(args.contains("--disable\nshell_tool\n"));
        }
    }
    let claude = bin.join("claude");
    fs::write(&claude, "#!/bin/sh\necho '{\"type\":\"result\",\"result\":\"rate limit 429\",\"is_error\":true,\"usage\":{\"input_tokens\":7,\"output_tokens\":2}}'\n").unwrap();
    let failure = Agent::new(Provider::Claude, Duration::from_secs(2), context(2))
        .ask("m", "p")
        .unwrap_err();
    assert_eq!(failure.kind, FailureKind::RateLimit);
    assert_eq!(failure.input_tokens, 7);
    fs::write(
        &claude,
        "#!/bin/sh\necho '{\"type\":\"tool_call\"}'\nsleep 5\n",
    )
    .unwrap();
    let started = std::time::Instant::now();
    let failure = Agent::new(Provider::Claude, Duration::from_secs(2), context(3))
        .ask("m", "p")
        .unwrap_err();
    assert_eq!(failure.kind, FailureKind::BadOutput);
    assert!(started.elapsed() < Duration::from_secs(2));
    fs::write(&claude, "#!/bin/sh\nsleep 5\n").unwrap();
    let failure = Agent::new(Provider::Claude, Duration::from_millis(60), context(4))
        .ask("m", "p")
        .unwrap_err();
    assert!(failure.message.contains("timeout"));
    fs::write(&claude, "#!/bin/sh\necho 'fatal' >&2\nexit 4\n").unwrap();
    let failure = Agent::new(Provider::Claude, Duration::from_secs(2), context(5))
        .ask("m", "p")
        .unwrap_err();
    assert!(failure.message.contains("exited with"));
    assert!(failure.message.contains("fatal"));
    fs::write(&claude, "#!/bin/sh\necho '{broken json'\n").unwrap();
    let failure = Agent::new(Provider::Claude, Duration::from_secs(2), context(6))
        .ask("m", "p")
        .unwrap_err();
    assert_eq!(failure.kind, FailureKind::BadOutput);
    fs::write(
        &claude,
        "#!/bin/sh\nhead -c 8500000 /dev/zero | tr '\\000' x\n",
    )
    .unwrap();
    let failure = Agent::new(Provider::Claude, Duration::from_secs(2), context(7))
        .ask("m", "p")
        .unwrap_err();
    assert_eq!(failure.kind, FailureKind::BadOutput);
    assert!(failure.message.contains("8 MiB"));
    let codex = bin.join("codex");
    fs::write(&codex, "#!/bin/sh\necho '{\"type\":\"item.completed\",\"item\":{\"type\":\"web_search_call\"}}'\necho '{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}'\n").unwrap();
    let failure = Agent::new(Provider::Codex, Duration::from_secs(2), context(8))
        .ask("m", "p")
        .unwrap_err();
    assert_eq!(failure.kind, FailureKind::BadOutput);
    assert!(failure.message.contains("tool event"));
    fs::write(&claude, "#!/bin/sh\necho '{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"id\":\"call1\"}]}}'\necho '{\"type\":\"result\",\"result\":\"ok\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}'\n").unwrap();
    let failure = Agent::new(Provider::Claude, Duration::from_secs(2), context(9))
        .ask("m", "p")
        .unwrap_err();
    assert_eq!(failure.kind, FailureKind::BadOutput);
    assert!(failure.message.contains("tool event"));
    fs::write(&claude, "#!/bin/sh\nsleep 3 &\necho '{\"type\":\"result\",\"result\":\"ok\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}'\n").unwrap();
    let started = std::time::Instant::now();
    Agent::new(Provider::Claude, Duration::from_secs(1), context(10))
        .ask("m", "p")
        .unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    unsafe {
        std::env::set_var("PATH", original_path);
        std::env::remove_var("CAPTURE_DIR");
    }
}
