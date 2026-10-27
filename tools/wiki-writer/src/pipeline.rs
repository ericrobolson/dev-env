use crate::{
    audit::{self, Audit},
    config::Config,
    prompts,
    resolve::{self, ClusterDecision, PairDecision, PairVerdict},
    scheduler::{self, Job, JobOutcome, ScheduleOptions},
    state::{atomic_write, identify_facts, input_hash, is_blocked, is_done, sha256},
    types::{
        Chunk, Entity, Fact, FactDraft, ItemRecord, ItemStatus, Notes, Outline, PageKind, PageSpec,
        Schema, Stage, Tier, TocEntry,
    },
    validate,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

pub fn survey_inputs(chunks: &[Chunk]) -> Vec<Chunk> {
    let count = chunks.len().min(5);
    if count == 0 {
        return vec![];
    }
    if count == 1 {
        return vec![chunks[0].clone()];
    }
    (0..count)
        .map(|index| {
            let selected = index * (chunks.len() - 1) / (count - 1);
            chunks[selected].clone()
        })
        .collect()
}

pub fn run_survey(
    project: &Path,
    config: &Config,
    toc: &[TocEntry],
    chunks: &[Chunk],
    options: &ScheduleOptions,
) -> Result<Option<Schema>> {
    if chunks.is_empty() {
        return Ok(None);
    }
    let (template, version) = prompts::template("survey")?;
    let seed = prompts::profile(&config.profile)?;
    let sample = survey_inputs(chunks);
    let input = serde_json::to_vec(&(
        toc,
        &sample,
        seed,
        config.agent,
        &config.models,
        &config.profile,
    ))?;
    let hash = input_hash("survey", version, &input, options.ceiling);
    let record_path = project.join("state/items/survey/global.json");
    let output_path = project.join("state/schema.yaml");
    if let Ok(bytes) = fs::read(&record_path)
        && let Ok(record) = serde_json::from_slice::<ItemRecord>(&bytes)
    {
        if is_done(project, &record, &hash) {
            return Ok(Some(serde_yaml::from_slice(&fs::read(output_path)?)?));
        }
        if is_blocked(&record, &hash) {
            return Ok(None);
        }
    }
    let data = serde_json::json!({"toc": toc, "sample_chunks": sample, "profile_seed": seed});
    let prompt = format!(
        "{template}\nInput data (JSON):\n{}",
        serde_json::to_string(&data)?
    );
    let job = Job {
        stage: Stage::Survey,
        item: "global".into(),
        start: Tier::Large,
        prompt,
    };
    let report = scheduler::run_jobs(
        project,
        vec![job],
        options,
        &|_, text| {
            let schema = serde_yaml::from_str::<Schema>(text).map_err(|e| e.to_string())?;
            let errors = validate::validate_schema(&schema);
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors.join("; "))
            }
        },
        &std::thread::sleep,
    )?;
    match report.outcomes.get(&(Stage::Survey, "global".into())) {
        Some(JobOutcome::Done(text)) => {
            let schema: Schema = serde_yaml::from_str(text)?;
            let bytes = serde_yaml::to_string(&schema)?.into_bytes();
            atomic_write(&output_path, &bytes)?;
            save_record(
                project,
                &record_path,
                hash,
                "state/schema.yaml",
                &bytes,
                ItemStatus::Done,
                vec![],
            )?;
            Ok(Some(schema))
        }
        Some(JobOutcome::Blocked(reason)) => {
            save_record(
                project,
                &record_path,
                hash,
                "",
                &[],
                ItemStatus::Blocked,
                vec![reason.clone()],
            )?;
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn verify_survey_current(project: &Path, config: &Config, ceiling: Tier) -> Result<()> {
    let chunks: Vec<Chunk> =
        serde_json::from_slice(&fs::read(project.join("state/ingest/chunks.json"))?)?;
    let toc: Vec<TocEntry> =
        serde_json::from_slice(&fs::read(project.join("state/ingest/toc.json"))?)?;
    let (_, version) = prompts::template("survey")?;
    let seed = prompts::profile(&config.profile)?;
    let sample = survey_inputs(&chunks);
    let input = serde_json::to_vec(&(
        &toc,
        &sample,
        seed,
        config.agent,
        &config.models,
        &config.profile,
    ))?;
    let hash = input_hash("survey", version, &input, ceiling);
    let record: ItemRecord = serde_json::from_slice(
        &fs::read(project.join("state/items/survey/global.json"))
            .context("survey is required before --only")?,
    )?;
    ensure!(
        is_done(project, &record, &hash),
        "survey schema is missing or stale; run without --only"
    );
    Ok(())
}

fn verify_extract_current(project: &Path, config: &Config, ceiling: Tier) -> Result<()> {
    let chunks: Vec<Chunk> =
        serde_json::from_slice(&fs::read(project.join("state/ingest/chunks.json"))?)?;
    let schema: Schema = serde_yaml::from_slice(&fs::read(project.join("state/schema.yaml"))?)?;
    let (_, version) = prompts::template("extract")?;
    let schema_bytes = serde_json::to_vec(&schema)?;
    for chunk in &chunks {
        let input = serde_json::to_vec(&(
            chunk,
            &schema_bytes,
            config.agent,
            &config.models,
            &config.profile,
        ))?;
        let hash = input_hash("extract", version, &input, ceiling);
        let path = project.join(format!("state/items/extract/{}.json", chunk.id));
        let record: ItemRecord = serde_json::from_slice(
            &fs::read(path)
                .with_context(|| format!("extract/{} is required before --only", chunk.id))?,
        )?;
        ensure!(
            is_done(project, &record, &hash) || is_blocked(&record, &hash),
            "extract/{} is missing or stale; run without --only",
            chunk.id
        );
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DraftNotes {
    facts: Vec<FactDraft>,
}

pub fn run_extract(
    project: &Path,
    config: &Config,
    chunks: &[Chunk],
    schema: &Schema,
    options: &ScheduleOptions,
) -> Result<Vec<Notes>> {
    let (template, version) = prompts::template("extract")?;
    fs::create_dir_all(project.join("state/notes"))?;
    fs::create_dir_all(project.join("state/items/extract"))?;
    let schema_bytes = serde_json::to_vec(schema)?;
    let mut jobs = Vec::new();
    let mut hashes = BTreeMap::new();
    let mut successful = BTreeMap::<String, Notes>::new();
    let by_id: BTreeMap<_, _> = chunks
        .iter()
        .map(|chunk| (chunk.id.clone(), chunk))
        .collect();
    for chunk in chunks {
        let input = serde_json::to_vec(&(
            chunk,
            &schema_bytes,
            config.agent,
            &config.models,
            &config.profile,
        ))?;
        let hash = input_hash("extract", version, &input, options.ceiling);
        hashes.insert(chunk.id.clone(), hash.clone());
        let record_path = project
            .join("state/items/extract")
            .join(format!("{}.json", chunk.id));
        if let Ok(bytes) = fs::read(&record_path)
            && let Ok(record) = serde_json::from_slice::<ItemRecord>(&bytes)
        {
            if is_done(project, &record, &hash) {
                let notes: Notes = serde_json::from_slice(&fs::read(
                    project
                        .join("state/notes")
                        .join(format!("{}.json", chunk.id)),
                )?)?;
                successful.insert(chunk.id.clone(), notes);
                continue;
            }
            if is_blocked(&record, &hash) {
                continue;
            }
        }
        let data = serde_json::json!({"chunk": chunk, "schema": schema});
        jobs.push(Job {
            stage: Stage::Extract,
            item: chunk.id.clone(),
            start: Tier::Small,
            prompt: format!(
                "{template}\nInput data (JSON):\n{}",
                serde_json::to_string(&data)?
            ),
        });
    }
    let report = scheduler::run_jobs(
        project,
        jobs,
        options,
        &|job, text| {
            let output: DraftNotes = serde_json::from_str(text).map_err(|e| e.to_string())?;
            let chunk = by_id
                .get(&job.item)
                .ok_or_else(|| "unknown chunk".to_owned())?;
            let nonspace = chunk.text.chars().filter(|c| !c.is_whitespace()).count();
            if output.facts.is_empty()
                && (nonspace > 2000
                    || (nonspace > 500 && chunk.text.chars().any(|c| c.is_ascii_digit())))
            {
                return Err("empty facts for a substantial factual chunk; extract grounded facts or explain the absence of facts".into());
            }
            let errors: Vec<String> = output
                .facts
                .iter()
                .flat_map(|fact| validate::grounding(fact, chunk, schema))
                .collect();
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors.join("; "))
            }
        },
        &std::thread::sleep,
    )?;
    for ((stage, id), outcome) in report.outcomes {
        ensure!(stage == Stage::Extract, "unexpected scheduler stage");
        let hash = hashes.get(&id).context("missing extract hash")?.clone();
        let record_path = project
            .join("state/items/extract")
            .join(format!("{id}.json"));
        match outcome {
            JobOutcome::Done(text) => {
                let draft: DraftNotes = serde_json::from_str(&text)?;
                let chunk = by_id.get(&id).context("unknown chunk")?;
                let facts = identify_facts(
                    project,
                    draft
                        .facts
                        .into_iter()
                        .map(|fact| (id.clone(), chunk.source.clone(), fact))
                        .collect(),
                )?;
                let notes = Notes { facts };
                let bytes = serde_json::to_vec_pretty(&notes)?;
                let relative = format!("state/notes/{id}.json");
                atomic_write(&project.join(&relative), &bytes)?;
                save_record(
                    project,
                    &record_path,
                    hash,
                    &relative,
                    &bytes,
                    ItemStatus::Done,
                    vec![],
                )?;
                successful.insert(id, notes);
            }
            JobOutcome::Blocked(reason) => {
                save_record(
                    project,
                    &record_path,
                    hash,
                    "",
                    &[],
                    ItemStatus::Blocked,
                    vec![reason],
                )?;
            }
            JobOutcome::Pending => {
                save_record(
                    project,
                    &record_path,
                    hash,
                    "",
                    &[],
                    ItemStatus::Pending,
                    vec!["extract pending".into()],
                )?;
            }
        }
    }
    Ok(successful.into_values().collect())
}

fn save_record(
    project: &Path,
    path: &Path,
    input_hash: String,
    relative: &str,
    output: &[u8],
    status: ItemStatus,
    errors: Vec<String>,
) -> Result<()> {
    let mut outputs = BTreeMap::new();
    let digest = if relative.is_empty() {
        None
    } else {
        Some(hex(output))
    };
    if let Some(digest) = &digest {
        outputs.insert(relative.to_owned(), digest.clone());
    }
    let record = ItemRecord {
        input_hash,
        output_hash: digest.clone(),
        output_path: if relative.is_empty() {
            None
        } else {
            Some(relative.into())
        },
        outputs,
        status,
        errors,
        escalated: false,
    };
    fs::create_dir_all(path.parent().context("record parent missing")?)?;
    let _ = project;
    atomic_write(path, &serde_json::to_vec_pretty(&record)?)
}

fn deterministic_outline(
    config: &Config,
    schema: &Schema,
    entities: &[Entity],
    gaps: &[String],
) -> Outline {
    let mut by_namespace = BTreeMap::<String, Vec<&Entity>>::new();
    for entity in entities {
        let plural = if let Some(stem) = entity.entity_type.strip_suffix('y') {
            format!("{stem}ies")
        } else {
            format!("{}s", entity.entity_type)
        };
        let namespace = if schema.namespaces.contains(&plural) {
            plural
        } else {
            schema.namespaces[0].clone()
        };
        by_namespace.entry(namespace).or_default().push(entity);
    }
    let mut pages = vec![PageSpec {
        id: "start".into(),
        title: config.project.clone(),
        kind: PageKind::Index,
        entity_ids: vec![],
        fact_ids: vec![],
    }];
    for (namespace, mut members) in by_namespace {
        members.sort_by_key(|entity| (entity.name.to_lowercase(), entity.id.clone()));
        for (index, group) in members.chunks(20).enumerate() {
            let mut fact_ids = BTreeSet::new();
            for entity in group {
                fact_ids.extend(entity.fact_ids.iter().cloned());
            }
            pages.push(PageSpec {
                id: format!("{namespace}:page_{:04}", index + 1),
                title: format!("{} {}", namespace.replace('_', " "), index + 1),
                kind: PageKind::Leaf,
                entity_ids: group.iter().map(|entity| entity.id.clone()).collect(),
                fact_ids: fact_ids.into_iter().collect(),
            });
        }
    }
    Outline {
        pages,
        gaps: gaps.to_vec(),
    }
}

fn hex(bytes: &[u8]) -> String {
    sha256(bytes)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditPolicy {
    fingerprint: String,
    effective_rate: f64,
}

pub fn run_audit(
    project: &Path,
    config: &Config,
    chunks: &[Chunk],
    schema: &Schema,
    options: &ScheduleOptions,
) -> Result<BTreeMap<String, Notes>> {
    let (template, version) = prompts::template("audit")?;
    fs::create_dir_all(project.join("state/audit"))?;
    fs::create_dir_all(project.join("state/effective"))?;
    fs::create_dir_all(project.join("state/items/audit"))?;
    let mut eligible = Vec::new();
    let mut raw = BTreeMap::new();
    let mut fingerprint_items = Vec::new();
    for chunk in chunks {
        let record_path = project.join(format!("state/items/extract/{}.json", chunk.id));
        let Ok(record_bytes) = fs::read(&record_path) else {
            continue;
        };
        let Ok(record) = serde_json::from_slice::<ItemRecord>(&record_bytes) else {
            continue;
        };
        if record.status != ItemStatus::Done {
            continue;
        }
        let notes_path = project.join(format!("state/notes/{}.json", chunk.id));
        let Ok(notes_bytes) = fs::read(notes_path) else {
            continue;
        };
        if record.output_hash.as_deref() != Some(hex(&notes_bytes).as_str()) {
            continue;
        }
        let notes: Notes = serde_json::from_slice(&notes_bytes)?;
        fingerprint_items.push((
            chunk.id.clone(),
            record.input_hash.clone(),
            hex(&notes_bytes),
        ));
        eligible.push(chunk.clone());
        raw.insert(chunk.id.clone(), (notes, record.escalated));
    }
    fingerprint_items.sort();
    let fingerprint = hex(&serde_json::to_vec(&(
        fingerprint_items,
        config.audit_sample_rate,
        config.audit_failure_threshold,
        version,
    ))?);
    let policy_path = project.join("state/audit_policy.json");
    let policy = fs::read(&policy_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<AuditPolicy>(&bytes).ok())
        .filter(|saved| saved.fingerprint == fingerprint);
    let rate = policy
        .as_ref()
        .map(|saved| saved.effective_rate)
        .unwrap_or(config.audit_sample_rate);
    let mut selected: std::collections::BTreeSet<String> =
        audit::sample(&eligible, rate).into_iter().collect();
    for chunk in &eligible {
        if chunk.table_like
            || raw.get(&chunk.id).is_some_and(|(notes, escalated)| {
                *escalated || notes.facts.iter().any(|fact| fact.draft.table_like)
            })
        {
            selected.insert(chunk.id.clone());
        }
    }
    let mut effective = BTreeMap::new();
    let mut failures = 0usize;
    let mut completed = 0usize;
    run_audit_selection(
        project,
        config,
        schema,
        options,
        template,
        version,
        &eligible,
        &raw,
        &selected,
        &mut effective,
        &mut failures,
        &mut completed,
    )?;
    if policy.is_none()
        && completed > 0
        && failures as f64 / completed as f64 > config.audit_failure_threshold
    {
        eprintln!("warning: audit/global: failure rate exceeded threshold; expanding to 100%");
        let expanded = AuditPolicy {
            fingerprint,
            effective_rate: 1.0,
        };
        atomic_write(&policy_path, &serde_json::to_vec_pretty(&expanded)?)?;
        let remaining: std::collections::BTreeSet<_> = eligible
            .iter()
            .map(|chunk| chunk.id.clone())
            .filter(|id| !selected.contains(id))
            .collect();
        run_audit_selection(
            project,
            config,
            schema,
            options,
            template,
            version,
            &eligible,
            &raw,
            &remaining,
            &mut effective,
            &mut failures,
            &mut completed,
        )?;
    }
    for chunk in &eligible {
        if !effective.contains_key(&chunk.id) {
            effective.insert(chunk.id.clone(), raw[&chunk.id].0.clone());
        }
    }
    for (id, notes) in &effective {
        let bytes = serde_json::to_vec_pretty(notes)?;
        let path = project.join(format!("state/effective/{id}.json"));
        if fs::read(&path).ok().as_deref() != Some(bytes.as_slice()) {
            atomic_write(&path, &bytes)?;
        }
    }
    Ok(effective)
}

fn verify_audit_current(project: &Path, config: &Config, ceiling: Tier) -> Result<()> {
    let chunks: Vec<Chunk> =
        serde_json::from_slice(&fs::read(project.join("state/ingest/chunks.json"))?)?;
    let schema: Schema = serde_yaml::from_slice(&fs::read(project.join("state/schema.yaml"))?)?;
    let (_, version) = prompts::template("audit")?;
    let mut eligible = Vec::new();
    let mut raw = BTreeMap::<String, (Notes, bool)>::new();
    let mut fingerprint_items = Vec::new();
    for chunk in &chunks {
        let record: ItemRecord = serde_json::from_slice(&fs::read(
            project.join(format!("state/items/extract/{}.json", chunk.id)),
        )?)?;
        if record.status == ItemStatus::Blocked {
            continue;
        }
        let bytes = fs::read(project.join(format!("state/notes/{}.json", chunk.id)))?;
        let notes: Notes = serde_json::from_slice(&bytes)?;
        fingerprint_items.push((chunk.id.clone(), record.input_hash, hex(&bytes)));
        eligible.push(chunk.clone());
        raw.insert(chunk.id.clone(), (notes, record.escalated));
    }
    fingerprint_items.sort();
    let fingerprint = hex(&serde_json::to_vec(&(
        fingerprint_items,
        config.audit_sample_rate,
        config.audit_failure_threshold,
        version,
    ))?);
    let rate = fs::read(project.join("state/audit_policy.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<AuditPolicy>(&bytes).ok())
        .filter(|policy| policy.fingerprint == fingerprint)
        .map(|policy| policy.effective_rate)
        .unwrap_or(config.audit_sample_rate);
    let mut selected: BTreeSet<String> = audit::sample(&eligible, rate).into_iter().collect();
    for chunk in &eligible {
        if chunk.table_like
            || raw.get(&chunk.id).is_some_and(|(notes, escalated)| {
                *escalated || notes.facts.iter().any(|fact| fact.draft.table_like)
            })
        {
            selected.insert(chunk.id.clone());
        }
    }
    for id in selected {
        let chunk = eligible
            .iter()
            .find(|chunk| chunk.id == id)
            .context("audit chunk missing")?;
        let notes = &raw[&id].0;
        let input = serde_json::to_vec(&(
            chunk,
            notes,
            &schema,
            config.audit_sample_rate,
            config.audit_failure_threshold,
            config.agent,
            &config.models,
            &config.profile,
        ))?;
        let hash = input_hash("audit", version, &input, ceiling);
        let path = project.join(format!("state/items/audit/{id}.json"));
        let record: ItemRecord = serde_json::from_slice(
            &fs::read(path).with_context(|| format!("audit/{id} is required before --only"))?,
        )?;
        ensure!(
            is_done(project, &record, &hash) || is_blocked(&record, &hash),
            "audit/{id} is missing or stale; run without --only"
        );
    }
    Ok(())
}

pub(crate) fn expected_audit_count(project: &Path, config: &Config) -> Result<usize> {
    let chunks: Vec<Chunk> =
        serde_json::from_slice(&fs::read(project.join("state/ingest/chunks.json"))?)?;
    let (_, version) = prompts::template("audit")?;
    let mut eligible = Vec::new();
    let mut raw = BTreeMap::<String, (Notes, bool)>::new();
    let mut fingerprint_items = Vec::new();
    for chunk in &chunks {
        let record: ItemRecord = serde_json::from_slice(&fs::read(
            project.join(format!("state/items/extract/{}.json", chunk.id)),
        )?)?;
        if record.status == ItemStatus::Blocked {
            continue;
        }
        ensure!(
            is_done(project, &record, &record.input_hash),
            "extract incomplete"
        );
        let bytes = fs::read(project.join(format!("state/notes/{}.json", chunk.id)))?;
        let notes: Notes = serde_json::from_slice(&bytes)?;
        fingerprint_items.push((chunk.id.clone(), record.input_hash, hex(&bytes)));
        eligible.push(chunk.clone());
        raw.insert(chunk.id.clone(), (notes, record.escalated));
    }
    fingerprint_items.sort();
    let fingerprint = hex(&serde_json::to_vec(&(
        fingerprint_items,
        config.audit_sample_rate,
        config.audit_failure_threshold,
        version,
    ))?);
    let rate = fs::read(project.join("state/audit_policy.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<AuditPolicy>(&bytes).ok())
        .filter(|policy| policy.fingerprint == fingerprint)
        .map(|policy| policy.effective_rate)
        .unwrap_or(config.audit_sample_rate);
    let mut selected: BTreeSet<String> = audit::sample(&eligible, rate).into_iter().collect();
    for chunk in &eligible {
        if chunk.table_like
            || raw.get(&chunk.id).is_some_and(|(notes, escalated)| {
                *escalated || notes.facts.iter().any(|fact| fact.draft.table_like)
            })
        {
            selected.insert(chunk.id.clone());
        }
    }
    Ok(selected.len())
}

#[allow(clippy::too_many_arguments)]
fn run_audit_selection(
    project: &Path,
    config: &Config,
    schema: &Schema,
    options: &ScheduleOptions,
    template: &str,
    version: u32,
    eligible: &[Chunk],
    raw: &BTreeMap<String, (Notes, bool)>,
    selected: &std::collections::BTreeSet<String>,
    effective: &mut BTreeMap<String, Notes>,
    failures: &mut usize,
    completed: &mut usize,
) -> Result<()> {
    let by_id: BTreeMap<_, _> = eligible
        .iter()
        .map(|chunk| (chunk.id.clone(), chunk))
        .collect();
    let mut jobs = Vec::new();
    let mut hashes = BTreeMap::new();
    for id in selected {
        let chunk = by_id[id];
        let notes = &raw[id].0;
        let input = serde_json::to_vec(&(
            chunk,
            notes,
            schema,
            config.audit_sample_rate,
            config.audit_failure_threshold,
            config.agent,
            &config.models,
            &config.profile,
        ))?;
        let hash = input_hash("audit", version, &input, options.ceiling);
        hashes.insert(id.clone(), hash.clone());
        let record_path = project.join(format!("state/items/audit/{id}.json"));
        if let Ok(record_bytes) = fs::read(&record_path)
            && let Ok(record) = serde_json::from_slice::<ItemRecord>(&record_bytes)
        {
            if is_done(project, &record, &hash) {
                let audit: Audit = serde_json::from_slice(&fs::read(
                    project.join(format!("state/audit/{id}.json")),
                )?)?;
                let drafts = audit::apply(notes, &audit, chunk, schema)?;
                let facts = identify_facts(
                    project,
                    drafts
                        .into_iter()
                        .map(|fact| (id.clone(), chunk.source.clone(), fact))
                        .collect(),
                )?;
                effective.insert(id.clone(), Notes { facts });
                continue;
            }
            if is_blocked(&record, &hash) {
                effective.insert(id.clone(), notes.clone());
                continue;
            }
        }
        let data = serde_json::json!({"chunk": chunk, "schema": schema, "raw_notes": notes});
        jobs.push(Job {
            stage: Stage::Audit,
            item: id.clone(),
            start: Tier::Mid,
            prompt: format!(
                "{template}\nInput data (JSON):\n{}",
                serde_json::to_string(&data)?
            ),
        });
    }
    let report = scheduler::run_jobs(
        project,
        jobs,
        options,
        &|job, text| {
            let decision: Audit = serde_json::from_str(text).map_err(|e| e.to_string())?;
            audit::apply(&raw[&job.item].0, &decision, by_id[&job.item], schema)
                .map(|_| ())
                .map_err(|e| e.to_string())
        },
        &std::thread::sleep,
    )?;
    for ((stage, id), outcome) in report.outcomes {
        ensure!(stage == Stage::Audit, "unexpected scheduler stage");
        let chunk = by_id[&id];
        let notes = &raw[&id].0;
        let record_path = project.join(format!("state/items/audit/{id}.json"));
        match outcome {
            JobOutcome::Done(text) => {
                let decision: Audit = serde_json::from_str(&text)?;
                let corrected = !decision.remove.is_empty()
                    || !decision.replace.is_empty()
                    || !decision.add.is_empty();
                *completed += 1;
                if corrected {
                    *failures += 1;
                }
                let bytes = serde_json::to_vec_pretty(&decision)?;
                let relative = format!("state/audit/{id}.json");
                atomic_write(&project.join(&relative), &bytes)?;
                save_record(
                    project,
                    &record_path,
                    hashes[&id].clone(),
                    &relative,
                    &bytes,
                    ItemStatus::Done,
                    vec![],
                )?;
                let drafts = audit::apply(notes, &decision, chunk, schema)?;
                let facts = identify_facts(
                    project,
                    drafts
                        .into_iter()
                        .map(|fact| (id.clone(), chunk.source.clone(), fact))
                        .collect(),
                )?;
                effective.insert(id, Notes { facts });
            }
            JobOutcome::Blocked(reason) => {
                *completed += 1;
                *failures += 1;
                save_record(
                    project,
                    &record_path,
                    hashes[&id].clone(),
                    "",
                    &[],
                    ItemStatus::Blocked,
                    vec![reason],
                )?;
                effective.insert(id, notes.clone());
            }
            JobOutcome::Pending => {
                save_record(
                    project,
                    &record_path,
                    hashes[&id].clone(),
                    "",
                    &[],
                    ItemStatus::Pending,
                    vec!["audit pending".into()],
                )?;
                effective.insert(id, notes.clone());
            }
        }
    }
    Ok(())
}

pub fn run_resolve(
    project: &Path,
    config: &Config,
    effective: &BTreeMap<String, Notes>,
    options: &ScheduleOptions,
) -> Result<Vec<Entity>> {
    let facts: Vec<Fact> = effective
        .values()
        .flat_map(|notes| notes.facts.clone())
        .collect();
    let provisional = resolve::group_exact(&facts);
    let by_id: BTreeMap<_, _> = provisional
        .iter()
        .map(|entity| (entity.id.clone(), entity.clone()))
        .collect();
    let fact_map: BTreeMap<_, _> = facts
        .iter()
        .map(|fact| (fact.id.clone(), fact.clone()))
        .collect();
    let (pair_template, pair_version) = prompts::template("resolve_pair")?;
    fs::create_dir_all(project.join("state/resolve/pairs"))?;
    fs::create_dir_all(project.join("state/resolve/clusters"))?;
    fs::create_dir_all(project.join("state/items/resolve"))?;
    let mut decisions = BTreeMap::<(String, String), PairDecision>::new();
    let mut pair_hashes = BTreeMap::new();
    let mut jobs = Vec::new();
    for (left_id, right_id) in resolve::candidates(&provisional) {
        let left = &by_id[&left_id];
        let right = &by_id[&right_id];
        let evidence: Vec<_> = left
            .fact_ids
            .iter()
            .chain(right.fact_ids.iter())
            .filter_map(|id| fact_map.get(id))
            .cloned()
            .collect();
        let input = serde_json::to_vec(&(
            left,
            right,
            &evidence,
            config.agent,
            &config.models,
            &config.profile,
        ))?;
        let hash = input_hash("resolve_pair", pair_version, &input, options.ceiling);
        let pair_id = format!("{left_id}_{right_id}");
        pair_hashes.insert(pair_id.clone(), hash.clone());
        let relative = format!("state/resolve/pairs/{pair_id}.json");
        let record_path = project.join(format!("state/items/resolve/{pair_id}.json"));
        if let Some(decision) =
            cached_json::<PairDecision>(project, &record_path, &hash, &relative)?
        {
            decisions.insert((left_id, right_id), decision);
            continue;
        }
        if fs::read(&record_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<ItemRecord>(&bytes).ok())
            .is_some_and(|record| is_blocked(&record, &hash))
        {
            add_gap(project, &format!("resolve/{pair_id}: blocked"))?;
            continue;
        }
        let data = serde_json::json!({"left":left,"right":right,"facts":evidence});
        jobs.push(Job {
            stage: Stage::Resolve,
            item: pair_id,
            start: Tier::Small,
            prompt: format!(
                "{pair_template}\nInput data (JSON):\n{}",
                serde_json::to_string(&data)?
            ),
        });
    }
    let pair_report = scheduler::run_jobs(
        project,
        jobs,
        options,
        &|job, text| {
            let decision: PairDecision = serde_json::from_str(text).map_err(|e| e.to_string())?;
            let (left, right) = job
                .item
                .split_once('_')
                .ok_or_else(|| "invalid pair ID".to_owned())?;
            let errors = resolve::validate_pair(&decision, &by_id[left], &by_id[right]);
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors.join("; "))
            }
        },
        &std::thread::sleep,
    )?;
    for ((_, pair_id), outcome) in pair_report.outcomes {
        let (left, right) = pair_id.split_once('_').context("invalid pair ID")?;
        let record_path = project.join(format!("state/items/resolve/{pair_id}.json"));
        match outcome {
            JobOutcome::Done(text) => {
                let decision: PairDecision = serde_json::from_str(&text)?;
                let relative = format!("state/resolve/pairs/{pair_id}.json");
                let bytes = serde_json::to_vec_pretty(&decision)?;
                atomic_write(&project.join(&relative), &bytes)?;
                save_record(
                    project,
                    &record_path,
                    pair_hashes[&pair_id].clone(),
                    &relative,
                    &bytes,
                    ItemStatus::Done,
                    vec![],
                )?;
                decisions.insert((left.into(), right.into()), decision);
            }
            JobOutcome::Blocked(reason) => {
                save_record(
                    project,
                    &record_path,
                    pair_hashes[&pair_id].clone(),
                    "",
                    &[],
                    ItemStatus::Blocked,
                    vec![reason],
                )?;
                add_gap(project, &format!("resolve/{pair_id}: blocked"))?;
            }
            JobOutcome::Pending => {
                save_record(
                    project,
                    &record_path,
                    pair_hashes[&pair_id].clone(),
                    "",
                    &[],
                    ItemStatus::Pending,
                    vec!["resolve pending".into()],
                )?;
                add_gap(project, &format!("resolve/{pair_id}: pending"))?;
            }
        }
    }
    let mut adjacency = BTreeMap::<String, BTreeSet<String>>::new();
    for id in by_id.keys() {
        adjacency.entry(id.clone()).or_default();
    }
    for ((left, right), decision) in &decisions {
        if decision.verdict != PairVerdict::No {
            adjacency
                .entry(left.clone())
                .or_default()
                .insert(right.clone());
            adjacency
                .entry(right.clone())
                .or_default()
                .insert(left.clone());
        }
    }
    let mut seen = BTreeSet::new();
    let mut components = Vec::new();
    for id in by_id.keys() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let mut stack = vec![id.clone()];
        let mut members = Vec::new();
        while let Some(next) = stack.pop() {
            members.push(next.clone());
            for neighbor in &adjacency[&next] {
                if seen.insert(neighbor.clone()) {
                    stack.push(neighbor.clone());
                }
            }
        }
        members.sort();
        components.push(members);
    }
    let (cluster_template, cluster_version) = prompts::template("resolve_cluster")?;
    let mut cluster_jobs = Vec::new();
    let mut cluster_hashes = BTreeMap::new();
    let mut groups = Vec::<Vec<String>>::new();
    let mut cluster_members = BTreeMap::<String, Vec<Entity>>::new();
    for component in components {
        if component.len() == 1 {
            groups.push(component);
            continue;
        }
        let clique = component.iter().enumerate().all(|(index, left)| {
            component.iter().skip(index + 1).all(|right| {
                decisions
                    .get(&(left.clone(), right.clone()))
                    .is_some_and(|d| d.verdict == PairVerdict::Yes)
            })
        });
        if clique {
            groups.push(component);
            continue;
        }
        let item = component[0].clone();
        let members: Vec<_> = component.iter().map(|id| by_id[id].clone()).collect();
        let member_facts: Vec<_> = members
            .iter()
            .flat_map(|entity| {
                entity
                    .fact_ids
                    .iter()
                    .filter_map(|id| fact_map.get(id))
                    .cloned()
            })
            .collect();
        let related: Vec<_> = decisions
            .iter()
            .filter(|((left, right), _)| component.contains(left) && component.contains(right))
            .collect();
        let input = serde_json::to_vec(&(
            &members,
            &member_facts,
            &related,
            config.agent,
            &config.models,
            &config.profile,
        ))?;
        let hash = input_hash("resolve_cluster", cluster_version, &input, options.ceiling);
        cluster_hashes.insert(item.clone(), hash.clone());
        let relative = format!("state/resolve/clusters/{item}.json");
        let record_path = project.join(format!("state/items/resolve/cluster_{item}.json"));
        if let Some(decision) =
            cached_json::<ClusterDecision>(project, &record_path, &hash, &relative)?
        {
            groups.extend(decision.groups);
            continue;
        }
        if fs::read(&record_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<ItemRecord>(&bytes).ok())
            .is_some_and(|record| is_blocked(&record, &hash))
        {
            add_gap(project, &format!("resolve/cluster_{item}: blocked"))?;
            groups.extend(members.iter().map(|entity| vec![entity.id.clone()]));
            continue;
        }
        cluster_members.insert(item.clone(), members.clone());
        let data =
            serde_json::json!({"entities":members,"facts":member_facts,"pair_decisions":related});
        cluster_jobs.push(Job {
            stage: Stage::Resolve,
            item,
            start: Tier::Mid,
            prompt: format!(
                "{cluster_template}\nInput data (JSON):\n{}",
                serde_json::to_string(&data)?
            ),
        });
    }
    let cluster_report = scheduler::run_jobs(
        project,
        cluster_jobs,
        options,
        &|job, text| {
            let decision: ClusterDecision =
                serde_json::from_str(text).map_err(|e| e.to_string())?;
            let members = &cluster_members[&job.item];
            let ids: BTreeSet<_> = members.iter().map(|entity| entity.id.as_str()).collect();
            let no_pairs: Vec<_> = decisions
                .iter()
                .filter(|((left, right), decision)| {
                    decision.verdict == PairVerdict::No
                        && ids.contains(left.as_str())
                        && ids.contains(right.as_str())
                })
                .map(|((left, right), _)| (left.clone(), right.clone()))
                .collect();
            let errors = resolve::validate_cluster(&decision, members, &no_pairs);
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors.join("; "))
            }
        },
        &std::thread::sleep,
    )?;
    for ((_, item), outcome) in cluster_report.outcomes {
        let record_path = project.join(format!("state/items/resolve/cluster_{item}.json"));
        match outcome {
            JobOutcome::Done(text) => {
                let decision: ClusterDecision = serde_json::from_str(&text)?;
                let relative = format!("state/resolve/clusters/{item}.json");
                let bytes = serde_json::to_vec_pretty(&decision)?;
                atomic_write(&project.join(&relative), &bytes)?;
                save_record(
                    project,
                    &record_path,
                    cluster_hashes[&item].clone(),
                    &relative,
                    &bytes,
                    ItemStatus::Done,
                    vec![],
                )?;
                groups.extend(decision.groups);
            }
            JobOutcome::Blocked(reason) => {
                save_record(
                    project,
                    &record_path,
                    cluster_hashes[&item].clone(),
                    "",
                    &[],
                    ItemStatus::Blocked,
                    vec![reason],
                )?;
                groups.extend(
                    cluster_members[&item]
                        .iter()
                        .map(|entity| vec![entity.id.clone()]),
                );
                add_gap(project, &format!("resolve/cluster_{item}: blocked"))?;
            }
            JobOutcome::Pending => {
                save_record(
                    project,
                    &record_path,
                    cluster_hashes[&item].clone(),
                    "",
                    &[],
                    ItemStatus::Pending,
                    vec!["resolve pending".into()],
                )?;
                groups.extend(
                    cluster_members[&item]
                        .iter()
                        .map(|entity| vec![entity.id.clone()]),
                );
                add_gap(project, &format!("resolve/cluster_{item}: pending"))?;
            }
        }
    }
    let mut entities = Vec::new();
    for group in groups {
        let mut members: Vec<_> = group.iter().map(|id| by_id[id].clone()).collect();
        members.sort_by(|a, b| a.id.cmp(&b.id));
        let mut fact_ids: Vec<_> = members
            .iter()
            .flat_map(|entity| entity.fact_ids.clone())
            .collect();
        fact_ids.sort();
        fact_ids.dedup();
        let mut aliases: Vec<_> = members
            .iter()
            .flat_map(|entity| entity.aliases.clone())
            .collect();
        aliases.sort();
        aliases.dedup();
        let first = &members[0];
        let id = fact_ids[0].replacen('f', "e", 1);
        entities.push(Entity {
            id,
            name: first.name.clone(),
            entity_type: first.entity_type.clone(),
            aliases,
            fact_ids,
        });
    }
    entities.sort_by(|a, b| a.id.cmp(&b.id));
    let bytes = serde_json::to_vec_pretty(&entities)?;
    atomic_write(&project.join("state/entities.json"), &bytes)?;
    save_record(
        project,
        &project.join("state/items/resolve/global.json"),
        input_hash(
            "resolve_global",
            1,
            &serde_json::to_vec(&(effective, decisions.iter().collect::<Vec<_>>()))?,
            options.ceiling,
        ),
        "state/entities.json",
        &bytes,
        ItemStatus::Done,
        vec![],
    )?;
    Ok(entities)
}

fn verify_resolve_current(project: &Path, config: &Config, ceiling: Tier) -> Result<()> {
    let effective = get_effective(project, &mut None)?.clone();
    let facts: Vec<Fact> = effective
        .values()
        .flat_map(|notes| notes.facts.clone())
        .collect();
    let provisional = resolve::group_exact(&facts);
    let by_id: BTreeMap<_, _> = provisional
        .iter()
        .map(|entity| (entity.id.clone(), entity))
        .collect();
    let fact_map: BTreeMap<_, _> = facts.iter().map(|fact| (fact.id.clone(), fact)).collect();
    let (_, pair_version) = prompts::template("resolve_pair")?;
    let mut decisions = BTreeMap::<(String, String), PairDecision>::new();
    for (left_id, right_id) in resolve::candidates(&provisional) {
        let left = by_id[&left_id];
        let right = by_id[&right_id];
        let evidence: Vec<_> = left
            .fact_ids
            .iter()
            .chain(right.fact_ids.iter())
            .filter_map(|id| fact_map.get(id))
            .cloned()
            .collect();
        let input = serde_json::to_vec(&(
            left,
            right,
            &evidence,
            config.agent,
            &config.models,
            &config.profile,
        ))?;
        let hash = input_hash("resolve_pair", pair_version, &input, ceiling);
        let pair_id = format!("{left_id}_{right_id}");
        let record: ItemRecord = serde_json::from_slice(
            &fs::read(project.join(format!("state/items/resolve/{pair_id}.json")))
                .with_context(|| format!("resolve/{pair_id} is required before --only"))?,
        )?;
        if is_done(project, &record, &hash) {
            let decision: PairDecision = serde_json::from_slice(&fs::read(
                project.join(format!("state/resolve/pairs/{pair_id}.json")),
            )?)?;
            decisions.insert((left_id, right_id), decision);
        } else {
            ensure!(
                is_blocked(&record, &hash),
                "resolve/{pair_id} is missing or stale; run without --only"
            );
        }
    }
    let hash = input_hash(
        "resolve_global",
        1,
        &serde_json::to_vec(&(&effective, decisions.iter().collect::<Vec<_>>()))?,
        ceiling,
    );
    let record: ItemRecord = serde_json::from_slice(
        &fs::read(project.join("state/items/resolve/global.json"))
            .context("resolve is required before --only")?,
    )?;
    ensure!(
        is_done(project, &record, &hash),
        "resolve entities are missing or stale; run without --only"
    );
    Ok(())
}

pub fn run_organize(
    project: &Path,
    config: &Config,
    schema: &Schema,
    entities: &[Entity],
    facts: &[Fact],
    gaps: &[String],
    options: &ScheduleOptions,
) -> Result<Option<Outline>> {
    let (template, version) = prompts::template("organize")?;
    let compact: Vec<_> = entities
        .iter()
        .map(|entity| {
            serde_json::json!({"id":entity.id,"name":entity.name,
        "entity_type":entity.entity_type,"aliases":entity.aliases,"fact_ids":entity.fact_ids})
        })
        .collect();
    let input = serde_json::to_vec(&(
        &compact,
        &schema.namespaces,
        gaps,
        config.agent,
        &config.models,
        &config.profile,
    ))?;
    let hash = input_hash("organize", version, &input, options.ceiling);
    let record_path = project.join("state/items/organize/global.json");
    if let Some(outline) =
        cached_yaml::<Outline>(project, &record_path, &hash, "state/outline.yaml")?
    {
        return Ok(Some(outline));
    }
    if fs::read(&record_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<ItemRecord>(&bytes).ok())
        .is_some_and(|record| is_blocked(&record, &hash))
    {
        return Ok(None);
    }
    let outline = if entities.is_empty() {
        Some(Outline {
            pages: vec![PageSpec {
                id: "start".into(),
                title: config.project.clone(),
                kind: PageKind::Index,
                entity_ids: vec![],
                fact_ids: vec![],
            }],
            gaps: gaps.to_vec(),
        })
    } else if entities.len() > 500 {
        let outline = deterministic_outline(config, schema, entities, gaps);
        let errors = validate::validate_outline(&outline, schema, entities, facts);
        ensure!(errors.is_empty(), "generated outline invalid: {}", errors.join("; "));
        Some(outline)
    } else {
        let data =
            serde_json::json!({"entities":compact,"namespaces":schema.namespaces,"gaps":gaps});
        let job = Job {
            stage: Stage::Organize,
            item: "global".into(),
            start: Tier::Large,
            prompt: format!(
                "{template}\nInput data (JSON):\n{}",
                serde_json::to_string(&data)?
            ),
        };
        let report = scheduler::run_jobs(
            project,
            vec![job],
            options,
            &|_, text| {
                let outline: Outline = serde_yaml::from_str(text).map_err(|e| e.to_string())?;
                let errors = validate::validate_outline(&outline, schema, entities, facts);
                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(errors.join("; "))
                }
            },
            &std::thread::sleep,
        )?;
        match report.outcomes.get(&(Stage::Organize, "global".into())) {
            Some(JobOutcome::Done(text)) => Some(serde_yaml::from_str(text)?),
            Some(JobOutcome::Blocked(reason)) => {
                save_record(
                    project,
                    &record_path,
                    hash.clone(),
                    "",
                    &[],
                    ItemStatus::Blocked,
                    vec![reason.clone()],
                )?;
                None
            }
            _ => None,
        }
    };
    if let Some(outline) = &outline {
        let bytes = serde_yaml::to_string(outline)?.into_bytes();
        atomic_write(&project.join("state/outline.yaml"), &bytes)?;
        save_record(
            project,
            &record_path,
            hash,
            "state/outline.yaml",
            &bytes,
            ItemStatus::Done,
            vec![],
        )?;
    }
    Ok(outline)
}

fn verify_organize_current(project: &Path, config: &Config, ceiling: Tier) -> Result<()> {
    let schema: Schema = serde_yaml::from_slice(&fs::read(project.join("state/schema.yaml"))?)?;
    let entities: Vec<Entity> =
        serde_json::from_slice(&fs::read(project.join("state/entities.json"))?)?;
    let outline: Outline = serde_yaml::from_slice(
        &fs::read(project.join("state/outline.yaml"))
            .context("organize is required before --only")?,
    )?;
    let compact: Vec<_> = entities
        .iter()
        .map(|entity| {
            serde_json::json!({"id":entity.id,"name":entity.name,
            "entity_type":entity.entity_type,"aliases":entity.aliases,"fact_ids":entity.fact_ids})
        })
        .collect();
    let (_, version) = prompts::template("organize")?;
    let input = serde_json::to_vec(&(
        &compact,
        &schema.namespaces,
        &outline.gaps,
        config.agent,
        &config.models,
        &config.profile,
    ))?;
    let hash = input_hash("organize", version, &input, ceiling);
    let record: ItemRecord = serde_json::from_slice(
        &fs::read(project.join("state/items/organize/global.json"))
            .context("organize is required before --only")?,
    )?;
    ensure!(
        is_done(project, &record, &hash),
        "organize outline is missing or stale; run without --only"
    );
    Ok(())
}

fn cached_json<T: serde::de::DeserializeOwned>(
    project: &Path,
    record_path: &Path,
    hash: &str,
    relative: &str,
) -> Result<Option<T>> {
    let Ok(bytes) = fs::read(record_path) else {
        return Ok(None);
    };
    let Ok(record) = serde_json::from_slice::<ItemRecord>(&bytes) else {
        return Ok(None);
    };
    if !is_done(project, &record, hash) {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&fs::read(
        project.join(relative),
    )?)?))
}

fn cached_yaml<T: serde::de::DeserializeOwned>(
    project: &Path,
    record_path: &Path,
    hash: &str,
    relative: &str,
) -> Result<Option<T>> {
    let Ok(bytes) = fs::read(record_path) else {
        return Ok(None);
    };
    let Ok(record) = serde_json::from_slice::<ItemRecord>(&bytes) else {
        return Ok(None);
    };
    if !is_done(project, &record, hash) {
        return Ok(None);
    }
    Ok(Some(serde_yaml::from_slice(&fs::read(
        project.join(relative),
    )?)?))
}

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub jobs: Option<usize>,
    pub until: Option<Stage>,
    pub only: Option<Stage>,
    pub ceiling: Tier,
    pub yes: bool,
    pub budget_tokens: Option<u64>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RunReport {
    pub done: usize,
    pub pending: usize,
    pub blocked: usize,
    pub needs_acceptance: bool,
    pub tokens: u64,
}

pub fn run(project: &Path, options: &RunOptions) -> Result<RunReport> {
    run_inner(project, options, None)
}

pub fn run_fake(project: &Path, options: &RunOptions, fixtures: &Path) -> Result<RunReport> {
    run_inner(project, options, Some(fixtures))
}

fn run_inner(project: &Path, options: &RunOptions, fake_root: Option<&Path>) -> Result<RunReport> {
    ensure!(
        options.only.is_none() || options.until.is_none(),
        "--only and --until are mutually exclusive"
    );
    let project = fs::canonicalize(project).context("project directory missing")?;
    let _lock = crate::state::lock(&project)?;
    let config = crate::config::load(&project)?;
    crate::project::verify_manifest(&project, &config)?;
    let jobs = options.jobs.unwrap_or(config.jobs);
    ensure!((1..=64).contains(&jobs), "jobs must be 1..64");
    let run_id = format!(
        "{}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos(),
        std::process::id()
    );
    let stages = [
        Stage::Ingest,
        Stage::Survey,
        Stage::Extract,
        Stage::Audit,
        Stage::Resolve,
        Stage::Organize,
        Stage::Write,
        Stage::Link,
        Stage::Review,
    ];
    let selected: Vec<_> = stages
        .iter()
        .copied()
        .filter(|stage| {
            if let Some(only) = options.only {
                *stage == only
            } else if let Some(until) = options.until {
                *stage <= until
            } else {
                true
            }
        })
        .collect();
    if options.only.is_some_and(|stage| stage != Stage::Ingest) {
        crate::ingest::verify_current(&project, &config)?;
    }
    if options.only.is_some_and(|stage| stage > Stage::Survey) {
        verify_survey_current(&project, &config, options.ceiling)?;
    }
    if options.only.is_some_and(|stage| stage > Stage::Extract) {
        verify_extract_current(&project, &config, options.ceiling)?;
    }
    if options.only.is_some_and(|stage| stage > Stage::Audit) {
        verify_audit_current(&project, &config, options.ceiling)?;
    }
    if options.only.is_some_and(|stage| stage > Stage::Resolve) {
        verify_resolve_current(&project, &config, options.ceiling)?;
    }
    if options.only.is_some_and(|stage| stage > Stage::Organize) {
        verify_organize_current(&project, &config, options.ceiling)?;
    }
    consume_redo_queue(&project, &selected)?;
    let mut chunks: Option<Vec<Chunk>> = None;
    let mut schema: Option<Schema> = None;
    let mut effective: Option<BTreeMap<String, Notes>> = None;
    let mut entities: Option<Vec<Entity>> = None;
    let mut outline: Option<Outline> = None;
    let mut needs_acceptance = false;
    let budget = options.budget_tokens.unwrap_or(u64::MAX);
    for stage in selected.iter().copied() {
        if stage != Stage::Ingest {
            clear_stage_gaps(&project, stage)?;
        }
        let charged = charged_run_tokens(&project, &run_id)?;
        let schedule = ScheduleOptions {
            provider: if fake_root.is_some() {
                crate::types::Provider::Fake
            } else {
                config.agent
            },
            fake_root: fake_root.map(Path::to_path_buf),
            models: config.models.clone(),
            ceiling: options.ceiling,
            budget_tokens: budget.saturating_sub(charged),
            jobs,
            timeout: std::time::Duration::from_secs(config.timeout_seconds),
            run: run_id.clone(),
        };
        if options.only.is_some() && stage != Stage::Ingest {
            let available = get_chunks(&project, &mut chunks)?;
            let toc: Vec<TocEntry> =
                serde_json::from_slice(&fs::read(project.join("state/ingest/toc.json"))?)?;
            if !acceptance(&project, &config, available, &toc, options)? {
                needs_acceptance = true;
                break;
            }
        }
        match stage {
            Stage::Ingest => {
                chunks = Some(crate::ingest::ingest(&project, &config)?);
            }
            Stage::Survey => {
                let available = get_chunks(&project, &mut chunks)?;
                let toc: Vec<TocEntry> =
                    serde_json::from_slice(&fs::read(project.join("state/ingest/toc.json"))?)?;
                if !acceptance(&project, &config, available, &toc, options)? {
                    needs_acceptance = true;
                    break;
                }
                schema = run_survey(&project, &config, &toc, available, &schedule)?;
                if schema.is_none() {
                    add_gap(&project, "survey/global: schema unavailable")?;
                    break;
                }
            }
            Stage::Extract => {
                let available = get_chunks(&project, &mut chunks)?.clone();
                let schema = get_schema(&project, &mut schema)?;
                run_extract(&project, &config, &available, schema, &schedule)?;
                for chunk in &available {
                    let path = project.join(format!("state/items/extract/{}.json", chunk.id));
                    let status = fs::read(path)
                        .ok()
                        .and_then(|bytes| serde_json::from_slice::<ItemRecord>(&bytes).ok())
                        .map(|record| record.status);
                    if status != Some(ItemStatus::Done) {
                        add_gap(
                            &project,
                            &format!(
                                "extract/{}: {}",
                                chunk.id,
                                if status == Some(ItemStatus::Blocked) {
                                    "blocked"
                                } else {
                                    "pending"
                                }
                            ),
                        )?;
                    }
                }
            }
            Stage::Audit => {
                let available = get_chunks(&project, &mut chunks)?.clone();
                let schema = get_schema(&project, &mut schema)?;
                effective = Some(run_audit(&project, &config, &available, schema, &schedule)?);
                for chunk in &available {
                    let path = project.join(format!("state/items/audit/{}.json", chunk.id));
                    if let Some(status) = fs::read(path)
                        .ok()
                        .and_then(|bytes| serde_json::from_slice::<ItemRecord>(&bytes).ok())
                        .map(|record| record.status)
                    {
                        match status {
                            ItemStatus::Blocked => add_gap(
                                &project,
                                &format!("audit/{}: blocked; raw facts retained", chunk.id),
                            )?,
                            ItemStatus::Pending => add_gap(
                                &project,
                                &format!("audit/{}: pending; raw facts retained", chunk.id),
                            )?,
                            ItemStatus::Done => {}
                        }
                    }
                }
            }
            Stage::Resolve => {
                let effective = get_effective(&project, &mut effective)?;
                entities = Some(run_resolve(&project, &config, effective, &schedule)?);
            }
            Stage::Organize => {
                let schema = get_schema(&project, &mut schema)?;
                let effective = get_effective(&project, &mut effective)?;
                let facts: Vec<Fact> = effective
                    .values()
                    .flat_map(|notes| notes.facts.clone())
                    .collect();
                let entities = get_entities(&project, &mut entities)?;
                let gaps = load_gaps(&project);
                outline = run_organize(
                    &project, &config, schema, entities, &facts, &gaps, &schedule,
                )?;
                if outline.is_none() {
                    add_gap(&project, "organize/global: outline unavailable")?;
                    break;
                }
            }
            Stage::Write => {
                let outline = get_outline(&project, &mut outline)?;
                let effective = get_effective(&project, &mut effective)?;
                let facts: Vec<Fact> = effective
                    .values()
                    .flat_map(|notes| notes.facts.clone())
                    .collect();
                let entities = get_entities(&project, &mut entities)?;
                let current_pages = crate::write::write_pages(
                    &project, &config, outline, entities, &facts, &schedule,
                )?;
                for page in &outline.pages {
                    if !current_pages.contains(&page.id) {
                        add_gap(&project, &format!("write/{}: blocked or pending", page.id))?;
                    }
                }
            }
            Stage::Link => {
                let outline = get_outline(&project, &mut outline)?;
                let entities = get_entities(&project, &mut entities)?;
                let report = crate::link::link(&project, outline, entities)?;
                if !report.broken_links.is_empty() || !report.missing_citations.is_empty() {
                    add_gap(&project, "link/global: lint errors")?;
                }
            }
            Stage::Review => {
                let outline = get_outline(&project, &mut outline)?;
                let effective = get_effective(&project, &mut effective)?;
                let facts: Vec<Fact> = effective
                    .values()
                    .flat_map(|notes| notes.facts.clone())
                    .collect();
                let chunks = get_chunks(&project, &mut chunks)?;
                let chunk_ids: Vec<String> = chunks.iter().map(|chunk| chunk.id.clone()).collect();
                crate::review::run_review(
                    &project, &config, outline, &facts, &chunk_ids, &schedule,
                )?;
            }
        }
    }
    let tokens = charged_run_tokens(&project, &run_id)?;
    let (done, pending, blocked) = count_records(&project, &selected)?;
    let gap_count = load_gaps(&project).len();
    clear_completed_forces(&project)?;
    Ok(RunReport {
        done,
        pending: pending
            + if gap_count > 0 && pending == 0 && blocked == 0 {
                1
            } else {
                0
            },
        blocked,
        needs_acceptance,
        tokens,
    })
}

fn consume_redo_queue(project: &Path, selected: &[Stage]) -> Result<()> {
    let path = project.join("state/redo_queue.json");
    let mut queue: Vec<crate::review::Finding> = fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    if queue.is_empty() {
        return Ok(());
    }
    let forced_path = project.join("state/forced.json");
    let mut forced = load_forced(project)?;
    let published = crate::write::published_hashes(project)?;
    let mut remaining = Vec::new();
    for finding in queue.drain(..) {
        if finding.stage == Stage::Write
            && crate::write::content_is_edited(project, &finding.item, &published)?
        {
            eprintln!(
                "warning: redo/write/{}: page has human edits; dropping automatic redo",
                finding.item
            );
            continue;
        }
        if !selected.contains(&finding.stage) {
            remaining.push(finding);
            continue;
        }
        let relative = match finding.stage {
            Stage::Extract => format!("state/items/extract/{}.json", finding.item),
            Stage::Write => format!("state/items/write/{}.json", finding.item.replace(':', "/")),
            _ => anyhow::bail!("invalid redo queue stage"),
        };
        let record_path = project.join(relative);
        let Ok(bytes) = fs::read(&record_path) else {
            eprintln!(
                "warning: redo/{}/{}: target missing; dropping stale finding",
                crate::status::stage_name(finding.stage),
                finding.item
            );
            continue;
        };
        let Ok(mut record) = serde_json::from_slice::<ItemRecord>(&bytes) else {
            eprintln!(
                "warning: redo/{}/{}: target record invalid; dropping stale finding",
                crate::status::stage_name(finding.stage),
                finding.item
            );
            continue;
        };
        record.input_hash.clear();
        record.status = ItemStatus::Pending;
        atomic_write(&record_path, &serde_json::to_vec_pretty(&record)?)?;
        forced.insert(format!(
            "{}/{}",
            crate::status::stage_name(finding.stage),
            finding.item
        ));
        atomic_write(&forced_path, &serde_json::to_vec_pretty(&forced)?)?;
    }
    atomic_write(&path, &serde_json::to_vec_pretty(&remaining)?)?;
    Ok(())
}

fn clear_completed_forces(project: &Path) -> Result<()> {
    let path = project.join("state/forced.json");
    let mut forced = load_forced(project)?;
    let original = forced.len();
    forced.retain(|entry| {
        let Some((stage, item)) = entry.split_once('/') else {
            return true;
        };
        let relative = if stage == "write" {
            item.replace(':', "/")
        } else {
            item.into()
        };
        let path = project
            .join("state/items")
            .join(stage)
            .join(relative)
            .with_extension("json");
        fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<ItemRecord>(&bytes).ok())
            .is_none_or(|record| record.status != ItemStatus::Done)
    });
    if forced.len() != original {
        atomic_write(&path, &serde_json::to_vec_pretty(&forced)?)?;
    }
    Ok(())
}

fn load_forced(project: &Path) -> Result<BTreeSet<String>> {
    match fs::read(project.join("state/forced.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes).context("invalid state/forced.json"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(BTreeSet::new()),
        Err(error) => Err(error.into()),
    }
}

fn get_chunks<'a>(project: &Path, cache: &'a mut Option<Vec<Chunk>>) -> Result<&'a Vec<Chunk>> {
    if cache.is_none() {
        *cache = Some(serde_json::from_slice(
            &fs::read(project.join("state/ingest/chunks.json"))
                .context("ingest is required first")?,
        )?);
    }
    Ok(cache.as_ref().unwrap())
}
fn get_schema<'a>(project: &Path, cache: &'a mut Option<Schema>) -> Result<&'a Schema> {
    if cache.is_none() {
        *cache = Some(serde_yaml::from_slice(
            &fs::read(project.join("state/schema.yaml"))
                .context("survey schema is required first")?,
        )?);
    }
    Ok(cache.as_ref().unwrap())
}
fn get_effective<'a>(
    project: &Path,
    cache: &'a mut Option<BTreeMap<String, Notes>>,
) -> Result<&'a BTreeMap<String, Notes>> {
    if cache.is_none() {
        let mut notes = BTreeMap::new();
        let chunks: Vec<Chunk> = serde_json::from_slice(
            &fs::read(project.join("state/ingest/chunks.json"))
                .context("ingest is required before effective notes")?,
        )?;
        for chunk in &chunks {
            let record_path = project.join(format!("state/items/extract/{}.json", chunk.id));
            let record: ItemRecord =
                serde_json::from_slice(&fs::read(record_path).with_context(|| {
                    format!("extract/{} is required before effective notes", chunk.id)
                })?)?;
            if record.status == ItemStatus::Blocked {
                continue;
            }
            ensure!(
                is_done(project, &record, &record.input_hash),
                "extract/{} is incomplete before effective notes",
                chunk.id
            );
            let path = project.join(format!("state/effective/{}.json", chunk.id));
            let effective: Notes =
                serde_json::from_slice(&fs::read(path).with_context(|| {
                    format!("effective/{} is missing; run audit first", chunk.id)
                })?)?;
            notes.insert(chunk.id.clone(), effective);
        }
        *cache = Some(notes);
    }
    Ok(cache.as_ref().unwrap())
}
fn get_entities<'a>(project: &Path, cache: &'a mut Option<Vec<Entity>>) -> Result<&'a Vec<Entity>> {
    if cache.is_none() {
        *cache = Some(serde_json::from_slice(
            &fs::read(project.join("state/entities.json"))
                .context("resolved entities are required first")?,
        )?);
    }
    Ok(cache.as_ref().unwrap())
}
fn get_outline<'a>(project: &Path, cache: &'a mut Option<Outline>) -> Result<&'a Outline> {
    if cache.is_none() {
        *cache = Some(serde_yaml::from_slice(
            &fs::read(project.join("state/outline.yaml")).context("outline is required first")?,
        )?);
    }
    Ok(cache.as_ref().unwrap())
}

fn acceptance(
    project: &Path,
    config: &Config,
    chunks: &[Chunk],
    toc: &[TocEntry],
    options: &RunOptions,
) -> Result<bool> {
    let mut semantic = config.clone();
    semantic.jobs = 0;
    semantic.timeout_seconds = 0;
    let templates = [
        "survey",
        "extract",
        "audit",
        "resolve_pair",
        "resolve_cluster",
        "organize",
        "write",
        "review",
    ];
    let versions: Vec<_> = templates
        .iter()
        .map(|name| Ok((*name, prompts::template(name)?.1)))
        .collect::<Result<_>>()?;
    let fingerprint = hex(&serde_json::to_vec(&(
        chunks,
        toc,
        semantic,
        versions,
        options.ceiling,
    ))?);
    let path = project.join("state/acceptance.json");
    let prior: Option<String> = fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    print_estimates(chunks, toc);
    if prior.as_deref() == Some(&fingerprint) {
        return Ok(true);
    }
    if options.yes {
        atomic_write(&path, &serde_json::to_vec(&fingerprint)?)?;
        return Ok(true);
    }
    println!("Acceptance required for this input and model configuration. Rerun with --yes.");
    Ok(false)
}

fn print_estimates(chunks: &[Chunk], toc: &[TocEntry]) {
    let n = chunks.len() as u64;
    let survey = serde_json::to_string(&(toc, crate::pipeline::survey_inputs(chunks)))
        .unwrap_or_default()
        .chars()
        .count()
        .div_ceil(4) as u64
        + 1500;
    let extract: u64 = chunks
        .iter()
        .map(|chunk| (chunk.text.chars().count() as u64).div_ceil(4) + 1500)
        .sum();
    println!(
        "Estimated tokens: survey {survey}, extract {extract}, audit {}, resolve {}, organize {}, write {}, review {}.",
        n.div_ceil(10) * 2000,
        n.saturating_sub(1) * 500,
        n * 200 + 1500,
        n * 3000,
        n.min(5) * 1500 + 1000
    );
    println!(
        "Assumptions: estimated pages and adjacent pairs; retries and escalation can add usage."
    );
}

fn charged_run_tokens(project: &Path, run: &str) -> Result<u64> {
    let path = project.join("state/ledger.jsonl");
    let Ok(bytes) = fs::read(path) else {
        return Ok(0);
    };
    let complete = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let mut total = 0u64;
    for line in bytes[..complete]
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let entry: crate::scheduler::LedgerEntry = serde_json::from_slice(line)?;
        if entry.run == run {
            total = total.saturating_add(entry.input_tokens + entry.output_tokens);
        }
    }
    Ok(total)
}

fn count_records(project: &Path, selected: &[Stage]) -> Result<(usize, usize, usize)> {
    let mut counts = (0, 0, 0);
    let active_sources: BTreeSet<String> = crate::config::load(project)?
        .sources
        .into_iter()
        .map(|source| source.id)
        .collect();
    let active_chunks: BTreeSet<String> = fs::read(project.join("state/ingest/chunks.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Vec<Chunk>>(&bytes).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|chunk| chunk.id)
        .collect();
    let active_pages: BTreeSet<String> = fs::read(project.join("state/outline.yaml"))
        .ok()
        .and_then(|bytes| serde_yaml::from_slice::<Outline>(&bytes).ok())
        .map(|outline| outline.pages.into_iter().map(|page| page.id).collect())
        .unwrap_or_default();
    let mut stack: Vec<_> = selected
        .iter()
        .map(|stage| {
            project
                .join("state/items")
                .join(crate::status::stage_name(*stage))
        })
        .filter(|path| path.exists())
        .collect();
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                stack.push(entry.path());
            } else if entry.path().extension().and_then(|ext| ext.to_str()) == Some("json") {
                let entry_path = entry.path();
                let stage_name = entry_path
                    .strip_prefix(project.join("state/items"))?
                    .components()
                    .next()
                    .unwrap()
                    .as_os_str()
                    .to_string_lossy()
                    .into_owned();
                let Some(stage) = crate::status::parse_stage(&stage_name) else {
                    continue;
                };
                let item = entry_path
                    .strip_prefix(project.join("state/items").join(&stage_name))?
                    .with_extension("")
                    .to_string_lossy()
                    .replace('/', ":");
                let active = match stage {
                    Stage::Ingest => item == "global" || active_sources.contains(&item),
                    Stage::Extract | Stage::Audit => active_chunks.contains(&item),
                    Stage::Write => active_pages.contains(&item),
                    Stage::Survey | Stage::Organize | Stage::Review => item == "global",
                    Stage::Resolve | Stage::Link => true,
                };
                if !active {
                    continue;
                }
                if entry.file_name() == "global.json"
                    && entry.path().to_string_lossy().contains("/ingest/")
                {
                    continue;
                }
                if entry.file_name() == "global.json"
                    && entry.path().to_string_lossy().contains("/resolve/")
                {
                    continue;
                }
                if let Ok(record) = serde_json::from_slice::<ItemRecord>(&fs::read(entry.path())?) {
                    match record.status {
                        ItemStatus::Done => counts.0 += 1,
                        ItemStatus::Pending => counts.1 += 1,
                        ItemStatus::Blocked => counts.2 += 1,
                    }
                }
            }
        }
    }
    Ok(counts)
}

fn load_gaps(project: &Path) -> Vec<String> {
    fs::read(project.join("state/gaps.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}
fn add_gap(project: &Path, gap: &str) -> Result<()> {
    let mut gaps = load_gaps(project);
    gaps.push(gap.into());
    gaps.sort();
    gaps.dedup();
    atomic_write(
        &project.join("state/gaps.json"),
        &serde_json::to_vec_pretty(&gaps)?,
    )
}

fn clear_stage_gaps(project: &Path, stage: Stage) -> Result<()> {
    let prefix = format!("{}/", crate::status::stage_name(stage));
    let gaps: Vec<_> = load_gaps(project)
        .into_iter()
        .filter(|gap| !gap.starts_with(&prefix))
        .collect();
    atomic_write(
        &project.join("state/gaps.json"),
        &serde_json::to_vec_pretty(&gaps)?,
    )
}
