use crate::{
    config::Config,
    prompts,
    scheduler::{self, Job, JobOutcome, ScheduleOptions},
    state::{atomic_write, input_hash, is_blocked, is_done, safe_path, sha256},
    types::{Entity, Fact, ItemRecord, ItemStatus, Outline, PageKind, Stage, Tier},
    validate::validate_markdown,
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub fn content_path(project: &Path, id: &str) -> Result<PathBuf> {
    let relative = page_relative(id)?;
    safe_path(
        project,
        &Path::new("content").join(relative).with_extension("md"),
    )
}

pub(crate) fn page_relative(id: &str) -> Result<PathBuf> {
    ensure!(
        id == "start"
            || (id.matches(':').count() == 1
                && id.bytes().all(|b| b.is_ascii_lowercase()
                    || b.is_ascii_digit()
                    || b == b'_'
                    || b == b':')),
        "invalid page ID {id}"
    );
    Ok(PathBuf::from(id.replace(':', "/")))
}

pub fn draft_path(project: &Path, id: &str) -> Result<PathBuf> {
    let relative = page_relative(id)?;
    safe_path(
        project,
        &Path::new("state/drafts")
            .join(relative)
            .with_extension("md"),
    )
}

pub fn current_write_pages(
    project: &Path,
    outline: &Outline,
) -> Result<Vec<crate::types::PageSpec>> {
    let mut pages = Vec::new();
    for page in &outline.pages {
        let draft = draft_path(project, &page.id)?;
        let relative = draft.strip_prefix(project)?.to_string_lossy().to_string();
        let record_path = record_path(project, &page.id)?;
        let Ok(bytes) = fs::read(record_path) else {
            continue;
        };
        let Ok(record) = serde_json::from_slice::<ItemRecord>(&bytes) else {
            continue;
        };
        if !record.input_hash.is_empty()
            && record.outputs.contains_key(&relative)
            && is_done(project, &record, &record.input_hash)
        {
            pages.push(page.clone());
        }
    }
    Ok(pages)
}

pub fn write_pages(
    project: &Path,
    config: &Config,
    outline: &Outline,
    entities: &[Entity],
    facts: &[Fact],
    options: &ScheduleOptions,
) -> Result<Vec<String>> {
    let (template, version) = prompts::template("write")?;
    let fact_map: BTreeMap<_, _> = facts.iter().map(|fact| (fact.id.as_str(), fact)).collect();
    let catalog: Vec<_> = outline
        .pages
        .iter()
        .map(|page| (&page.id, &page.title))
        .collect();
    let mut jobs = Vec::new();
    let mut hashes = BTreeMap::new();
    let mut current = Vec::new();
    let mut published = published_hashes(project)?;
    for page in &outline.pages {
        let assigned: Vec<_> = page
            .fact_ids
            .iter()
            .filter_map(|id| fact_map.get(id.as_str()).copied())
            .collect();
        let related: Vec<_> = entities
            .iter()
            .filter(|entity| page.entity_ids.contains(&entity.id))
            .collect();
        let input = serde_json::to_vec(&(
            page,
            &assigned,
            &catalog,
            config.agent,
            &config.models,
            &config.profile,
        ))?;
        let hash = input_hash("write", version, &input, options.ceiling);
        let legacy_input = serde_json::to_vec(&(
            page,
            &assigned,
            &catalog,
            &outline.gaps,
            config.agent,
            &config.models,
            &config.profile,
        ))?;
        let legacy_hash = input_hash("write", version, &legacy_input, options.ceiling);
        hashes.insert(page.id.clone(), hash.clone());
        let record_path = record_path(project, &page.id)?;
        if let Ok(bytes) = fs::read(&record_path)
            && let Ok(mut record) = serde_json::from_slice::<ItemRecord>(&bytes)
        {
            if is_done(project, &record, &hash) || is_done(project, &record, &legacy_hash) {
                if record.input_hash == legacy_hash {
                    record.input_hash = hash.clone();
                    atomic_write(&record_path, &serde_json::to_vec_pretty(&record)?)?;
                }
                let content = content_path(project, &page.id)?;
                if !content.exists() {
                    let draft = fs::read(draft_path(project, &page.id)?)?;
                    fs::create_dir_all(content.parent().unwrap())?;
                    atomic_write(&content, &draft)?;
                    published.insert(page.id.clone(), sha256(&draft));
                }
                current.push(page.id.clone());
                continue;
            }
            if is_blocked(&record, &hash) {
                continue;
            }
        }
        let data = serde_json::json!({"page":page,"facts":assigned,"related_entities":related,
            "page_catalog":catalog});
        jobs.push(Job {
            stage: Stage::Write,
            item: page.id.clone(),
            start: if page.kind == PageKind::Leaf {
                Tier::Small
            } else {
                Tier::Mid
            },
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
            let page = outline
                .pages
                .iter()
                .find(|page| page.id == job.item)
                .ok_or_else(|| "unknown page".to_owned())?;
            let errors = validate_markdown(text, page, outline);
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors.join("; "))
            }
        },
        &std::thread::sleep,
    )?;
    for ((_, id), outcome) in report.outcomes {
        let record_path = record_path(project, &id)?;
        let hash = hashes[&id].clone();
        match outcome {
            JobOutcome::Done(text) => {
                let draft = draft_path(project, &id)?;
                let content = content_path(project, &id)?;
                fs::create_dir_all(draft.parent().context("draft parent missing")?)?;
                fs::create_dir_all(content.parent().context("content parent missing")?)?;
                let edited = content_is_edited(project, &id, &published)?;
                atomic_write(&draft, text.as_bytes())?;
                if edited {
                    eprintln!(
                        "warning: write/{id}: protected hand-edited Markdown; new draft saved at {}",
                        draft.display()
                    );
                } else {
                    atomic_write(&content, text.as_bytes())?;
                    published.insert(id.clone(), sha256(text.as_bytes()));
                }
                let relative = draft.strip_prefix(project)?.to_string_lossy().to_string();
                save_record(
                    &record_path,
                    hash,
                    &relative,
                    text.as_bytes(),
                    ItemStatus::Done,
                    vec![],
                )?;
                current.push(id);
            }
            JobOutcome::Blocked(reason) => save_record(
                &record_path,
                hash,
                "",
                &[],
                ItemStatus::Blocked,
                vec![reason],
            )?,
            JobOutcome::Pending => {}
        }
    }
    current.sort();
    save_published_hashes(project, &published)?;
    Ok(current)
}

pub(crate) fn published_hashes(project: &Path) -> Result<BTreeMap<String, String>> {
    let path = safe_path(project, Path::new("state/content_published.json"))?;
    if path.exists() {
        return Ok(serde_json::from_slice(&fs::read(path)?)?);
    }
    let legacy = safe_path(project, Path::new("state/link.json"))?;
    if legacy.exists() {
        return Ok(serde_json::from_slice(&fs::read(legacy)?)?);
    }
    Ok(BTreeMap::new())
}

pub(crate) fn save_published_hashes(
    project: &Path,
    hashes: &BTreeMap<String, String>,
) -> Result<()> {
    let path = safe_path(project, Path::new("state/content_published.json"))?;
    atomic_write(&path, &serde_json::to_vec_pretty(hashes)?)
}

pub(crate) fn content_is_edited(
    project: &Path,
    id: &str,
    published: &BTreeMap<String, String>,
) -> Result<bool> {
    let path = content_path(project, id)?;
    let actual = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let expected = published.get(id).cloned().or_else(|| {
        fs::read(record_path(project, id).ok()?)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<ItemRecord>(&bytes).ok())
            .and_then(|record| record.output_hash)
    });
    Ok(expected.as_deref() != Some(sha256(&actual).as_str()))
}

fn record_path(project: &Path, id: &str) -> Result<PathBuf> {
    let relative = page_relative(id)?;
    Ok(project
        .join("state/items/write")
        .join(relative)
        .with_extension("json"))
}

fn save_record(
    path: &Path,
    hash: String,
    relative: &str,
    output: &[u8],
    status: ItemStatus,
    errors: Vec<String>,
) -> Result<()> {
    fs::create_dir_all(path.parent().context("record parent missing")?)?;
    let digest = sha256(output);
    let outputs = if relative.is_empty() {
        BTreeMap::new()
    } else {
        BTreeMap::from([(relative.into(), digest.clone())])
    };
    let record = ItemRecord {
        input_hash: hash,
        output_hash: if relative.is_empty() {
            None
        } else {
            Some(digest)
        },
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
    atomic_write(path, &serde_json::to_vec_pretty(&record)?)
}
