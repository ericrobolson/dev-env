use crate::{
    state::atomic_write,
    types::{Entity, Outline, PageKind, PageSpec},
    validate::validate_markdown,
    write::{
        content_is_edited, content_path, current_write_pages, published_hashes,
        save_published_hashes,
    },
};
use anyhow::Result;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    sync::LazyLock,
};

static CITATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\^f[^\]]*\]").unwrap());
static DOKULINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[([^\]|#]+)(?:#[^\]|]+)?(?:\|[^\]]*)?\]\]").unwrap());

pub fn link_markdown(
    text: &str,
    page: &PageSpec,
    outline: &Outline,
    entities: &[Entity],
) -> String {
    let mut entity_pages = BTreeMap::<&str, Vec<&str>>::new();
    for spec in &outline.pages {
        if spec.kind == PageKind::Leaf {
            for id in &spec.entity_ids {
                entity_pages.entry(id).or_default().push(&spec.id);
            }
        }
    }
    let mut aliases = BTreeMap::<String, (BTreeSet<String>, BTreeSet<(String, String)>)>::new();
    for entity in entities {
        let Some(destinations) = entity_pages.get(entity.id.as_str()) else {
            continue;
        };
        if destinations.len() != 1 || destinations[0] == page.id {
            continue;
        }
        for alias in std::iter::once(&entity.name).chain(entity.aliases.iter()) {
            if !alias.trim().is_empty() {
                let entry = aliases.entry(alias.to_lowercase()).or_default();
                entry.0.insert(destinations[0].into());
                entry.1.insert((alias.clone(), entity.entity_type.clone()));
            }
        }
    }
    let aliases: Vec<_> = aliases
        .into_iter()
        .filter_map(|(alias, (destinations, spellings))| {
            if destinations.len() == 1 {
                Some((alias, destinations.into_iter().next().unwrap(), spellings))
            } else {
                None
            }
        })
        .collect();
    let patterns: Vec<_> = aliases
        .iter()
        .flat_map(|(alias, dest, spellings)| {
            spellings.iter().filter_map(move |(spelling, entity_type)| {
                let exact_case = matches!(
                    entity_type.as_str(),
                    "rule" | "action" | "condition" | "phase"
                );
                RegexBuilder::new(&regex::escape(spelling))
                    .case_insensitive(!exact_case)
                    .build()
                    .ok()
                    .map(|re| (alias, dest, re))
            })
        })
        .collect();
    let mut protected = 0usize;
    let mut replacements = Vec::<(usize, usize, String)>::new();
    for (event, range) in Parser::new(text).into_offset_iter() {
        match event {
            Event::Start(Tag::Link { .. } | Tag::Image { .. }) => protected += 1,
            Event::End(TagEnd::Link | TagEnd::Image) => protected = protected.saturating_sub(1),
            Event::Text(_) if protected == 0 => {
                let segment = &text[range.clone()];
                let citation_spans: Vec<_> = CITATION
                    .find_iter(segment)
                    .map(|m| (m.start(), m.end()))
                    .collect();
                let mut matches = Vec::new();
                for (alias, dest, pattern) in &patterns {
                    for found in pattern.find_iter(segment) {
                        if citation_spans
                            .iter()
                            .any(|(start, end)| found.start() < *end && found.end() > *start)
                        {
                            continue;
                        }
                        if !boundary(segment, found.start(), found.end()) {
                            continue;
                        }
                        if !alias.chars().any(char::is_whitespace)
                            && followed_by_capitalized_word(segment, found.end())
                        {
                            continue;
                        }
                        matches.push((
                            found.start(),
                            found.end(),
                            alias.len(),
                            alias.as_str(),
                            dest.as_str(),
                        ));
                    }
                }
                matches.sort_by(|a, b| a.0.cmp(&b.0).then(b.2.cmp(&a.2)).then(a.3.cmp(b.3)));
                let mut end = 0;
                for (start, stop, _, _, dest) in matches {
                    if start < end {
                        continue;
                    }
                    let label = &segment[start..stop];
                    replacements.push((
                        range.start + start,
                        range.start + stop,
                        format!("[{label}]({dest})"),
                    ));
                    end = stop;
                }
            }
            _ => {}
        }
    }
    let mut output = text.to_owned();
    for (start, end, replacement) in replacements.into_iter().rev() {
        output.replace_range(start..end, &replacement);
    }
    output
}

fn followed_by_capitalized_word(text: &str, end: usize) -> bool {
    let after = &text[end..];
    after.chars().next().is_some_and(char::is_whitespace)
        && after
            .trim_start()
            .chars()
            .next()
            .is_some_and(char::is_uppercase)
}

fn boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LintReport {
    pub broken_links: Vec<String>,
    pub orphans: Vec<String>,
    pub missing_citations: Vec<String>,
    pub gaps: Vec<String>,
}

pub fn link(project: &Path, outline: &Outline, entities: &[Entity]) -> Result<LintReport> {
    let mut fingerprints = BTreeMap::new();
    let mut published = published_hashes(project)?;
    for page in current_write_pages(project, outline)? {
        let path = content_path(project, &page.id)?;
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let edited = content_is_edited(project, &page.id, &published)?;
        let linked = if edited {
            eprintln!("warning: link/{}: protected hand-edited Markdown", page.id);
            text.clone()
        } else {
            link_markdown(&text, &page, outline, entities)
        };
        if linked != text {
            atomic_write(&path, linked.as_bytes())?;
        }
        let hash: String = Sha256::digest(linked.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        fingerprints.insert(page.id.clone(), hash);
        if !edited {
            published.insert(page.id.clone(), fingerprints[&page.id].clone());
        }
    }
    save_published_hashes(project, &published)?;
    atomic_write(
        &project.join("state/link.json"),
        &serde_json::to_vec_pretty(&fingerprints)?,
    )?;
    lint(project)
}

pub fn lint(project: &Path) -> Result<LintReport> {
    let outline: Outline = serde_yaml::from_slice(&fs::read(project.join("state/outline.yaml"))?)?;
    let valid: BTreeSet<_> = outline.pages.iter().map(|page| page.id.as_str()).collect();
    let mut report = LintReport {
        broken_links: vec![],
        orphans: vec![],
        missing_citations: vec![],
        gaps: outline.gaps.clone(),
    };
    if let Ok(bytes) = fs::read(project.join("state/gaps.json"))
        && let Ok(gaps) = serde_json::from_slice::<Vec<String>>(&bytes)
    {
        report.gaps.extend(gaps);
    }
    let mut inbound = BTreeSet::new();
    for page in &outline.pages {
        let path = content_path(project, &page.id)?;
        let Ok(text) = fs::read_to_string(&path) else {
            report
                .gaps
                .push(format!("write/{}: content missing", page.id));
            continue;
        };
        for error in validate_markdown(&text, page, &outline) {
            if error.contains("link") {
                report.broken_links.push(format!("{}: {error}", page.id));
            } else if error.contains("citation") {
                report
                    .missing_citations
                    .push(format!("{}: {error}", page.id));
            }
        }
        for event in Parser::new(&text) {
            if let Event::Start(Tag::Link { dest_url, .. }) = event {
                let target = dest_url.split('#').next().unwrap_or("");
                if valid.contains(target) {
                    inbound.insert(target.to_owned());
                }
            }
        }
        for captures in DOKULINK.captures_iter(&text) {
            if valid.contains(&captures[1]) {
                inbound.insert(captures[1].to_owned());
            }
        }
    }
    for page in &outline.pages {
        if page.id != "start" && !inbound.contains(&page.id) {
            report.orphans.push(page.id.clone());
        }
    }
    report.broken_links.sort();
    report.broken_links.dedup();
    report.orphans.sort();
    report.orphans.dedup();
    report.missing_citations.sort();
    report.missing_citations.dedup();
    report.gaps.sort();
    report.gaps.dedup();
    atomic_write(
        &project.join("state/lint.json"),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
