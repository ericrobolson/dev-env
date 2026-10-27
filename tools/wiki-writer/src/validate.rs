use crate::types::{Chunk, Entity, Fact, FactDraft, Outline, PageKind, Schema};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use regex::Regex;
use std::{collections::BTreeSet, sync::LazyLock};

static NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[+-]?[0-9]+(?:[.,][0-9]+)*%?").unwrap());
static NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9_]*$").unwrap());

pub fn validate_schema(schema: &Schema) -> Vec<String> {
    let mut errors = Vec::new();
    if schema.entity_types.is_empty() {
        errors.push("entity_types must contain at least one type".into());
    }
    if schema.namespaces.is_empty() {
        errors.push("namespaces must contain at least one namespace".into());
    }
    for (name, entity_type) in &schema.entity_types {
        if !NAME.is_match(name) {
            errors.push(format!("invalid entity_type {name}"));
        }
        let mut fields = BTreeSet::new();
        for field in &entity_type.fields {
            if !NAME.is_match(field) {
                errors.push(format!("invalid field {field} for {name}"));
            }
            if !fields.insert(field) {
                errors.push(format!("duplicate field {field} for {name}"));
            }
        }
    }
    let mut namespaces = BTreeSet::new();
    for name in &schema.namespaces {
        if !NAME.is_match(name) {
            errors.push(format!("invalid namespace {name}"));
        }
        if !namespaces.insert(name) {
            errors.push(format!("duplicate namespace {name}"));
        }
    }
    errors
}

pub fn grounding(fact: &FactDraft, chunk: &Chunk, schema: &Schema) -> Vec<String> {
    let mut errors = Vec::new();
    for (field, value) in [
        ("entity", &fact.entity),
        ("entity_type", &fact.entity_type),
        ("text", &fact.text),
        ("quote", &fact.quote),
    ] {
        if value.trim().is_empty() {
            errors.push(format!("{field} must be nonempty"));
        }
    }
    match schema.entity_types.get(&fact.entity_type) {
        Some(kind) => {
            for field in fact.fields.keys() {
                if !kind.fields.contains(field) {
                    errors.push(format!(
                        "unknown field {field} for entity_type {}",
                        fact.entity_type
                    ));
                }
            }
        }
        None => errors.push(format!(
            "unknown entity_type {}; allowed: {:?}",
            fact.entity_type,
            schema.entity_types.keys().collect::<Vec<_>>()
        )),
    }
    let quote = normalize(&fact.quote);
    if !quote.is_empty() && !normalize(&chunk.text).contains(&quote) {
        errors.push(format!(
            "quote {:?} is not in chunk {}",
            fact.quote, chunk.id
        ));
    }
    let quote_numbers: BTreeSet<_> = NUMBER
        .find_iter(&quote)
        .map(|match_| match_.as_str().to_owned())
        .collect();
    for (label, value) in std::iter::once(("entity", &fact.entity))
        .chain(fact.aliases.iter().map(|value| ("alias", value)))
        .chain(std::iter::once(("text", &fact.text)))
        .chain(
            fact.fields
                .iter()
                .map(|(field, value)| (field.as_str(), value)),
        )
    {
        for token in NUMBER.find_iter(&normalize(value)) {
            if !quote_numbers.contains(token.as_str()) {
                errors.push(format!(
                    "{label} numeric token {} absent from quote",
                    token.as_str()
                ));
            }
        }
    }
    if fact.pages.is_empty() {
        errors.push("pages must be nonempty".into());
    }
    for page in &fact.pages {
        if page.source != chunk.source {
            errors.push(format!(
                "page source {} must equal {}",
                page.source, chunk.source
            ));
        }
        if page.page < chunk.first_page || page.page > chunk.last_page {
            errors.push(format!(
                "page {} outside allowed range {}..{}",
                page.page, chunk.first_page, chunk.last_page
            ));
        }
    }
    errors
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub fn validate_outline(
    outline: &Outline,
    schema: &Schema,
    entities: &[Entity],
    facts: &[Fact],
) -> Vec<String> {
    let mut errors = Vec::new();
    let entity_map: std::collections::BTreeMap<_, _> = entities
        .iter()
        .map(|entity| (entity.id.as_str(), entity))
        .collect();
    let fact_ids: BTreeSet<_> = facts.iter().map(|fact| fact.id.as_str()).collect();
    let mut seen_pages = BTreeSet::new();
    let mut assigned_entities = BTreeSet::new();
    let mut assigned_facts = BTreeSet::new();
    let mut start_count = 0;
    for page in &outline.pages {
        if !seen_pages.insert(&page.id) {
            errors.push(format!("duplicate page ID {}", page.id));
        }
        if page.title.trim().is_empty() {
            errors.push(format!("empty title for {}", page.id));
        }
        if page.id == "start" {
            start_count += 1;
            if page.kind != PageKind::Index {
                errors.push("start must be index kind".into());
            }
        } else {
            let mut parts = page.id.split(':');
            let namespace = parts.next().unwrap_or("");
            let slug = parts.next().unwrap_or("");
            if parts.next().is_some()
                || !schema.namespaces.iter().any(|name| name == namespace)
                || !NAME.is_match(slug)
            {
                errors.push(format!("invalid page ID {}", page.id));
            }
        }
        let mut page_entities = BTreeSet::new();
        for id in &page.entity_ids {
            if !page_entities.insert(id) {
                errors.push(format!("duplicate entity {id} on {}", page.id));
            }
            if !entity_map.contains_key(id.as_str()) {
                errors.push(format!("unknown entity {id} on {}", page.id));
            }
            assigned_entities.insert(id.as_str());
        }
        let mut page_facts = BTreeSet::new();
        for id in &page.fact_ids {
            if !page_facts.insert(id) {
                errors.push(format!("duplicate fact {id} on {}", page.id));
            }
            if !fact_ids.contains(id.as_str()) {
                errors.push(format!("unknown fact {id} on {}", page.id));
            }
            if !page.entity_ids.iter().any(|entity_id| {
                entity_map
                    .get(entity_id.as_str())
                    .is_some_and(|entity| entity.fact_ids.contains(id))
            }) {
                errors.push(format!(
                    "fact {id} does not belong to a page entity on {}",
                    page.id
                ));
            }
            assigned_facts.insert(id.as_str());
        }
    }
    if start_count != 1 {
        errors.push(format!("expected one start index, found {start_count}"));
    }
    for entity in entities {
        if !assigned_entities.contains(entity.id.as_str()) {
            errors.push(format!("unassigned entity {}", entity.id));
        }
    }
    for fact in facts {
        if !assigned_facts.contains(fact.id.as_str()) {
            errors.push(format!("uncovered fact {}", fact.id));
        }
    }
    if entities.is_empty()
        && (outline.pages.len() != 1
            || !outline.pages[0].entity_ids.is_empty()
            || !outline.pages[0].fact_ids.is_empty())
    {
        errors.push("empty entity set permits only empty start page".into());
    }
    errors
}

pub fn validate_markdown(
    markdown: &str,
    page: &crate::types::PageSpec,
    outline: &Outline,
) -> Vec<String> {
    let mut errors = Vec::new();
    let assigned: BTreeSet<_> = page.fact_ids.iter().map(String::as_str).collect();
    let valid_pages: BTreeSet<_> = outline.pages.iter().map(|page| page.id.as_str()).collect();
    let citation = Regex::new(r"\[\^f[0-9]{4,}\]").unwrap();
    let looking = Regex::new(r"\[\^f[^\]]*\]").unwrap();
    let dokulink = Regex::new(r"\[\[([^\]|]+)(?:\|[^\]]*)?\]\]").unwrap();
    let mut seen = BTreeSet::new();
    let mut code_start = None;
    let mut code_spans = Vec::new();
    let mut heading_seen = false;
    for (event, range) in Parser::new(markdown).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_start = Some(range.start),
            Event::End(TagEnd::CodeBlock) => {
                if let Some(start) = code_start.take() {
                    code_spans.push((start, range.end));
                }
            }
            Event::Code(_) => code_spans.push((range.start, range.end)),
            Event::Start(Tag::Heading { level, .. }) if !heading_seen => {
                heading_seen = true;
                if level != pulldown_cmark::HeadingLevel::H1 {
                    errors.push("first heading must be H1".into());
                }
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                if let Some(reason) = invalid_link(&dest_url, &page.id, &valid_pages) {
                    errors.push(reason);
                }
            }
            _ => {}
        }
    }
    for found in looking.find_iter(markdown) {
        if code_spans
            .iter()
            .any(|(start, end)| found.start() >= *start && found.end() <= *end)
        {
            continue;
        }
        let marker = found.as_str();
        if !citation.is_match(marker) || citation.find(marker).is_none_or(|m| m.as_str() != marker)
        {
            errors.push(format!("malformed citation {marker}"));
        } else {
            let id = &marker[2..marker.len() - 1];
            if !assigned.contains(id) {
                errors.push(format!("unknown citation {id} on {}", page.id));
            } else {
                seen.insert(id);
            }
        }
    }
    for captures in dokulink.captures_iter(markdown) {
        let found = captures.get(0).unwrap();
        if code_spans
            .iter()
            .any(|(start, end)| found.start() >= *start && found.end() <= *end)
        {
            continue;
        }
        if let Some(reason) = invalid_link(&captures[1], &page.id, &valid_pages) {
            errors.push(reason);
        }
    }
    if !heading_seen {
        errors.push("missing H1 heading".into());
    }
    for id in assigned {
        if !seen.contains(id) {
            errors.push(format!("missing citation {id} on {}", page.id));
        }
    }
    errors
}

fn invalid_link(dest: &str, current: &str, valid_pages: &BTreeSet<&str>) -> Option<String> {
    if dest.starts_with("http://") || dest.starts_with("https://") || dest.starts_with("mailto:") {
        return None;
    }
    let target = dest.split('#').next().unwrap_or("");
    let target = if target.is_empty() { current } else { target };
    if target.contains("://") || (target.contains(':') && !valid_pages.contains(target)) {
        return Some(format!("broken link {dest}"));
    }
    if !valid_pages.contains(target) {
        return Some(format!("broken link {dest}"));
    }
    None
}
