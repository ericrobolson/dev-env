use sha2::{Digest, Sha256};
use std::os::unix::fs::symlink;
use std::{collections::BTreeMap, fs};
use tempfile::tempdir;
use wiki_writer::{
    config, state,
    types::{ItemRecord, ItemStatus, Tier},
};

#[test]
fn atomic_write_preserves_complete_prior_file_on_failure() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("value.json");
    state::atomic_write(&path, b"old").unwrap();
    // A failure before rename must leave the previous complete file in place.
    assert!(state::atomic_write(&dir.path().join("missing/value.json"), b"new").is_err());
    assert_eq!(fs::read(path).unwrap(), b"old");
}

#[test]
fn cache_requires_matching_hash_and_every_output() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("state")).unwrap();
    fs::write(dir.path().join("state/one.json"), b"one").unwrap();
    fs::write(dir.path().join("state/two.json"), b"two").unwrap();
    let mut outputs = BTreeMap::new();
    outputs.insert("state/one.json".to_owned(), hex(b"one"));
    outputs.insert("state/two.json".to_owned(), hex(b"two"));
    let record = ItemRecord {
        input_hash: "input".into(),
        output_hash: None,
        output_path: None,
        outputs,
        status: ItemStatus::Done,
        errors: vec![],
        escalated: false,
    };
    assert!(state::is_done(dir.path(), &record, "input"));
    assert!(!state::is_done(dir.path(), &record, "changed"));
    fs::write(dir.path().join("state/two.json"), b"corrupt").unwrap();
    assert!(!state::is_done(dir.path(), &record, "input"));
    fs::remove_file(dir.path().join("state/one.json")).unwrap();
    assert!(!state::is_done(dir.path(), &record, "input"));
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn semantic_hash_uses_version_inputs_and_ceiling() {
    let initial = state::input_hash("extract", 1, b"same inputs", Tier::Small);
    assert_eq!(
        initial,
        state::input_hash("extract", 1, b"same inputs", Tier::Small)
    );
    assert_ne!(
        initial,
        state::input_hash("extract", 2, b"same inputs", Tier::Small)
    );
    assert_ne!(
        initial,
        state::input_hash("extract", 1, b"upstream changed", Tier::Small)
    );
    assert_ne!(
        initial,
        state::input_hash("extract", 1, b"same inputs", Tier::Mid)
    );
    assert_ne!(
        initial,
        state::input_hash("extract", 1, b"same inputs/model override", Tier::Small)
    );
}

#[test]
fn process_lock_is_exclusive_and_released_on_drop() {
    let dir = tempdir().unwrap();
    let first = state::lock(dir.path()).unwrap();
    assert!(state::lock(dir.path()).is_err());
    drop(first);
    let _second = state::lock(dir.path()).unwrap();
}

#[test]
fn project_lock_refuses_symlinked_state_directory() {
    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    symlink(outside.path(), dir.path().join("state")).unwrap();
    assert!(state::lock(dir.path()).is_err());
    assert!(!outside.path().join("lock").exists());
}

#[test]
fn config_defaults_overrides_and_rejects_invalid_values() {
    let dir = tempdir().unwrap();
    let base = "version: 1\nproject: demo\nagent: codex\nsources: []\n";
    fs::write(dir.path().join("wikify.yaml"), base).unwrap();
    let cfg = config::load(dir.path()).unwrap();
    assert_eq!(cfg.jobs, 4);
    assert_eq!(cfg.audit_sample_rate, 0.10);
    assert_eq!(cfg.models.small, "gpt-6-luna");
    fs::write(
        dir.path().join("wikify.yaml"),
        format!("{base}jobs: 2\nmodels: {{small: custom}}\n"),
    )
    .unwrap();
    let cfg = config::load(dir.path()).unwrap();
    assert_eq!(cfg.jobs, 2);
    assert_eq!(cfg.models.small, "custom");
    assert_eq!(cfg.models.mid, "gpt-6.1-sol");
    for bad in [
        "agent: fake",
        "profile: invalid",
        "jobs: 0",
        "audit_sample_rate: 1.1",
        "audit_failure_threshold: .nan",
        "timeout_seconds: 0",
        "unexpected: value",
    ] {
        let yaml = if bad.starts_with("agent:") {
            base.replace("agent: codex", bad)
        } else {
            format!("{base}{bad}\n")
        };
        fs::write(dir.path().join("wikify.yaml"), yaml).unwrap();
        assert!(config::load(dir.path()).is_err(), "accepted {bad}");
    }
    let bad_source = base.replace(
        "sources: []",
        &format!(
            "sources:\n  - id: s0001\n    name: source.md\n    path: ../outside.md\n    sha256: {}",
            "0".repeat(64)
        ),
    );
    fs::write(dir.path().join("wikify.yaml"), bad_source).unwrap();
    assert!(config::load(dir.path()).is_err());
}
