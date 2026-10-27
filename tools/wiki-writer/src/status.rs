use crate::{
    config,
    state::is_done,
    types::{Chunk, ItemRecord, ItemStatus, Outline, Page, Stage},
};
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

pub fn print(project: &Path) -> Result<()> {
    let project_config = config::load(project)?;
    let ingest_current = crate::ingest::verify_current(project, &project_config).is_ok();
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
    let mut counts = BTreeMap::<Stage, (usize, usize, usize, usize)>::new();
    let root = project.join("state/items");
    if root.exists() {
        let mut stack = vec![root];
        while let Some(directory) = stack.pop() {
            for entry in fs::read_dir(directory)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    stack.push(entry.path());
                    continue;
                }
                if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
                    continue;
                }
                let stage_name = entry
                    .path()
                    .strip_prefix(project.join("state/items"))?
                    .components()
                    .next()
                    .unwrap()
                    .as_os_str()
                    .to_string_lossy()
                    .to_string();
                let Some(stage) = parse_stage(&stage_name) else {
                    continue;
                };
                if matches!(stage, Stage::Ingest | Stage::Resolve)
                    && entry.file_name() == "global.json"
                {
                    continue;
                }
                let Ok(record) = serde_json::from_slice::<ItemRecord>(&fs::read(entry.path())?)
                else {
                    continue;
                };
                let row = counts.entry(stage).or_default();
                match record.status {
                    ItemStatus::Done if is_done(project, &record, &record.input_hash) => row.0 += 1,
                    ItemStatus::Done => row.1 += 1,
                    ItemStatus::Pending => row.1 += 1,
                    ItemStatus::Blocked => row.2 += 1,
                }
                if record.escalated {
                    row.3 += 1;
                }
            }
        }
    }
    println!("stage done pending blocked escalation tokens dollars");
    let ledger: Vec<crate::scheduler::LedgerEntry> = fs::read(project.join("state/ledger.jsonl"))
        .ok()
        .map(|bytes| {
            bytes
                .split(|byte| *byte == b'\n')
                .filter(|line| !line.is_empty())
                .filter_map(|line| serde_json::from_slice(line).ok())
                .collect()
        })
        .unwrap_or_default();
    for stage in stages {
        let (mut done, mut pending, mut blocked, mut escalated) =
            counts.get(&stage).copied().unwrap_or_default();
        if !ingest_current {
            done = 0;
            pending = if stage == Stage::Ingest {
                project_config.sources.len()
            } else {
                0
            };
            blocked = 0;
            escalated = 0;
        }
        if ingest_current && stage == Stage::Link && link_is_current(project)? {
            done = 1;
        }
        let expectation = if !ingest_current && stage != Stage::Ingest {
            waiting(Stage::Ingest)
        } else {
            expected_work(project, stage)?
        };
        if stage == Stage::Resolve && expectation.waiting_for.is_none() {
            if resolve_global_current(project) {
                done += 1;
            } else {
                pending += 1;
            }
        }
        if let Some(expected) = expectation.count {
            pending += expected.saturating_sub(done + pending + blocked);
        }
        let entries: Vec<_> = ledger.iter().filter(|entry| entry.stage == stage).collect();
        let tokens: u64 = entries
            .iter()
            .map(|entry| entry.input_tokens + entry.output_tokens)
            .sum();
        let cost = if entries.is_empty() || entries.iter().any(|entry| entry.cost_usd.is_none()) {
            "unknown".into()
        } else {
            format!(
                "{:.4}",
                entries
                    .iter()
                    .map(|entry| entry.cost_usd.unwrap_or(0.0))
                    .sum::<f64>()
            )
        };
        if let Some(prerequisite) = expectation.waiting_for {
            println!(
                "{} {done} unknown {blocked} {escalated} {tokens} {cost} (waiting for {})",
                stage_name(stage),
                stage_name(prerequisite)
            );
        } else {
            println!(
                "{} {done} {pending} {blocked} {escalated} {tokens} {cost}",
                stage_name(stage)
            );
        }
    }
    if let Ok(bytes) = fs::read(project.join("state/gaps.json"))
        && let Ok(gaps) = serde_json::from_slice::<Vec<String>>(&bytes)
    {
        for gap in gaps {
            println!("gap: {gap}");
        }
    }
    if let Ok(bytes) = fs::read(project.join("state/ingest/pages.json"))
        && let Ok(pages) = serde_json::from_slice::<Vec<Page>>(&bytes)
    {
        for page in pages.into_iter().filter(|page| page.scanned) {
            println!(
                "scanned: {} page {}",
                page.reference.source, page.reference.page
            );
        }
    }
    if let Ok(bytes) = fs::read(project.join("state/redo_queue.json"))
        && let Ok(queue) = serde_json::from_slice::<Vec<crate::review::Finding>>(&bytes)
    {
        for finding in queue {
            println!(
                "redo: {} {}: {}",
                stage_name(finding.stage),
                finding.item,
                finding.reason
            );
        }
    }
    Ok(())
}

fn resolve_global_current(project: &Path) -> bool {
    fs::read(project.join("state/items/resolve/global.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<ItemRecord>(&bytes).ok())
        .is_some_and(|record| is_done(project, &record, &record.input_hash))
}

fn link_is_current(project: &Path) -> Result<bool> {
    let Ok(bytes) = fs::read(project.join("state/link.json")) else {
        return Ok(false);
    };
    let Ok(fingerprints) = serde_json::from_slice::<BTreeMap<String, String>>(&bytes) else {
        return Ok(false);
    };
    let Ok(bytes) = fs::read(project.join("state/lint.json")) else {
        return Ok(false);
    };
    if serde_json::from_slice::<crate::link::LintReport>(&bytes).is_err() {
        return Ok(false);
    }
    let Ok(bytes) = fs::read(project.join("state/outline.yaml")) else {
        return Ok(false);
    };
    let outline: Outline = serde_yaml::from_slice(&bytes)?;
    let pages = crate::write::current_write_pages(project, &outline)?;
    if fingerprints.len() != pages.len() {
        return Ok(false);
    }
    for page in pages {
        let path = crate::write::content_path(project, &page.id)?;
        let Ok(content) = fs::read(path) else {
            return Ok(false);
        };
        let digest: String = Sha256::digest(&content)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if fingerprints.get(&page.id) != Some(&digest) {
            return Ok(false);
        }
    }
    Ok(true)
}

struct ExpectedWork {
    count: Option<usize>,
    waiting_for: Option<Stage>,
}

fn expected_work(project: &Path, stage: Stage) -> Result<ExpectedWork> {
    let chunks_path = project.join("state/ingest/chunks.json");
    let schema_path = project.join("state/schema.yaml");
    let entities_path = project.join("state/entities.json");
    let outline_path = project.join("state/outline.yaml");
    let chunks = || -> Result<Vec<Chunk>> { Ok(serde_json::from_slice(&fs::read(&chunks_path)?)?) };
    let result = match stage {
        Stage::Ingest => ExpectedWork {
            count: Some(config::load(project)?.sources.len()),
            waiting_for: None,
        },
        Stage::Survey if !chunks_path.exists() => waiting(Stage::Ingest),
        Stage::Survey => known(1),
        Stage::Extract if !chunks_path.exists() => waiting(Stage::Ingest),
        Stage::Extract if !schema_path.exists() => waiting(Stage::Survey),
        Stage::Extract => known(chunks()?.len()),
        Stage::Audit if !schema_path.exists() => waiting(Stage::Survey),
        Stage::Audit => {
            match crate::pipeline::expected_audit_count(project, &config::load(project)?) {
                Ok(count) => known(count),
                Err(_) => waiting(Stage::Extract),
            }
        }
        Stage::Resolve if !project.join("state/effective").exists() => waiting(Stage::Audit),
        Stage::Resolve => known(0),
        Stage::Organize if !entities_path.exists() => waiting(Stage::Resolve),
        Stage::Organize => known(1),
        Stage::Write if !outline_path.exists() => waiting(Stage::Organize),
        Stage::Write => {
            let outline: Outline = serde_yaml::from_slice(&fs::read(&outline_path)?)?;
            known(outline.pages.len())
        }
        Stage::Link if !outline_path.exists() => waiting(Stage::Organize),
        Stage::Link => {
            let outline: Outline = serde_yaml::from_slice(&fs::read(&outline_path)?)?;
            if crate::write::current_write_pages(project, &outline)?.is_empty() {
                waiting(Stage::Write)
            } else {
                known(1)
            }
        }
        Stage::Review if !link_is_current(project)? => waiting(Stage::Link),
        Stage::Review => known(1),
    };
    Ok(result)
}

fn known(count: usize) -> ExpectedWork {
    ExpectedWork {
        count: Some(count),
        waiting_for: None,
    }
}

fn waiting(stage: Stage) -> ExpectedWork {
    ExpectedWork {
        count: None,
        waiting_for: Some(stage),
    }
}

pub fn stage_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Ingest => "ingest",
        Stage::Survey => "survey",
        Stage::Extract => "extract",
        Stage::Audit => "audit",
        Stage::Resolve => "resolve",
        Stage::Organize => "organize",
        Stage::Write => "write",
        Stage::Link => "link",
        Stage::Review => "review",
    }
}

pub fn parse_stage(name: &str) -> Option<Stage> {
    match name {
        "ingest" => Some(Stage::Ingest),
        "survey" => Some(Stage::Survey),
        "extract" => Some(Stage::Extract),
        "audit" => Some(Stage::Audit),
        "resolve" => Some(Stage::Resolve),
        "organize" => Some(Stage::Organize),
        "write" => Some(Stage::Write),
        "link" => Some(Stage::Link),
        "review" => Some(Stage::Review),
        _ => None,
    }
}
