use crate::{
    config::Config,
    link::LintReport,
    prompts,
    scheduler::{self, Job, JobOutcome, ScheduleOptions},
    state::{atomic_write, input_hash, is_blocked, is_done, sha256},
    types::{Fact, ItemRecord, ItemStatus, Outline, PageSpec, Stage, Tier},
    write::{content_is_edited, content_path, current_write_pages, published_hashes},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub stage: Stage,
    pub item: String,
    pub reason: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub findings: Vec<Finding>,
}

pub fn sample_pages(pages: &[PageSpec]) -> Vec<PageSpec> {
    let mut sorted = pages.to_vec();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));
    let count = sorted.len().min(5);
    if count == 0 {
        return vec![];
    }
    if count == 1 {
        return vec![sorted[0].clone()];
    }
    (0..count)
        .map(|index| sorted[index * (sorted.len() - 1) / (count - 1)].clone())
        .collect()
}

pub fn validate_review(review: &Review, outline: &Outline, chunks: &[String]) -> Vec<String> {
    let pages: BTreeSet<_> = outline.pages.iter().map(|page| page.id.as_str()).collect();
    let chunks: BTreeSet<_> = chunks.iter().map(String::as_str).collect();
    let mut errors = Vec::new();
    for finding in &review.findings {
        if finding.reason.trim().is_empty() {
            errors.push(format!("empty review reason for {}", finding.item));
        }
        match finding.stage {
            Stage::Extract if chunks.contains(finding.item.as_str()) => {}
            Stage::Write if pages.contains(finding.item.as_str()) => {}
            _ => errors.push(format!(
                "invalid review target {:?}/{}",
                finding.stage, finding.item
            )),
        }
    }
    errors
}

pub fn run_review(
    project: &Path,
    config: &Config,
    outline: &Outline,
    facts: &[Fact],
    chunks: &[String],
    options: &ScheduleOptions,
) -> Result<Option<Review>> {
    let (template, version) = prompts::template("review")?;
    let lint: LintReport = serde_json::from_slice(&fs::read(project.join("state/lint.json"))?)?;
    let available = current_write_pages(project, outline)?;
    if available.is_empty() {
        return Ok(None);
    }
    let selected = sample_pages(&available);
    let fact_map: BTreeMap<_, _> = facts.iter().map(|fact| (fact.id.as_str(), fact)).collect();
    let mut evidence = Vec::new();
    for page in selected {
        let text = fs::read_to_string(content_path(project, &page.id)?)
            .with_context(|| format!("review page {} missing", page.id))?;
        let assigned: Vec<_> = page
            .fact_ids
            .iter()
            .filter_map(|id| fact_map.get(id.as_str()).copied())
            .collect();
        evidence.push(serde_json::json!({"page":page,"markdown":text,"facts":assigned}));
    }
    let input = serde_json::to_vec(&(
        &evidence,
        &lint,
        config.agent,
        &config.models,
        &config.profile,
    ))?;
    let hash = input_hash("review", version, &input, options.ceiling);
    let record_path = project.join("state/items/review/global.json");
    if let Ok(bytes) = fs::read(&record_path)
        && let Ok(record) = serde_json::from_slice::<ItemRecord>(&bytes)
    {
        if is_done(project, &record, &hash) {
            let review: Review =
                serde_json::from_slice(&fs::read(project.join("state/review.json"))?)?;
            enqueue_once(project, &hash, &review)?;
            return Ok(Some(review));
        }
        if is_blocked(&record, &hash) {
            return Ok(None);
        }
    }
    let data = serde_json::json!({"pages":evidence,"lint":lint,"known_chunks":chunks});
    let job = Job {
        stage: Stage::Review,
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
            let review: Review = serde_json::from_str(text).map_err(|e| e.to_string())?;
            let errors = validate_review(&review, outline, chunks);
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors.join("; "))
            }
        },
        &std::thread::sleep,
    )?;
    match report.outcomes.get(&(Stage::Review, "global".into())) {
        Some(JobOutcome::Done(text)) => {
            let review: Review = serde_json::from_str(text)?;
            let bytes = serde_json::to_vec_pretty(&review)?;
            atomic_write(&project.join("state/review.json"), &bytes)?;
            fs::create_dir_all(
                record_path
                    .parent()
                    .context("review record parent missing")?,
            )?;
            let digest = sha256(&bytes);
            let record = ItemRecord {
                input_hash: hash.clone(),
                output_hash: Some(digest.clone()),
                output_path: Some("state/review.json".into()),
                outputs: BTreeMap::from([("state/review.json".into(), digest)]),
                status: ItemStatus::Done,
                errors: vec![],
                escalated: false,
            };
            atomic_write(&record_path, &serde_json::to_vec_pretty(&record)?)?;
            enqueue_once(project, &hash, &review)?;
            Ok(Some(review))
        }
        Some(JobOutcome::Blocked(reason)) => {
            fs::create_dir_all(
                record_path
                    .parent()
                    .context("review record parent missing")?,
            )?;
            let record = ItemRecord {
                input_hash: hash,
                output_hash: None,
                output_path: None,
                outputs: BTreeMap::new(),
                status: ItemStatus::Blocked,
                errors: vec![reason.clone()],
                escalated: false,
            };
            atomic_write(&record_path, &serde_json::to_vec_pretty(&record)?)?;
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn enqueue_once(project: &Path, hash: &str, review: &Review) -> Result<()> {
    let marker_path = project.join("state/review_enqueued.json");
    let mut markers: BTreeSet<String> = fs::read(&marker_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    if markers.contains(hash) {
        return Ok(());
    }
    let queue_path = project.join("state/redo_queue.json");
    let mut queue: Vec<Finding> = fs::read(&queue_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let published = published_hashes(project)?;
    for finding in &review.findings {
        if finding.stage == Stage::Write && content_is_edited(project, &finding.item, &published)? {
            eprintln!(
                "warning: review/write/{}: page has human edits; skipping automatic redo",
                finding.item
            );
            continue;
        }
        if !queue.contains(finding) {
            queue.push(finding.clone());
        }
    }
    atomic_write(&queue_path, &serde_json::to_vec_pretty(&queue)?)?;
    markers.insert(hash.into());
    atomic_write(&marker_path, &serde_json::to_vec_pretty(&markers)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{state::sha256, write::save_published_hashes};

    #[test]
    fn review_does_not_queue_redo_of_hand_edited_page() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path();
        fs::create_dir_all(project.join("content")).unwrap();
        fs::create_dir_all(project.join("state")).unwrap();
        fs::write(project.join("content/start.md"), "Human edit\n").unwrap();
        save_published_hashes(
            project,
            &BTreeMap::from([("start".into(), sha256(b"Generated\n"))]),
        )
        .unwrap();
        let review = Review {
            findings: vec![Finding {
                stage: Stage::Write,
                item: "start".into(),
                reason: "rewrite".into(),
            }],
        };
        enqueue_once(project, "review-1", &review).unwrap();
        let queue: Vec<Finding> =
            serde_json::from_slice(&fs::read(project.join("state/redo_queue.json")).unwrap())
                .unwrap();
        assert!(queue.is_empty());
    }
}
