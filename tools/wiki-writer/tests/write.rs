use std::{collections::BTreeMap, fs, time::Duration};
use tempfile::tempdir;
use wiki_writer::{
    config,
    link::{link, link_markdown},
    project,
    scheduler::ScheduleOptions,
    types::{Entity, Fact, FactDraft, Outline, PageKind, PageRef, PageSpec, Provider, Tier},
    validate::validate_markdown,
    write::write_pages,
};

fn fixture() -> (PageSpec, Outline, Vec<Entity>, Vec<Fact>) {
    let start = PageSpec {
        id: "start".into(),
        title: "Home".into(),
        kind: PageKind::Index,
        entity_ids: vec![],
        fact_ids: vec![],
    };
    let copper = PageSpec {
        id: "items:copper".into(),
        title: "Copper".into(),
        kind: PageKind::Leaf,
        entity_ids: vec!["e0001".into()],
        fact_ids: vec!["f0001".into()],
    };
    let silver = PageSpec {
        id: "items:silver".into(),
        title: "Silver".into(),
        kind: PageKind::Leaf,
        entity_ids: vec!["e0002".into()],
        fact_ids: vec!["f0002".into()],
    };
    let outline = Outline {
        pages: vec![start, copper.clone(), silver],
        gaps: vec![],
    };
    let entities = vec![
        Entity {
            id: "e0001".into(),
            name: "Copper token".into(),
            entity_type: "item".into(),
            aliases: vec!["copper".into()],
            fact_ids: vec!["f0001".into()],
        },
        Entity {
            id: "e0002".into(),
            name: "Silver token".into(),
            entity_type: "item".into(),
            aliases: vec!["silver".into()],
            fact_ids: vec!["f0002".into()],
        },
    ];
    let facts = ["f0001", "f0002"]
        .iter()
        .map(|id| Fact {
            id: (*id).into(),
            chunk: "c0001".into(),
            draft: FactDraft {
                entity: "Token".into(),
                entity_type: "item".into(),
                aliases: vec![],
                fields: BTreeMap::new(),
                text: "A fact".into(),
                quote: "A fact".into(),
                pages: vec![PageRef {
                    source: "s0001".into(),
                    page: 1,
                }],
                table_like: false,
            },
        })
        .collect();
    (copper, outline, entities, facts)
}

#[test]
fn writer_requires_assigned_citations_and_valid_links() {
    let (page, outline, _, _) = fixture();
    let errors = validate_markdown(
        "# Copper\nA fact [^f0001] and [Silver](items:silver).",
        &page,
        &outline,
    );
    assert!(errors.is_empty(), "{errors:?}");
    assert!(
        validate_markdown("# Copper\n`[^f0001]`", &page, &outline)
            .iter()
            .any(|e| e.contains("missing citation"))
    );
    assert!(
        validate_markdown("# Copper\nUnknown [^f9999]", &page, &outline)
            .iter()
            .any(|e| e.contains("unknown citation"))
    );
    assert!(
        validate_markdown(
            "# Copper\nA fact [^f0001] and [Broken](items:missing).",
            &page,
            &outline
        )
        .iter()
        .any(|e| e.contains("broken link"))
    );
}

#[test]
fn alias_linking_skips_code_existing_links_and_is_idempotent() {
    let (page, outline, entities, _) = fixture();
    let input =
        "A silver token appears. `silver token` and [silver token](items:silver) stay. [^f0001]";
    let linked = link_markdown(input, &page, &outline, &entities);
    assert!(linked.contains("[silver token](items:silver) appears"));
    assert!(linked.contains("`silver token`"));
    assert_eq!(link_markdown(&linked, &page, &outline, &entities), linked);
}

#[test]
fn short_single_word_condition_does_not_link_ordinary_prose() {
    let (page, outline, mut entities, _) = fixture();
    entities[1].name = "Down".into();
    entities[1].entity_type = "condition".into();
    let text = "A -1DT shift moves down one Die Type. A model can be Down.";
    let linked = link_markdown(text, &page, &outline, &entities);
    assert!(linked.contains("moves down one Die Type"), "{linked}");
    assert!(linked.contains("[Down](items:silver)"), "{linked}");
}

#[test]
fn rule_and_action_names_need_term_context_for_auto_links() {
    let (page, outline, mut entities, _) = fixture();
    for (name, kind, text) in [
        (
            "Heavy",
            "rule",
            "Grenades, Heavy Armor, and Medkits are free.",
        ),
        ("Move", "action", "Treasure tokens move with their carrier."),
        (
            "Leader out of action",
            "rule",
            "Take a model within LOS of your leader out of action.",
        ),
    ] {
        entities[1].name = name.into();
        entities[1].entity_type = kind.into();
        assert_eq!(link_markdown(text, &page, &outline, &entities), text);
    }
}

#[test]
fn writer_preserves_hand_edited_markdown_when_inputs_change() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.md");
    fs::write(&source, "# Copper\nA copper token weighs 12 grams.\n").unwrap();
    let project_dir =
        project::init(dir.path(), "demo", &[source], Provider::Codex, "generic").unwrap();
    let cfg = config::load(&project_dir).unwrap();
    let (page, _, entities, facts) = fixture();
    let mut outline = Outline {
        pages: vec![page.clone()],
        gaps: vec![],
    };
    let root = dir.path().join("fixtures");
    let fixture_dir = root.join("write/items:copper").join(&cfg.models.small);
    fs::create_dir_all(&fixture_dir).unwrap();
    for (attempt, text) in [
        (1, "# Copper\nA fact [^f0001].\n"),
        (2, "# Copper\nA revised fact [^f0001].\n"),
    ] {
        fs::write(fixture_dir.join(format!("{attempt}.json")), serde_json::json!({"response":{"text":text,"model":cfg.models.small,"input_tokens":1,"output_tokens":1,"cost_usd":null,"usage_reported":true}}).to_string()).unwrap();
    }
    let options = ScheduleOptions {
        provider: Provider::Fake,
        fake_root: Some(root),
        models: cfg.models.clone(),
        ceiling: Tier::Small,
        budget_tokens: 1000,
        jobs: 1,
        timeout: Duration::from_secs(1),
        run: "edit-test".into(),
    };
    write_pages(&project_dir, &cfg, &outline, &entities, &facts, &options).unwrap();
    let content = project_dir.join("content/items/copper.md");
    fs::write(&content, "# Copper\nA fact [^f0001].\nMy hand edit.\n").unwrap();
    let ledger_before = fs::read(project_dir.join("state/ledger.jsonl")).unwrap();
    outline.gaps.push("extract/elsewhere: pending".into());
    write_pages(&project_dir, &cfg, &outline, &entities, &facts, &options).unwrap();
    assert_eq!(
        fs::read(project_dir.join("state/ledger.jsonl")).unwrap(),
        ledger_before
    );
    outline.pages[0].title = "Copper revised".into();
    write_pages(&project_dir, &cfg, &outline, &entities, &facts, &options).unwrap();
    assert!(
        fs::read_to_string(&content)
            .unwrap()
            .contains("My hand edit.")
    );
    assert!(
        fs::read_to_string(project_dir.join("state/drafts/items/copper.md"))
            .unwrap()
            .contains("A revised fact")
    );
    let edited = "# Copper\nA fact [^f0001].\nMy hand edit mentions silver token.\n";
    fs::write(&content, edited).unwrap();
    let (_, full_outline, _, _) = fixture();
    fs::write(
        project_dir.join("state/outline.yaml"),
        serde_yaml::to_string(&full_outline).unwrap(),
    )
    .unwrap();
    link(&project_dir, &full_outline, &entities).unwrap();
    assert_eq!(fs::read_to_string(&content).unwrap(), edited);
}
