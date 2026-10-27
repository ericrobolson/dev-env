use std::{fs, path::Path};
use tempfile::tempdir;
use wiki_writer::{
    pipeline::{RunOptions, run_fake},
    project,
    types::{Provider, Tier},
};

#[test]
fn default_run_ingests_and_requires_acceptance_before_any_model_call() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.md");
    fs::write(
        &source,
        "# Demo\nThe copper token weighs 12 grams and the silver token weighs 24 grams.\n",
    )
    .unwrap();
    let project_dir =
        project::init(dir.path(), "demo", &[source], Provider::Codex, "generic").unwrap();
    let options = RunOptions {
        jobs: None,
        until: None,
        only: None,
        ceiling: Tier::Large,
        yes: false,
        budget_tokens: None,
    };
    let report = run_fake(&project_dir, &options, &dir.path().join("missing-fixtures")).unwrap();
    assert!(report.needs_acceptance);
    assert_eq!(report.tokens, 0);
    assert!(project_dir.join("state/ingest/chunks.json").exists());
    assert!(!project_dir.join("state/ledger.jsonl").exists());
    let only = RunOptions {
        only: Some(wiki_writer::types::Stage::Survey),
        ..options.clone()
    };
    let only_report = run_fake(&project_dir, &only, &dir.path().join("missing-fixtures")).unwrap();
    assert!(only_report.needs_acceptance);
    assert!(!project_dir.join("state/ledger.jsonl").exists());
}

fn response(root: &Path, stage: &str, item: &str, model: &str, text: &str) {
    let path = root.join(stage).join(item).join(model).join("1.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        serde_json::json!({"response":{"text":text,"model":model,
        "input_tokens":1,"output_tokens":1,"cost_usd":null,"usage_reported":true}})
        .to_string(),
    )
    .unwrap();
}

#[test]
fn complete_two_source_fake_run_reuses_every_model_output() {
    let dir = tempdir().unwrap();
    let copper = dir.path().join("copper.md");
    let silver = dir.path().join("silver.md");
    fs::write(
        &copper,
        "# Copper\nThe copper token weighs 12 grams and is a useful example.\n",
    )
    .unwrap();
    fs::write(
        &silver,
        "# Silver\nThe silver token weighs 24 grams and is a useful example.\n",
    )
    .unwrap();
    let project_dir = project::init(
        dir.path(),
        "demo",
        &[copper, silver],
        Provider::Codex,
        "generic",
    )
    .unwrap();
    let cfg = wiki_writer::config::load(&project_dir).unwrap();
    let root = dir.path().join("fixtures");
    response(
        &root,
        "survey",
        "global",
        &cfg.models.large,
        "entity_types:\n  item:\n    fields: [weight]\nnamespaces: [items]\n",
    );
    for (index, (entity, weight)) in [("copper token", "12"), ("silver token", "24")]
        .into_iter()
        .enumerate()
    {
        let chunk = format!("c{:04}", index + 1);
        let source = format!("s{:04}", index + 1);
        let output = serde_json::json!({"facts":[{"entity":entity,"entity_type":"item","aliases":[],
            "fields":{"weight":format!("{weight} grams")},"text":format!("The {entity} weighs {weight} grams"),
            "quote":format!("The {entity} weighs {weight} grams"),"pages":[{"source":source,"page":1}],"table_like":false}]});
        response(
            &root,
            "extract",
            &chunk,
            &cfg.models.small,
            &output.to_string(),
        );
        response(
            &root,
            "audit",
            &chunk,
            &cfg.models.mid,
            r#"{"remove":[],"replace":[],"add":[],"reasons":[]}"#,
        );
    }
    let outline = "pages:\n  - id: start\n    title: Home\n    kind: index\n    entity_ids: []\n    fact_ids: []\n  - id: items:copper\n    title: Copper\n    kind: leaf\n    entity_ids: [e0001]\n    fact_ids: [f0001]\n  - id: items:silver\n    title: Silver\n    kind: leaf\n    entity_ids: [e0002]\n    fact_ids: [f0002]\ngaps: []\n";
    response(&root, "organize", "global", &cfg.models.large, outline);
    response(
        &root,
        "write",
        "start",
        &cfg.models.mid,
        "# Home\n[Copper](items:copper) and [Silver](items:silver).\n",
    );
    response(
        &root,
        "write",
        "items:copper",
        &cfg.models.small,
        "# Copper\nThe copper token weighs 12 grams [^f0001].\n",
    );
    response(
        &root,
        "write",
        "items:silver",
        &cfg.models.small,
        "# Silver\nThe silver token weighs 24 grams [^f0002].\n",
    );
    response(
        &root,
        "review",
        "global",
        &cfg.models.large,
        r#"{"findings":[]}"#,
    );
    let options = RunOptions {
        jobs: Some(2),
        until: None,
        only: None,
        ceiling: Tier::Large,
        yes: true,
        budget_tokens: Some(10_000),
    };
    let first = run_fake(&project_dir, &options, &root).unwrap();
    assert_eq!(first.blocked, 0);
    assert!(project_dir.join("state/review.json").exists());
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wiki-writer"))
        .arg("status")
        .arg("--project")
        .arg(&project_dir)
        .output()
        .unwrap();
    assert!(status.status.success());
    let printed = String::from_utf8_lossy(&status.stdout);
    assert!(printed.contains("link 1 0 0"), "{printed}");
    let review_record_path = project_dir.join("state/items/review/global.json");
    let review_record_bytes = fs::read(&review_record_path).unwrap();
    fs::remove_file(&review_record_path).unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wiki-writer"))
        .arg("status")
        .arg("--project")
        .arg(&project_dir)
        .output()
        .unwrap();
    let printed = String::from_utf8_lossy(&status.stdout);
    assert!(printed.contains("review 0 1 0"), "{printed}");
    fs::write(&review_record_path, review_record_bytes).unwrap();
    let link_path = project_dir.join("state/link.json");
    let link_bytes = fs::read(&link_path).unwrap();
    fs::remove_file(&link_path).unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wiki-writer"))
        .arg("status")
        .arg("--project")
        .arg(&project_dir)
        .output()
        .unwrap();
    let printed = String::from_utf8_lossy(&status.stdout);
    assert!(printed.contains("link 0 1 0"), "{printed}");
    fs::write(&link_path, link_bytes).unwrap();
    let stale_record = project_dir.join("state/items/audit/old.json");
    fs::write(&stale_record, r#"{"input_hash":"old","output_hash":null,"output_path":null,"outputs":{},"status":"blocked","errors":["old"],"escalated":false}"#).unwrap();
    let only_link = RunOptions {
        only: Some(wiki_writer::types::Stage::Link),
        ..options.clone()
    };
    let link_report = run_fake(&project_dir, &only_link, &root).unwrap();
    assert_eq!((link_report.pending, link_report.blocked), (0, 0));
    let full_report = run_fake(&project_dir, &options, &root).unwrap();
    assert_eq!((full_report.pending, full_report.blocked), (0, 0));
    fs::remove_file(stale_record).unwrap();
    let ledger = fs::read(project_dir.join("state/ledger.jsonl")).unwrap();
    let entities = fs::read(project_dir.join("state/entities.json")).unwrap();
    let outline_bytes = fs::read(project_dir.join("state/outline.yaml")).unwrap();
    fs::remove_dir_all(root).unwrap();
    let second = run_fake(&project_dir, &options, &dir.path().join("fixtures")).unwrap();
    assert_eq!(second.tokens, 0);
    assert_eq!(
        fs::read(project_dir.join("state/ledger.jsonl")).unwrap(),
        ledger
    );
    assert_eq!(
        fs::read(project_dir.join("state/entities.json")).unwrap(),
        entities
    );
    assert_eq!(
        fs::read(project_dir.join("state/outline.yaml")).unwrap(),
        outline_bytes
    );
    fs::write(
        project_dir.join("state/redo_queue.json"),
        r#"[{"stage":"write","item":"items:copper","reason":"redo detail"}]"#,
    )
    .unwrap();
    response(
        &dir.path().join("fixtures"),
        "write",
        "items:copper",
        &cfg.models.small,
        "# Copper\nThe copper token weighs 12 grams [^f0001].\n",
    );
    let retry_fixture = dir
        .path()
        .join("fixtures/write/items:copper")
        .join(&cfg.models.small)
        .join("1.json");
    let second_fixture = retry_fixture.with_file_name("2.json");
    fs::rename(&retry_fixture, &second_fixture).unwrap();
    let third = run_fake(&project_dir, &options, &dir.path().join("fixtures")).unwrap();
    assert_eq!(third.blocked, 0);
    let new_ledger = fs::read_to_string(project_dir.join("state/ledger.jsonl")).unwrap();
    assert_eq!(
        new_ledger.lines().count(),
        String::from_utf8(ledger).unwrap().lines().count() + 1
    );
    let queue: Vec<serde_json::Value> =
        serde_json::from_slice(&fs::read(project_dir.join("state/redo_queue.json")).unwrap())
            .unwrap();
    assert!(queue.is_empty());
    fs::write(
        project_dir.join("state/redo_queue.json"),
        r#"[{"stage":"write","item":"items:missing","reason":"stale"}]"#,
    )
    .unwrap();
    let stale_redo = run_fake(&project_dir, &options, &dir.path().join("fixtures")).unwrap();
    assert_eq!((stale_redo.pending, stale_redo.blocked), (0, 0));
    let queue: Vec<serde_json::Value> =
        serde_json::from_slice(&fs::read(project_dir.join("state/redo_queue.json")).unwrap())
            .unwrap();
    assert!(queue.is_empty());
    let forced_path = project_dir.join("state/forced.json");
    let original_forced = fs::read(&forced_path).ok();
    fs::write(&forced_path, b"{broken").unwrap();
    fs::write(
        project_dir.join("state/redo_queue.json"),
        r#"[{"stage":"write","item":"items:copper","reason":"redo"}]"#,
    )
    .unwrap();
    assert!(run_fake(&project_dir, &options, &dir.path().join("fixtures")).is_err());
    assert_eq!(fs::read(&forced_path).unwrap(), b"{broken");
    if let Some(original_forced) = original_forced {
        fs::write(&forced_path, original_forced).unwrap();
    } else {
        fs::remove_file(&forced_path).unwrap();
    }
    fs::write(project_dir.join("state/redo_queue.json"), b"[]").unwrap();
    let extract_record = project_dir.join("state/items/extract/c0001.json");
    let note_before = fs::read(project_dir.join("state/notes/c0001.json")).unwrap();
    let registry_before = fs::read(project_dir.join("state/fact_ids.json")).unwrap();
    fs::remove_file(&extract_record).unwrap();
    let output = serde_json::json!({"facts":[{"entity":"copper token","entity_type":"item","aliases":[],
        "fields":{"weight":"12 grams"},"text":"The copper token weighs 12 grams",
        "quote":"The copper token weighs 12 grams","pages":[{"source":"s0001","page":1}],"table_like":false}]});
    response(
        &dir.path().join("fixtures"),
        "extract",
        "c0001",
        &cfg.models.small,
        &output.to_string(),
    );
    let fixture = dir
        .path()
        .join("fixtures/extract/c0001")
        .join(&cfg.models.small)
        .join("1.json");
    fs::rename(&fixture, fixture.with_file_name("2.json")).unwrap();
    let only_extract = RunOptions {
        only: Some(wiki_writer::types::Stage::Extract),
        ..options.clone()
    };
    let recovered = run_fake(&project_dir, &only_extract, &dir.path().join("fixtures")).unwrap();
    assert_eq!((recovered.pending, recovered.blocked), (0, 0));
    assert!(extract_record.exists());
    assert_eq!(
        fs::read(project_dir.join("state/notes/c0001.json")).unwrap(),
        note_before
    );
    assert_eq!(
        fs::read(project_dir.join("state/fact_ids.json")).unwrap(),
        registry_before
    );
    let effective_path = project_dir.join("state/effective/c0001.json");
    let effective_bytes = fs::read(&effective_path).unwrap();
    fs::remove_file(&effective_path).unwrap();
    let only_resolve = RunOptions {
        only: Some(wiki_writer::types::Stage::Resolve),
        ..options.clone()
    };
    let error = run_fake(&project_dir, &only_resolve, &dir.path().join("fixtures")).unwrap_err();
    assert!(error.to_string().contains("effective"), "{error:#}");
    fs::write(&effective_path, effective_bytes).unwrap();
    let audit_record = fs::read_dir(project_dir.join("state/items/audit"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().and_then(|s| s.to_str()) == Some("json"))
        .unwrap();
    let audit_bytes = fs::read(&audit_record).unwrap();
    fs::remove_file(&audit_record).unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wiki-writer"))
        .arg("status")
        .arg("--project")
        .arg(&project_dir)
        .output()
        .unwrap();
    let printed = String::from_utf8_lossy(&status.stdout);
    assert!(printed.contains("audit 0 1 0"), "{printed}");
    let error = run_fake(&project_dir, &only_resolve, &dir.path().join("fixtures")).unwrap_err();
    assert!(error.to_string().contains("audit"), "{error:#}");
    fs::write(&audit_record, audit_bytes).unwrap();
    let resolve_record_path = project_dir.join("state/items/resolve/global.json");
    let resolve_record_bytes = fs::read(&resolve_record_path).unwrap();
    let mut resolve_record: serde_json::Value =
        serde_json::from_slice(&resolve_record_bytes).unwrap();
    resolve_record["status"] = serde_json::json!("pending");
    fs::write(
        &resolve_record_path,
        serde_json::to_vec(&resolve_record).unwrap(),
    )
    .unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wiki-writer"))
        .arg("status")
        .arg("--project")
        .arg(&project_dir)
        .output()
        .unwrap();
    let printed = String::from_utf8_lossy(&status.stdout);
    assert!(printed.contains("resolve 0 1 0"), "{printed}");
    let only_organize = RunOptions {
        only: Some(wiki_writer::types::Stage::Organize),
        ..options.clone()
    };
    let error = run_fake(&project_dir, &only_organize, &dir.path().join("fixtures")).unwrap_err();
    assert!(error.to_string().contains("resolve"), "{error:#}");
    fs::write(&resolve_record_path, resolve_record_bytes).unwrap();
    let organize_record_path = project_dir.join("state/items/organize/global.json");
    let organize_record_bytes = fs::read(&organize_record_path).unwrap();
    let mut organize_record: serde_json::Value =
        serde_json::from_slice(&organize_record_bytes).unwrap();
    organize_record["status"] = serde_json::json!("pending");
    fs::write(
        &organize_record_path,
        serde_json::to_vec(&organize_record).unwrap(),
    )
    .unwrap();
    let only_write = RunOptions {
        only: Some(wiki_writer::types::Stage::Write),
        ..options.clone()
    };
    let error = run_fake(&project_dir, &only_write, &dir.path().join("fixtures")).unwrap_err();
    assert!(error.to_string().contains("organize"), "{error:#}");
    fs::write(&organize_record_path, organize_record_bytes).unwrap();
    let note_path = project_dir.join("state/notes/c0001.json");
    let note_bytes = fs::read(&note_path).unwrap();
    fs::remove_file(&note_path).unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wiki-writer"))
        .arg("status")
        .arg("--project")
        .arg(&project_dir)
        .output()
        .unwrap();
    assert!(status.status.success());
    let printed = String::from_utf8_lossy(&status.stdout);
    assert!(printed.contains("extract 1 1 0"), "{printed}");
    let only_audit = RunOptions {
        only: Some(wiki_writer::types::Stage::Audit),
        ..options.clone()
    };
    let error = run_fake(&project_dir, &only_audit, &dir.path().join("fixtures")).unwrap_err();
    assert!(error.to_string().contains("extract"), "{error:#}");
    fs::write(&note_path, note_bytes).unwrap();
    let config_path = project_dir.join("wikify.yaml");
    let original_config = fs::read_to_string(&config_path).unwrap();
    fs::write(
        &config_path,
        original_config.replace(&cfg.models.large, "changed-large-model"),
    )
    .unwrap();
    let only_extract = RunOptions {
        only: Some(wiki_writer::types::Stage::Extract),
        ..options.clone()
    };
    let error = run_fake(&project_dir, &only_extract, &dir.path().join("fixtures")).unwrap_err();
    assert!(error.to_string().contains("survey"), "{error:#}");
    fs::write(&config_path, original_config).unwrap();
    fs::write(
        project_dir.join("sources/s0001.md"),
        "# Copper\nThe copper token now weighs 13 grams and is a useful example.\n",
    )
    .unwrap();
    let only_survey = RunOptions {
        only: Some(wiki_writer::types::Stage::Survey),
        ..options
    };
    let error = run_fake(&project_dir, &only_survey, &dir.path().join("fixtures")).unwrap_err();
    assert!(error.to_string().contains("ingest"), "{error:#}");
}
