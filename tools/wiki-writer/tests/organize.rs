use std::{collections::BTreeMap, fs, time::Duration};
use tempfile::tempdir;
use wiki_writer::{config, pipeline::run_resolve, project, scheduler::ScheduleOptions};
use wiki_writer::{
    resolve::{
        ClusterDecision, PairDecision, PairVerdict, candidates, group_exact, validate_cluster,
        validate_pair,
    },
    types::{
        Entity, EntityType, Fact, FactDraft, ItemRecord, ItemStatus, Notes, Outline, PageKind,
        PageRef, PageSpec, Provider, Schema, Tier,
    },
    validate::validate_outline,
};

fn entity(id: &str, name: &str, kind: &str, fact: &str) -> Entity {
    Entity {
        id: id.into(),
        name: name.into(),
        entity_type: kind.into(),
        aliases: vec![],
        fact_ids: vec![fact.into()],
    }
}

#[test]
fn resolve_budget_pause_records_pending_pair_and_gap() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.md");
    fs::write(
        &source,
        "# Source\nThis source has enough text to initialize a local project for testing.\n",
    )
    .unwrap();
    let project_dir =
        project::init(dir.path(), "demo", &[source], Provider::Codex, "generic").unwrap();
    let config = config::load(&project_dir).unwrap();
    let effective = BTreeMap::from([
        (
            "c0001".into(),
            Notes {
                facts: vec![fact("f0001", "Copper token", "item")],
            },
        ),
        (
            "c0002".into(),
            Notes {
                facts: vec![fact("f0002", "Copper tokens", "item")],
            },
        ),
    ]);
    let options = ScheduleOptions {
        provider: Provider::Fake,
        fake_root: Some(dir.path().join("missing")),
        models: config.models.clone(),
        ceiling: Tier::Large,
        budget_tokens: 0,
        jobs: 1,
        timeout: Duration::from_secs(1),
        run: "resolve".into(),
    };
    run_resolve(&project_dir, &config, &effective, &options).unwrap();
    let path = project_dir.join("state/items/resolve/e0001_e0002.json");
    let record: ItemRecord = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(record.status, ItemStatus::Pending);
    let gaps: Vec<String> =
        serde_json::from_slice(&fs::read(project_dir.join("state/gaps.json")).unwrap()).unwrap();
    assert!(
        gaps.iter()
            .any(|gap| gap.contains("resolve/e0001_e0002: pending"))
    );
}

#[test]
fn cached_blocked_pair_restores_gap_after_ingest_reset() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.md");
    fs::write(
        &source,
        "# Source\nThis source has enough text for a local project and a valid chunk.\n",
    )
    .unwrap();
    let project_dir =
        project::init(dir.path(), "demo", &[source], Provider::Codex, "generic").unwrap();
    let config = config::load(&project_dir).unwrap();
    let effective = BTreeMap::from([
        (
            "c0001".into(),
            Notes {
                facts: vec![fact("f0001", "Copper token", "item")],
            },
        ),
        (
            "c0002".into(),
            Notes {
                facts: vec![fact("f0002", "Copper tokens", "item")],
            },
        ),
    ]);
    let options = ScheduleOptions {
        provider: Provider::Fake,
        fake_root: Some(dir.path().join("missing")),
        models: config.models.clone(),
        ceiling: Tier::Small,
        budget_tokens: 1000,
        jobs: 1,
        timeout: Duration::from_secs(1),
        run: "blocked-gap".into(),
    };
    run_resolve(&project_dir, &config, &effective, &options).unwrap();
    let record: ItemRecord = serde_json::from_slice(
        &fs::read(project_dir.join("state/items/resolve/e0001_e0002.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(record.status, ItemStatus::Blocked);
    fs::write(project_dir.join("state/gaps.json"), b"[]").unwrap();
    run_resolve(&project_dir, &config, &effective, &options).unwrap();
    let gaps: Vec<String> =
        serde_json::from_slice(&fs::read(project_dir.join("state/gaps.json")).unwrap()).unwrap();
    assert!(
        gaps.iter().any(|gap| gap == "resolve/e0001_e0002: blocked"),
        "{gaps:?}"
    );
}
fn fact(id: &str, name: &str, kind: &str) -> Fact {
    Fact {
        id: id.into(),
        chunk: "c0001".into(),
        draft: FactDraft {
            entity: name.into(),
            entity_type: kind.into(),
            aliases: vec![],
            fields: BTreeMap::new(),
            text: "A grounded fact".into(),
            quote: "A grounded fact".into(),
            pages: vec![PageRef {
                source: "s0001".into(),
                page: 1,
            }],
            table_like: false,
        },
    }
}

#[test]
fn exact_grouping_and_candidates_respect_entity_type() {
    let facts = vec![
        fact("f0001", "Copper Token", "item"),
        fact("f0002", " copper   token ", "item"),
        fact("f0003", "Copper Token", "rule"),
    ];
    let grouped = group_exact(&facts);
    assert_eq!(grouped.len(), 2);
    assert_eq!(grouped[0].id, "e0001");
    assert_eq!(grouped[0].fact_ids, ["f0001", "f0002"]);
    assert!(candidates(&grouped).is_empty());
    let similar = vec![
        entity("e0001", "Copper token", "item", "f0001"),
        entity("e0002", "Copper tokens", "item", "f0002"),
        entity("e0003", "Completely unrelated", "item", "f0003"),
        entity("e0004", "Copper tokens", "rule", "f0004"),
    ];
    assert_eq!(candidates(&similar), vec![("e0001".into(), "e0002".into())]);
}

#[test]
fn pair_and_cluster_evidence_must_cover_members_and_obey_no_edges() {
    let left = entity("e0001", "Copper token", "item", "f0001");
    let right = entity("e0002", "Copper tokens", "item", "f0002");
    let yes = PairDecision {
        verdict: PairVerdict::Yes,
        evidence: vec!["f0001".into(), "f0002".into()],
    };
    assert!(validate_pair(&yes, &left, &right).is_empty());
    let weak = PairDecision {
        verdict: PairVerdict::Yes,
        evidence: vec!["f0001".into()],
    };
    assert!(!validate_pair(&weak, &left, &right).is_empty());
    let partition = ClusterDecision {
        groups: vec![vec!["e0001".into(), "e0002".into()]],
        evidence: vec!["f0001".into(), "f0002".into()],
    };
    assert!(validate_cluster(&partition, &[left.clone(), right.clone()], &[]).is_empty());
    assert!(
        !validate_cluster(
            &partition,
            &[left, right],
            &[("e0001".into(), "e0002".into())]
        )
        .is_empty()
    );
}

#[test]
fn outline_requires_start_coverage_and_valid_namespaces() {
    let schema = Schema {
        entity_types: BTreeMap::from([("item".into(), EntityType { fields: vec![] })]),
        namespaces: vec!["items".into()],
    };
    let entities = vec![entity("e0001", "Copper token", "item", "f0001")];
    let facts = vec![fact("f0001", "Copper token", "item")];
    let good = Outline {
        pages: vec![
            PageSpec {
                id: "start".into(),
                title: "Home".into(),
                kind: PageKind::Index,
                entity_ids: vec![],
                fact_ids: vec![],
            },
            PageSpec {
                id: "items:copper_token".into(),
                title: "Copper token".into(),
                kind: PageKind::Leaf,
                entity_ids: vec!["e0001".into()],
                fact_ids: vec!["f0001".into()],
            },
        ],
        gaps: vec![],
    };
    assert!(validate_outline(&good, &schema, &entities, &facts).is_empty());
    let mut bad = good.clone();
    bad.pages[1].id = "items:Bad Slug".into();
    assert!(!validate_outline(&bad, &schema, &entities, &facts).is_empty());
    let mut bad = good.clone();
    bad.pages.pop();
    assert!(!validate_outline(&bad, &schema, &entities, &facts).is_empty());
}
