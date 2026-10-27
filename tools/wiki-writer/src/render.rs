use crate::{
    config,
    state::{atomic_write, safe_path, sha256},
    types::{Fact, Outline, PageRef, Source},
    write::{content_path, current_write_pages},
};
use anyhow::{Context, Result, ensure};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RenderReport {
    pub written: usize,
    pub unchanged: usize,
    pub protected: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RenderEntry {
    source_hash: String,
    rendered_hash: String,
}

pub fn render_markdown(
    markdown: &str,
    facts: &BTreeMap<String, Fact>,
    sources: &[Source],
) -> Result<String> {
    let (prepared, tokens) = tokenize_citations(markdown, facts)?;
    let mut output = String::new();
    let mut list_stack = Vec::<bool>::new();
    let mut in_table_head = false;
    let mut in_code = false;
    let mut quote_depth = 0usize;
    let mut heading = None;
    let mut seen_refs = BTreeSet::<Vec<PageRef>>::new();
    for event in Parser::new_ext(&prepared, Options::ENABLE_TABLES) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let count = match level {
                    pulldown_cmark::HeadingLevel::H1 => 6,
                    pulldown_cmark::HeadingLevel::H2 => 5,
                    pulldown_cmark::HeadingLevel::H3 => 4,
                    pulldown_cmark::HeadingLevel::H4 => 3,
                    _ => 2,
                };
                heading = Some(count);
                output.push_str(&"=".repeat(count));
                output.push(' ');
            }
            Event::End(TagEnd::Heading(_)) => {
                let count = heading.take().unwrap_or(2);
                output.push(' ');
                output.push_str(&"=".repeat(count));
                output.push_str("\n\n");
            }
            Event::Start(Tag::Paragraph) => {
                if quote_depth > 0 && output.ends_with('\n') {
                    output.push_str(&"> ".repeat(quote_depth));
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if list_stack.is_empty() {
                    output.push_str("\n\n");
                }
            }
            Event::Start(Tag::Strong) => output.push_str("**"),
            Event::End(TagEnd::Strong) => output.push_str("**"),
            Event::Start(Tag::Emphasis) => output.push_str("//"),
            Event::End(TagEnd::Emphasis) => output.push_str("//"),
            Event::Start(Tag::CodeBlock(_)) => {
                in_code = true;
                output.push_str("<code>\n");
            }
            Event::End(TagEnd::CodeBlock) => {
                if !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str("</code>\n\n");
                in_code = false;
            }
            Event::Start(Tag::List(start)) => list_stack.push(start.is_some()),
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
                if list_stack.is_empty() {
                    output.push('\n');
                }
            }
            Event::Start(Tag::Item) => {
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str(&" ".repeat(list_stack.len() * 2));
                output.push(if *list_stack.last().unwrap_or(&false) {
                    '-'
                } else {
                    '*'
                });
                output.push(' ');
            }
            Event::End(TagEnd::Item) => {
                if !output.ends_with('\n') {
                    output.push('\n');
                }
            }
            Event::Start(Tag::TableHead) => {
                in_table_head = true;
                output.push_str("^ ");
            }
            Event::End(TagEnd::TableHead) => {
                if output.ends_with(' ') {
                    output.pop();
                }
                output.push('\n');
                in_table_head = false;
            }
            Event::Start(Tag::TableRow) => {
                output.push_str("| ");
            }
            Event::End(TagEnd::TableRow) => {
                if output.ends_with(' ') {
                    output.pop();
                }
                output.push('\n');
            }
            Event::End(TagEnd::TableCell) => {
                output.push_str(if in_table_head { " ^ " } else { " | " })
            }
            Event::End(TagEnd::Table) => output.push('\n'),
            Event::Start(Tag::Link { dest_url, .. }) => {
                output.push_str("[[");
                output.push_str(&escape_link_target(&dest_url));
                output.push('|');
            }
            Event::End(TagEnd::Link) => {
                output.push_str("]] ");
                output.pop();
            }
            Event::Start(Tag::Image { .. }) => {}
            Event::End(TagEnd::Image) => {}
            Event::Start(Tag::BlockQuote(_)) => {
                quote_depth += 1;
                output.push_str(&"> ".repeat(quote_depth));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                quote_depth = quote_depth.saturating_sub(1);
                output.push_str("\n\n");
            }
            Event::Text(text) => {
                if in_code {
                    output.push_str(
                        &text.replace("</code>", "</code><nowiki>&lt;/code&gt;</nowiki><code>"),
                    );
                } else {
                    emit_text(&mut output, &text, &tokens, facts, sources, &mut seen_refs)?;
                }
            }
            Event::Code(text) => {
                output.push_str("''");
                output.push_str(&literal(&text));
                output.push_str("''");
            }
            Event::SoftBreak => output.push('\n'),
            Event::HardBreak => output.push_str("\\\\\n"),
            Event::Rule => output.push_str("----\n\n"),
            Event::Html(text) | Event::InlineHtml(text) => output.push_str(&literal(&text)),
            _ => {}
        }
    }
    Ok(output.trim_end().to_owned() + "\n")
}

fn escape_link_target(target: &str) -> String {
    let mut escaped = String::new();
    for character in target.chars() {
        match character {
            '|' => escaped.push_str("%7C"),
            '[' => escaped.push_str("%5B"),
            ']' => escaped.push_str("%5D"),
            '<' => escaped.push_str("%3C"),
            '>' => escaped.push_str("%3E"),
            '\\' => escaped.push_str("%5C"),
            '\n' => escaped.push_str("%0A"),
            '\r' => escaped.push_str("%0D"),
            '\t' => escaped.push_str("%09"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn tokenize_citations(
    markdown: &str,
    facts: &BTreeMap<String, Fact>,
) -> Result<(String, BTreeMap<String, String>)> {
    let mut code_spans = Vec::new();
    let mut block_start = None;
    for (event, range) in Parser::new(markdown).into_offset_iter() {
        match event {
            Event::Code(_) => code_spans.push((range.start, range.end)),
            Event::Start(Tag::CodeBlock(_)) => block_start = Some(range.start),
            Event::End(TagEnd::CodeBlock) => {
                if let Some(start) = block_start.take() {
                    code_spans.push((start, range.end));
                }
            }
            _ => {}
        }
    }
    let citation = Regex::new(r"\[\^f[0-9]{4,}\]")?;
    let looking = Regex::new(r"\[\^f[^\]]*\]")?;
    let mut prefix_number = 0u64;
    while markdown.contains(&format!("WIKICITE{prefix_number}")) {
        prefix_number += 1;
    }
    let prefix = format!("WIKICITE{prefix_number}");
    let mut replacements = Vec::new();
    let mut tokens = BTreeMap::new();
    for found in looking.find_iter(markdown) {
        if code_spans
            .iter()
            .any(|(start, end)| found.start() >= *start && found.end() <= *end)
        {
            continue;
        }
        let marker = found.as_str();
        ensure!(
            citation
                .find(marker)
                .is_some_and(|match_| match_.as_str() == marker),
            "malformed citation {marker}"
        );
        let id = &marker[2..marker.len() - 1];
        ensure!(facts.contains_key(id), "unknown citation {id}");
        let token = format!("{prefix}{}END", tokens.len());
        tokens.insert(token.clone(), id.into());
        replacements.push((found.start(), found.end(), token));
    }
    let mut prepared = markdown.to_owned();
    for (start, end, token) in replacements.into_iter().rev() {
        prepared.replace_range(start..end, &token);
    }
    Ok((prepared, tokens))
}

fn emit_text(
    output: &mut String,
    text: &str,
    tokens: &BTreeMap<String, String>,
    facts: &BTreeMap<String, Fact>,
    sources: &[Source],
    seen: &mut BTreeSet<Vec<PageRef>>,
) -> Result<()> {
    if tokens.is_empty() {
        output.push_str(&literal(text));
        return Ok(());
    }
    let mut offset = 0;
    while offset < text.len() {
        let next = tokens
            .keys()
            .filter_map(|token| {
                text[offset..]
                    .find(token)
                    .map(|index| (offset + index, token))
            })
            .min_by_key(|(index, _)| *index);
        let Some((start, token)) = next else {
            output.push_str(&literal(&text[offset..]));
            break;
        };
        output.push_str(&literal(&text[offset..start]));
        let id = &tokens[token];
        let fact = &facts[id];
        let mut pages = fact.draft.pages.clone();
        pages.sort();
        pages.dedup();
        if seen.insert(pages.clone()) {
            output.push_str(&citation_text(&pages, sources)?);
        }
        offset = start + token.len();
    }
    Ok(())
}

fn literal(text: &str) -> String {
    const SPECIAL: &str = "*/_'[]{}|^<>=%";
    let mut out = String::new();
    let mut index = 0;
    while index < text.len() {
        if text[index..].starts_with("</nowiki>") {
            for ch in "</nowiki>".chars() {
                out.push_str(&format!("<nowiki>{ch}</nowiki>"));
            }
            index += "</nowiki>".len();
            continue;
        }
        let ch = text[index..].chars().next().unwrap();
        if !SPECIAL.contains(ch) {
            out.push(ch);
            index += ch.len_utf8();
            continue;
        }
        if ch == '|' || ch == '^' {
            out.push_str(&format!("<nowiki>{ch}</nowiki>"));
            index += ch.len_utf8();
            continue;
        }
        let mut run = String::new();
        while index < text.len() && !text[index..].starts_with("</nowiki>") {
            let next = text[index..].chars().next().unwrap();
            if !SPECIAL.contains(next) || next == '|' || next == '^' {
                break;
            }
            run.push(next);
            index += next.len_utf8();
        }
        out.push_str("<nowiki>");
        out.push_str(&run);
        out.push_str("</nowiki>");
    }
    out
}

fn citation_text(pages: &[PageRef], sources: &[Source]) -> Result<String> {
    let source_order: BTreeMap<_, _> = sources
        .iter()
        .enumerate()
        .map(|(index, source)| (source.id.as_str(), (index, source.name.as_str())))
        .collect();
    let mut grouped = BTreeMap::<usize, (String, Vec<u32>)>::new();
    for page in pages {
        let (index, name) = source_order
            .get(page.source.as_str())
            .context("citation source missing")?;
        grouped
            .entry(*index)
            .or_insert_with(|| ((*name).into(), vec![]))
            .1
            .push(page.page);
    }
    let mut parts = Vec::new();
    for (_, (name, mut numbers)) in grouped {
        numbers.sort();
        numbers.dedup();
        let mut ranges = Vec::new();
        let mut index = 0;
        while index < numbers.len() {
            let start = numbers[index];
            let mut end = start;
            while index + 1 < numbers.len() && numbers[index + 1] == end + 1 {
                index += 1;
                end = numbers[index];
            }
            ranges.push(if start == end {
                format!("{start}")
            } else {
                format!("{start}–{end}")
            });
            index += 1;
        }
        let label = if numbers.len() == 1 { "p." } else { "pp." };
        let source_label = if sources.len() > 1 {
            format!("{name}, ")
        } else {
            String::new()
        };
        parts.push(format!("{source_label}{label} {}", ranges.join(", ")));
    }
    Ok(format!("(({}))", parts.join("; ")))
}

pub fn render(project: &Path) -> Result<RenderReport> {
    let config = config::load(project)?;
    let outline: Outline = serde_yaml::from_slice(&fs::read(project.join("state/outline.yaml"))?)?;
    let mut facts = BTreeMap::new();
    if let Ok(entries) = fs::read_dir(project.join("state/effective")) {
        for entry in entries {
            let entry = entry?;
            if entry.path().extension().and_then(|ext| ext.to_str()) == Some("json") {
                let notes: crate::types::Notes = serde_json::from_slice(&fs::read(entry.path())?)?;
                for fact in notes.facts {
                    facts.insert(fact.id.clone(), fact);
                }
            }
        }
    }
    let manifest_path = project.join("state/render.json");
    let mut manifest: BTreeMap<String, RenderEntry> = fs::read(&manifest_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let mut report = RenderReport {
        written: 0,
        unchanged: 0,
        protected: vec![],
    };
    let wanted: BTreeSet<_> = outline.pages.iter().map(|page| page.id.clone()).collect();
    for page in current_write_pages(project, &outline)? {
        let content_path = content_path(project, &page.id)?;
        let Ok(markdown) = fs::read_to_string(content_path) else {
            continue;
        };
        let rendered = render_markdown(&markdown, &facts, &config.sources)?;
        let source_hash = hex(markdown.as_bytes());
        let rendered_hash = hex(rendered.as_bytes());
        let destination = wiki_path(project, &page.id)?;
        let actual = fs::read(&destination).ok();
        let previous = manifest.get(&page.id);
        if let Some(bytes) = &actual {
            if previous.is_none() || previous.is_some_and(|old| old.rendered_hash != hex(bytes)) {
                eprintln!("warning: render/{}: protected wiki edit", page.id);
                report.protected.push(page.id.clone());
                continue;
            }
            if hex(bytes) == rendered_hash {
                report.unchanged += 1;
                continue;
            }
        }
        fs::create_dir_all(destination.parent().context("wiki page parent missing")?)?;
        atomic_write(&destination, rendered.as_bytes())?;
        manifest.insert(
            page.id.clone(),
            RenderEntry {
                source_hash,
                rendered_hash,
            },
        );
        report.written += 1;
    }
    let stale: Vec<_> = manifest
        .keys()
        .filter(|id| !wanted.contains(*id))
        .cloned()
        .collect();
    for id in stale {
        let destination = wiki_path(project, &id)?;
        if fs::read(&destination).ok().is_some_and(|bytes| {
            manifest
                .get(&id)
                .is_some_and(|old| old.rendered_hash == hex(&bytes))
        }) {
            fs::remove_file(destination)?;
            manifest.remove(&id);
        } else {
            report.protected.push(id);
        }
    }
    report.protected.sort();
    atomic_write(&manifest_path, &serde_json::to_vec_pretty(&manifest)?)?;
    Ok(report)
}

fn wiki_path(project: &Path, id: &str) -> Result<PathBuf> {
    let relative = crate::write::page_relative(id)?;
    safe_path(
        project,
        &Path::new("dokuwiki/data/pages")
            .join(relative)
            .with_extension("txt"),
    )
}

fn hex(bytes: &[u8]) -> String {
    sha256(bytes)
}
