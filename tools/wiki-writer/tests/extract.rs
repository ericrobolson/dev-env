use std::collections::BTreeMap;
use std::{fs, time::Duration};
use tempfile::tempdir;
use wiki_writer::{
    config,
    pipeline::{run_extract, run_survey, survey_inputs},
    project,
    scheduler::ScheduleOptions,
    types::{
        Chunk, EntityType, FactDraft, ItemRecord, ItemStatus, PageRef, Provider, Schema, Tier,
    },
    validate::grounding,
};

fn chunk(id: usize, text: &str) -> Chunk {
    Chunk {
        id: format!("c{id:04}"),
        source: "s0001".into(),
        first_page: 1,
        last_page: 2,
        text: text.into(),
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

fn fact() -> FactDraft {
    FactDraft {
        entity: "Copper token".into(),
        entity_type: "item".into(),
        aliases: vec!["12g token".into()],
        fields: BTreeMap::from([("weight".into(), "12 grams".into())]),
        text: "The token weighs 12 grams.".into(),
        quote: "Copper token 12g token weighs 12 grams".into(),
        pages: vec![PageRef {
            source: "s0001".into(),
            page: 1,
        }],
        table_like: false,
    }
}

#[test]
fn survey_selects_at_most_five_evenly_spaced_chunks() {
    let chunks: Vec<_> = (1..=11).map(|i| chunk(i, "text")).collect();
    let selected = survey_inputs(&chunks);
    assert_eq!(
        selected
            .iter()
            .map(|chunk| chunk.id.as_str())
            .collect::<Vec<_>>(),
        ["c0001", "c0003", "c0006", "c0008", "c0011"]
    );
    assert!(survey_inputs(&[]).is_empty());
    assert_eq!(survey_inputs(&chunks[..1])[0].id, "c0001");
}

#[test]
fn grounding_checks_quote_numeric_fields_and_page_bounds() {
    let chunk = chunk(
        1,
        "A Copper token 12g token weighs 12 grams in the source text.",
    );
    assert!(grounding(&fact(), &chunk, &schema()).is_empty());
    let mut bad = fact();
    bad.quote = "".into();
    assert!(
        grounding(&bad, &chunk, &schema())
            .iter()
            .any(|e| e.contains("quote"))
    );
    let mut bad = fact();
    bad.fields.insert("weight".into(), "24 grams".into());
    assert!(
        grounding(&bad, &chunk, &schema())
            .iter()
            .any(|e| e.contains("24"))
    );
    let mut bad = fact();
    bad.pages[0].page = 3;
    assert!(
        grounding(&bad, &chunk, &schema())
            .iter()
            .any(|e| e.contains("page"))
    );
    let mut bad = fact();
    bad.pages[0].source = "s0002".into();
    assert!(
        grounding(&bad, &chunk, &schema())
            .iter()
            .any(|e| e.contains("source"))
    );
    let mut bad = fact();
    bad.entity_type = "creature".into();
    assert!(
        grounding(&bad, &chunk, &schema())
            .iter()
            .any(|e| e.contains("entity_type"))
    );
    let mut bad = fact();
    bad.fields.insert("unknown".into(), "value".into());
    assert!(
        grounding(&bad, &chunk, &schema())
            .iter()
            .any(|e| e.contains("unknown"))
    );
}

#[test]
fn fake_survey_and_parallel_extract_keep_stable_fact_ids_and_cache() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.md");
    fs::write(
        &source,
        "# Source\nCopper token weighs 12 grams and has a clear physical weight.\n",
    )
    .unwrap();
    let project_dir =
        project::init(dir.path(), "demo", &[source], Provider::Codex, "generic").unwrap();
    let cfg = config::load(&project_dir).unwrap();
    let root = dir.path().join("fixtures");
    let survey_path = root.join(format!("survey/global/{}/1.json", cfg.models.large));
    fs::create_dir_all(survey_path.parent().unwrap()).unwrap();
    fs::write(&survey_path, r#"{"response":{"text":"entity_types:\n  item:\n    fields: [weight]\nnamespaces: [items]\n","model":"large","input_tokens":1,"output_tokens":1,"cost_usd":null,"usage_reported":true}}"#).unwrap();
    let text = "Copper token weighs 12 grams and has a clear physical weight.";
    let chunks = vec![chunk(1, text), chunk(2, text)];
    let options = ScheduleOptions {
        provider: Provider::Fake,
        fake_root: Some(root.clone()),
        models: cfg.models.clone(),
        ceiling: Tier::Large,
        budget_tokens: 1000,
        jobs: 2,
        timeout: Duration::from_secs(1),
        run: "test".into(),
    };
    let schema = run_survey(&project_dir, &cfg, &[], &chunks, &options)
        .unwrap()
        .unwrap();
    assert!(schema.entity_types.contains_key("item"));
    for id in ["c0001", "c0002"] {
        let path = root.join(format!("extract/{}/{}/1.json", id, cfg.models.small));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let output = serde_json::json!({"facts":[{"entity":"Copper token","entity_type":"item",
            "aliases":[],"fields":{"weight":"12 grams"},"text":"weighs 12 grams",
            "quote":"Copper token weighs 12 grams","pages":[{"source":"s0001","page":1}],"table_like":false}]});
        fs::write(
            path,
            serde_json::json!({"response":{"text":output.to_string(),"model":cfg.models.small,
            "input_tokens":1,"output_tokens":1,"cost_usd":null,"usage_reported":true}})
            .to_string(),
        )
        .unwrap();
    }
    let notes = run_extract(&project_dir, &cfg, &chunks, &schema, &options).unwrap();
    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0].facts[0].id, "f0001");
    assert_eq!(notes[1].facts[0].id, "f0001");
    fs::remove_dir_all(&root).unwrap();
    let cached = run_extract(&project_dir, &cfg, &chunks, &schema, &options).unwrap();
    assert_eq!(cached, notes);
    let mut changed_schema = schema.clone();
    changed_schema.namespaces.push("rules".into());
    let paused = ScheduleOptions {
        budget_tokens: 0,
        ..options
    };
    assert!(
        run_extract(&project_dir, &cfg, &chunks, &changed_schema, &paused)
            .unwrap()
            .is_empty()
    );
    for id in ["c0001", "c0002"] {
        let record: ItemRecord = serde_json::from_slice(
            &fs::read(project_dir.join(format!("state/items/extract/{id}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(record.status, ItemStatus::Pending);
    }
}

#[test]
fn long_factual_chunk_rejects_empty_facts_and_repairs() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.md");
    fs::write(
        &source,
        "# Source\nCopper token weighs 12 grams and has a clear physical weight.\n",
    )
    .unwrap();
    let project_dir =
        project::init(dir.path(), "demo", &[source], Provider::Codex, "generic").unwrap();
    let cfg = config::load(&project_dir).unwrap();
    let root = dir.path().join("fixtures");
    let chunk = chunk(
        1,
        &format!(
            "Copper token weighs 12 grams. {}",
            "The token has a clear weight. ".repeat(30)
        ),
    );
    let base = root.join(format!("extract/c0001/{}", cfg.models.small));
    fs::create_dir_all(&base).unwrap();
    let response = |text: String| {
        serde_json::json!({"response":{"text":text,"model":cfg.models.small,
        "input_tokens":1,"output_tokens":1,"cost_usd":null,"usage_reported":true}})
        .to_string()
    };
    fs::write(base.join("1.json"), response(r#"{"facts":[]}"#.into())).unwrap();
    let good = serde_json::json!({"facts":[{"entity":"Copper token","entity_type":"item",
        "aliases":[],"fields":{"weight":"12 grams"},"text":"Copper token weighs 12 grams",
        "quote":"Copper token weighs 12 grams","pages":[{"source":"s0001","page":1}],"table_like":false}]});
    fs::write(base.join("2.json"), response(good.to_string())).unwrap();
    let options = ScheduleOptions {
        provider: Provider::Fake,
        fake_root: Some(root),
        models: cfg.models.clone(),
        ceiling: Tier::Small,
        budget_tokens: 1000,
        jobs: 1,
        timeout: Duration::from_secs(1),
        run: "extract".into(),
    };
    let notes = run_extract(&project_dir, &cfg, &[chunk], &schema(), &options).unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].facts.len(), 1);
    let rows: Vec<serde_json::Value> = fs::read_to_string(project_dir.join("state/ledger.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        rows.iter()
            .map(|row| row["outcome"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["bad_output", "ok"]
    );
}
