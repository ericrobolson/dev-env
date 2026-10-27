use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, time::Duration};
use tempfile::tempdir;
use wiki_writer::{
    audit::{Audit, Replacement, apply, sample},
    types::{Chunk, EntityType, Fact, FactDraft, Notes, PageRef, Schema},
};
use wiki_writer::{
    config,
    pipeline::run_audit,
    project,
    scheduler::ScheduleOptions,
    types::{ItemRecord, ItemStatus, Provider, Tier},
};

fn chunk(index: usize) -> Chunk {
    Chunk {
        id: format!("c{index:04}"),
        source: "s0001".into(),
        first_page: 1,
        last_page: 1,
        text: "Copper token weighs 12 grams. Silver token weighs 24 grams.".into(),
        table_like: false,
    }
}
fn schema() -> Schema {
    Schema {
        entity_types: BTreeMap::from([(
            "item".into(),
            EntityType {
                fields: vec!["weight".into()],
            },
        )]),
        namespaces: vec!["items".into()],
    }
}
fn draft(entity: &str, weight: &str) -> FactDraft {
    FactDraft {
        entity: entity.into(),
        entity_type: "item".into(),
        aliases: vec![],
        fields: BTreeMap::from([("weight".into(), format!("{weight} grams"))]),
        text: format!("{entity} weighs {weight} grams"),
        quote: format!("{entity} weighs {weight} grams"),
        pages: vec![PageRef {
            source: "s0001".into(),
            page: 1,
        }],
        table_like: false,
    }
}

#[test]
fn audit_sample_rounds_up_and_is_stable() {
    let chunks: Vec<_> = (1..=21).map(chunk).collect();
    let first = sample(&chunks, 0.10);
    assert_eq!(first.len(), 3);
    assert_eq!(first, sample(&chunks, 0.10));
    assert!(sample(&[], 0.10).is_empty());
    assert_eq!(sample(&chunks, 0.0).len(), 0);
}

#[test]
fn audit_apply_is_grounded_rejects_unknown_and_does_not_mutate_raw_notes() {
    let original = Notes {
        facts: vec![Fact {
            id: "f0001".into(),
            chunk: "c0001".into(),
            draft: draft("Copper token", "12"),
        }],
    };
    let before = original.clone();
    let correction = Audit {
        remove: vec!["f0001".into()],
        replace: vec![],
        add: vec![draft("Silver token", "24")],
        reasons: vec!["Correction".into()],
    };
    let once = apply(&original, &correction, &chunk(1), &schema()).unwrap();
    let twice = apply(&original, &correction, &chunk(1), &schema()).unwrap();
    assert_eq!(once, twice);
    assert_eq!(once, vec![draft("Silver token", "24")]);
    assert_eq!(original, before);
    let mut unknown = correction.clone();
    unknown.remove = vec!["f9999".into()];
    assert!(apply(&original, &unknown, &chunk(1), &schema()).is_err());
    let mut duplicate = correction.clone();
    duplicate.replace.push(Replacement {
        id: "f0001".into(),
        fact: draft("Copper token", "12"),
    });
    assert!(apply(&original, &duplicate, &chunk(1), &schema()).is_err());
    let mut ungrounded = correction;
    ungrounded.add[0]
        .fields
        .insert("weight".into(), "25 grams".into());
    assert!(apply(&original, &ungrounded, &chunk(1), &schema()).is_err());
}

#[test]
fn correction_above_threshold_expands_audit_and_keeps_raw_notes() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.md");
    fs::write(
        &source,
        "# Source\nCopper token weighs 12 grams. Silver token weighs 24 grams.\n",
    )
    .unwrap();
    let project_dir =
        project::init(dir.path(), "demo", &[source], Provider::Codex, "generic").unwrap();
    let cfg = config::load(&project_dir).unwrap();
    let chunks: Vec<_> = (1..=5).map(chunk).collect();
    fs::create_dir_all(project_dir.join("state/notes")).unwrap();
    fs::create_dir_all(project_dir.join("state/items/extract")).unwrap();
    let mut raw_bytes = BTreeMap::new();
    for (index, chunk) in chunks.iter().enumerate() {
        let notes = Notes {
            facts: vec![Fact {
                id: format!("f{:04}", index + 1),
                chunk: chunk.id.clone(),
                draft: draft("Copper token", "12"),
            }],
        };
        let bytes = serde_json::to_vec_pretty(&notes).unwrap();
        let relative = format!("state/notes/{}.json", chunk.id);
        fs::write(project_dir.join(&relative), &bytes).unwrap();
        raw_bytes.insert(chunk.id.clone(), bytes.clone());
        let digest: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let record = ItemRecord {
            input_hash: format!("input-{}", chunk.id),
            output_hash: Some(digest.clone()),
            output_path: Some(relative.clone()),
            outputs: BTreeMap::from([(relative, digest)]),
            status: ItemStatus::Done,
            errors: vec![],
            escalated: false,
        };
        fs::write(
            project_dir.join(format!("state/items/extract/{}.json", chunk.id)),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
    }
    let selected = sample(&chunks, 0.10)[0].clone();
    let root = dir.path().join("fixtures");
    for chunk in &chunks {
        let path = root.join(format!("audit/{}/{}/1.json", chunk.id, cfg.models.mid));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let index: usize = chunk.id[1..].parse().unwrap();
        let decision = if chunk.id == selected {
            serde_json::json!({"remove":[format!("f{index:04}")],"replace":[],"add":[draft("Silver token", "24")],"reasons":["wrong token"]})
        } else {
            serde_json::json!({"remove":[],"replace":[],"add":[],"reasons":[]})
        };
        fs::write(
            path,
            serde_json::json!({"response":{"text":decision.to_string(),"model":cfg.models.mid,
            "input_tokens":1,"output_tokens":1,"cost_usd":null,"usage_reported":true}})
            .to_string(),
        )
        .unwrap();
    }
    let options = ScheduleOptions {
        provider: Provider::Fake,
        fake_root: Some(root.clone()),
        models: cfg.models.clone(),
        ceiling: Tier::Large,
        budget_tokens: 1000,
        jobs: 2,
        timeout: Duration::from_secs(1),
        run: "audit".into(),
    };
    let effective = run_audit(&project_dir, &cfg, &chunks, &schema(), &options).unwrap();
    assert_eq!(effective.len(), 5);
    let policy: serde_json::Value =
        serde_json::from_slice(&fs::read(project_dir.join("state/audit_policy.json")).unwrap())
            .unwrap();
    assert_eq!(policy["effective_rate"], 1.0);
    for (id, bytes) in raw_bytes {
        assert_eq!(
            fs::read(project_dir.join(format!("state/notes/{id}.json"))).unwrap(),
            bytes
        );
    }
    fs::remove_dir_all(&root).unwrap();
    let again = run_audit(&project_dir, &cfg, &chunks, &schema(), &options).unwrap();
    assert_eq!(effective, again);
    let pending_record = project_dir.join(format!("state/items/audit/{selected}.json"));
    fs::remove_file(&pending_record).unwrap();
    let mut no_budget = options;
    no_budget.budget_tokens = 0;
    run_audit(&project_dir, &cfg, &chunks, &schema(), &no_budget).unwrap();
    let record: ItemRecord = serde_json::from_slice(&fs::read(&pending_record).unwrap()).unwrap();
    assert_eq!(record.status, ItemStatus::Pending);
}
