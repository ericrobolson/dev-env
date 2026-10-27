use crate::{
    config::Config,
    project::verify_manifest,
    state::{atomic_write, input_hash, is_done, safe_path, sha256},
    types::{Chunk, ItemRecord, ItemStatus, Page, PageRef, Tier, TocEntry},
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Component, Path},
    process::Command,
};

const MIN: usize = 16_000;
const TARGET: usize = 20_000;
const MAX: usize = 24_000;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceExtract {
    digest: String,
    pages: Vec<Page>,
    toc: Vec<TocEntry>,
}

pub fn ingest(project: &Path, config: &Config) -> Result<Vec<Chunk>> {
    verify_manifest(project, config)?;
    let ingest_dir = safe_path(project, Path::new("state/ingest"))?;
    fs::create_dir_all(safe_path(project, Path::new("state/ingest/sources"))?)?;
    fs::create_dir_all(safe_path(project, Path::new("state/ingest/pages"))?)?;
    fs::create_dir_all(safe_path(project, Path::new("state/items/ingest"))?)?;
    let mut all_pages = Vec::new();
    let mut all_toc = Vec::new();
    let mut chunks = Vec::new();
    let mut gaps = Vec::new();
    for source in &config.sources {
        let source_path = safe_path(project, Path::new(&source.path))?;
        let bytes = fs::read(&source_path)?;
        let digest = hex(&bytes);
        let cache_path = ingest_dir
            .join("sources")
            .join(format!("{}.json", source.id));
        let record_path = project
            .join("state/items/ingest")
            .join(format!("{}.json", source.id));
        let hash = input_hash(
            "ingest",
            2,
            &serde_json::to_vec(&(&bytes, MIN, TARGET, MAX))?,
            Tier::Small,
        );
        let record = fs::read(&record_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<ItemRecord>(&bytes).ok());
        let cached = record
            .as_ref()
            .filter(|record| is_done(project, record, &hash))
            .and_then(|_| fs::read(&cache_path).ok())
            .and_then(|bytes| serde_json::from_slice::<SourceExtract>(&bytes).ok())
            .filter(|cache| cache.digest == digest);
        let extracted = if let Some(cache) = cached {
            cache
        } else {
            let parsed = match source_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
            {
                "md" | "txt" => extract_text(&source.id, &bytes),
                "pdf" => extract_pdf(&source.id, &source_path),
                "epub" => extract_epub(&source.id, &source_path),
                _ => bail!("unsupported source path: {}", source.path),
            };
            match parsed {
                Ok((pages, toc)) => {
                    let result = SourceExtract { digest, pages, toc };
                    let output = serde_json::to_vec_pretty(&result)?;
                    atomic_write(&cache_path, &output)?;
                    let relative = format!("state/ingest/sources/{}.json", source.id);
                    let output_hash = hex(&output);
                    let record = ItemRecord {
                        input_hash: hash.clone(),
                        output_hash: Some(output_hash.clone()),
                        output_path: Some(relative.clone()),
                        outputs: BTreeMap::from([(relative, output_hash)]),
                        status: ItemStatus::Done,
                        errors: vec![],
                        escalated: false,
                    };
                    atomic_write(&record_path, &serde_json::to_vec_pretty(&record)?)?;
                    result
                }
                Err(e) => {
                    let reason = e.to_string();
                    gaps.push(format!("ingest/{}: {}", source.id, reason));
                    let record = ItemRecord {
                        input_hash: hash,
                        output_hash: None,
                        output_path: None,
                        outputs: BTreeMap::new(),
                        status: ItemStatus::Blocked,
                        errors: vec![reason],
                        escalated: false,
                    };
                    atomic_write(&record_path, &serde_json::to_vec_pretty(&record)?)?;
                    continue;
                }
            }
        };
        let mut pages = extracted.pages;
        for page in &mut pages {
            page.id = (all_pages.len() + 1) as u32;
            if page.scanned {
                gaps.push(format!(
                    "ingest/{}: skipped page {} (little text)",
                    source.id, page.reference.page
                ));
            }
            write_if_changed(
                &ingest_dir.join("pages").join(format!("{:04}.txt", page.id)),
                page.text.as_bytes(),
            )?;
            all_pages.push(page.clone());
        }
        all_toc.extend(extracted.toc);
        if source.path.ends_with(".pdf") {
            for page in &pages {
                let next_id = chunks.len() + 1;
                chunks.extend(chunk_pages(std::slice::from_ref(page), &source.id, next_id));
            }
        } else {
            chunks.extend(chunk_pages(&pages, &source.id, chunks.len() + 1));
        }
    }
    gaps.sort();
    gaps.dedup();
    write_if_changed(
        &ingest_dir.join("pages.json"),
        &serde_json::to_vec_pretty(&all_pages)?,
    )?;
    write_if_changed(
        &ingest_dir.join("toc.json"),
        &serde_json::to_vec_pretty(&all_toc)?,
    )?;
    write_if_changed(
        &ingest_dir.join("chunks.json"),
        &serde_json::to_vec_pretty(&chunks)?,
    )?;
    write_if_changed(
        &project.join("state/gaps.json"),
        &serde_json::to_vec_pretty(&gaps)?,
    )?;
    let mut outputs = BTreeMap::new();
    for filename in ["pages.json", "toc.json", "chunks.json"] {
        let relative = format!("state/ingest/{filename}");
        outputs.insert(relative.clone(), hex(&fs::read(project.join(&relative))?));
    }
    for page in &all_pages {
        let relative = format!("state/ingest/pages/{:04}.txt", page.id);
        outputs.insert(relative.clone(), hex(&fs::read(project.join(&relative))?));
    }
    let mut source_inputs = Vec::new();
    for source in &config.sources {
        let bytes = fs::read(
            project
                .join("state/items/ingest")
                .join(format!("{}.json", source.id)),
        )?;
        let record: ItemRecord = serde_json::from_slice(&bytes)?;
        source_inputs.push((source.id.clone(), record.input_hash, record.output_hash));
    }
    let hash = input_hash(
        "ingest_global",
        1,
        &serde_json::to_vec(&source_inputs)?,
        Tier::Small,
    );
    let record = ItemRecord {
        input_hash: hash,
        output_hash: None,
        output_path: None,
        outputs,
        status: ItemStatus::Done,
        errors: vec![],
        escalated: false,
    };
    write_if_changed(
        &project.join("state/items/ingest/global.json"),
        &serde_json::to_vec_pretty(&record)?,
    )?;
    Ok(chunks)
}

pub fn verify_current(project: &Path, config: &Config) -> Result<()> {
    verify_manifest(project, config)?;
    let mut source_inputs = Vec::new();
    for source in &config.sources {
        let bytes = fs::read(safe_path(project, Path::new(&source.path))?)?;
        let hash = input_hash(
            "ingest",
            2,
            &serde_json::to_vec(&(&bytes, MIN, TARGET, MAX))?,
            Tier::Small,
        );
        let path = project
            .join("state/items/ingest")
            .join(format!("{}.json", source.id));
        let record: ItemRecord =
            serde_json::from_slice(&fs::read(path).context("ingest is required before --only")?)?;
        ensure!(
            record.input_hash == hash
                && (is_done(project, &record, &hash) || record.status == ItemStatus::Blocked),
            "ingest inputs changed or outputs missing; run without --only"
        );
        source_inputs.push((source.id.clone(), record.input_hash, record.output_hash));
    }
    let hash = input_hash(
        "ingest_global",
        1,
        &serde_json::to_vec(&source_inputs)?,
        Tier::Small,
    );
    let global: ItemRecord = serde_json::from_slice(
        &fs::read(project.join("state/items/ingest/global.json"))
            .context("ingest is required before --only")?,
    )?;
    ensure!(
        is_done(project, &global, &hash),
        "ingest outputs missing or stale; run without --only"
    );
    Ok(())
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<()> {
    if fs::read(path).ok().as_deref() == Some(bytes) {
        return Ok(());
    }
    atomic_write(path, bytes)
}

fn extract_text(source: &str, bytes: &[u8]) -> Result<(Vec<Page>, Vec<TocEntry>)> {
    let text = std::str::from_utf8(bytes).context("source is not UTF-8")?;
    let mut sections = Vec::<(String, Option<(String, u8)>)>::new();
    let mut current = String::new();
    let mut heading = None;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        let hashes = trimmed.bytes().take_while(|b| *b == b'#').count();
        if (1..=6).contains(&hashes) && trimmed.as_bytes().get(hashes) == Some(&b' ') {
            if !current.is_empty() {
                sections.push((std::mem::take(&mut current), heading.take()));
            }
            heading = Some((trimmed[hashes..].trim().to_owned(), hashes as u8));
        }
        current.push_str(line);
    }
    if !current.is_empty() {
        sections.push((current, heading));
    }
    let mut pages = Vec::new();
    let mut toc = Vec::new();
    for (index, (text, heading)) in sections.into_iter().enumerate() {
        let page = (index + 1) as u32;
        if let Some((title, level)) = heading {
            toc.push(TocEntry {
                title,
                source: source.into(),
                page,
                level,
            });
        }
        pages.push(Page {
            id: page,
            reference: PageRef {
                source: source.into(),
                page,
            },
            scanned: text.chars().filter(|c| !c.is_whitespace()).count() < 40,
            text,
        });
    }
    Ok((pages, toc))
}

fn extract_pdf(source: &str, path: &Path) -> Result<(Vec<Page>, Vec<TocEntry>)> {
    let info = Command::new("pdfinfo")
        .arg(path)
        .output()
        .context("pdfinfo failed to start")?;
    ensure!(
        info.status.success(),
        "pdfinfo failed: {}",
        String::from_utf8_lossy(&info.stderr)
    );
    let info = String::from_utf8(info.stdout).context("pdfinfo output is not UTF-8")?;
    let count: usize = info
        .lines()
        .find_map(|line| {
            line.strip_prefix("Pages:")
                .and_then(|rest| rest.trim().parse().ok())
        })
        .context("pdfinfo did not report Pages")?;
    ensure!(count > 0, "PDF has no pages");
    let output = Command::new("pdftotext")
        .arg("-raw")
        .arg(path)
        .arg("-")
        .output()
        .context("pdftotext failed to start")?;
    ensure!(
        output.status.success(),
        "pdftotext failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let all_text = String::from_utf8(output.stdout).context("pdftotext output is not UTF-8")?;
    let mut page_texts: Vec<&str> = all_text.split('\u{000c}').collect();
    if all_text.ends_with('\u{000c}') {
        page_texts.pop();
    }
    ensure!(
        page_texts.len() == count,
        "pdftotext returned {} pages; pdfinfo reported {count}",
        page_texts.len()
    );
    let mut pages = Vec::new();
    for (offset, text) in page_texts.into_iter().enumerate() {
        let index = offset + 1;
        pages.push(Page {
            id: index as u32,
            reference: PageRef {
                source: source.into(),
                page: index as u32,
            },
            scanned: text.chars().filter(|c| !c.is_whitespace()).count() < 40,
            text: text.into(),
        });
    }
    let toc = match pdf_outline(source, path, count as u32) {
        Ok(toc) => toc,
        Err(e) => {
            eprintln!("warning: ingest/{source}: PDF outline ignored: {e}");
            Vec::new()
        }
    };
    Ok((pages, toc))
}

fn pdf_outline(source: &str, path: &Path, count: u32) -> Result<Vec<TocEntry>> {
    let doc = lopdf::Document::load(path)?;
    let catalog = doc.catalog()?;
    let Some(outlines) = catalog
        .get(b"Outlines")
        .ok()
        .and_then(|obj| pdf_dict(&doc, obj))
    else {
        return Ok(vec![]);
    };
    let Some(first) = outlines.get(b"First").ok() else {
        return Ok(vec![]);
    };
    let pages = doc.get_pages();
    let page_ids: std::collections::BTreeMap<_, _> =
        pages.into_iter().map(|(number, id)| (id, number)).collect();
    let mut visited = BTreeSet::new();
    let mut found = Vec::<(u32, usize, String, u8)>::new();
    walk_pdf_outline(
        &doc,
        catalog,
        first.clone(),
        &page_ids,
        &mut visited,
        &mut found,
        1,
    );
    found.sort_by_key(|(page, order, _, _)| (*page, *order));
    found.dedup_by(|a, b| a.0 == b.0 && a.2 == b.2);
    Ok(found
        .into_iter()
        .filter(|(page, _, title, _)| *page > 0 && *page <= count && !title.trim().is_empty())
        .map(|(page, _, title, level)| TocEntry {
            title,
            source: source.into(),
            page,
            level,
        })
        .collect())
}

fn pdf_dict(doc: &lopdf::Document, object: &lopdf::Object) -> Option<lopdf::Dictionary> {
    doc.dereference(object).ok()?.1.as_dict().ok().cloned()
}

fn walk_pdf_outline(
    doc: &lopdf::Document,
    catalog: &lopdf::Dictionary,
    first: lopdf::Object,
    pages: &std::collections::BTreeMap<lopdf::ObjectId, u32>,
    visited: &mut BTreeSet<lopdf::ObjectId>,
    found: &mut Vec<(u32, usize, String, u8)>,
    level: u8,
) {
    let mut next = Some(first);
    let mut count = 0usize;
    while let Some(object) = next {
        count += 1;
        if count > 10_000 {
            break;
        }
        if let Ok(id) = object.as_reference()
            && !visited.insert(id)
        {
            break;
        }
        let Some(node) = pdf_dict(doc, &object) else {
            break;
        };
        let title = node
            .get(b"Title")
            .ok()
            .and_then(|obj| doc.dereference(obj).ok())
            .and_then(|(_, obj)| obj.as_str().ok())
            .map(|bytes| String::from_utf8_lossy(bytes).to_string());
        let destination = node.get(b"Dest").ok().cloned().or_else(|| {
            node.get(b"A")
                .ok()
                .and_then(|obj| pdf_dict(doc, obj))
                .and_then(|action| action.get(b"D").ok().cloned())
        });
        if let (Some(title), Some(destination)) = (title, destination)
            && let Some(page) = pdf_destination(doc, catalog, &destination, pages, 0)
        {
            found.push((page, found.len(), title, level));
        }
        if let Ok(child) = node.get(b"First") {
            walk_pdf_outline(
                doc,
                catalog,
                child.clone(),
                pages,
                visited,
                found,
                level.saturating_add(1),
            );
        }
        next = node.get(b"Next").ok().cloned();
    }
}

fn pdf_destination(
    doc: &lopdf::Document,
    catalog: &lopdf::Dictionary,
    object: &lopdf::Object,
    pages: &std::collections::BTreeMap<lopdf::ObjectId, u32>,
    depth: u8,
) -> Option<u32> {
    if depth > 8 {
        return None;
    }
    let resolved = doc.dereference(object).ok()?.1;
    match resolved {
        lopdf::Object::Array(items) => items
            .first()
            .and_then(|item| item.as_reference().ok())
            .and_then(|id| pages.get(&id).copied()),
        lopdf::Object::Dictionary(dict) => dict
            .get(b"D")
            .ok()
            .and_then(|d| pdf_destination(doc, catalog, d, pages, depth + 1)),
        lopdf::Object::Name(name) | lopdf::Object::String(name, _) => {
            if let Ok(dests) = catalog.get(b"Dests")
                && let Some(dict) = pdf_dict(doc, dests)
                && let Ok(dest) = dict.get(name)
                && let Some(page) = pdf_destination(doc, catalog, dest, pages, depth + 1)
            {
                return Some(page);
            }
            if let Ok(names) = catalog.get(b"Names")
                && let Some(names) = pdf_dict(doc, names)
                && let Ok(dests) = names.get(b"Dests")
                && let Some(tree) = pdf_dict(doc, dests)
                && let Ok(entries) = tree.get(b"Names").and_then(lopdf::Object::as_array)
            {
                for pair in entries.chunks_exact(2) {
                    if pair[0].as_str().ok() == Some(name.as_slice()) {
                        return pdf_destination(doc, catalog, &pair[1], pages, depth + 1);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn extract_epub(source: &str, path: &Path) -> Result<(Vec<Page>, Vec<TocEntry>)> {
    let file = fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file).context("invalid EPUB ZIP")?;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = Path::new(entry.name());
        ensure!(
            !name.is_absolute() && name.components().all(|c| matches!(c, Component::Normal(_))),
            "unsafe EPUB path"
        );
        ensure!(
            entry
                .unix_mode()
                .is_none_or(|mode| mode & 0o170000 != 0o120000),
            "EPUB symlink refused"
        );
    }
    let container = read_zip(&mut archive, "META-INF/container.xml")?;
    let document = roxmltree::Document::parse(&container).context("invalid EPUB container XML")?;
    let opf_path = document
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "rootfile")
        .and_then(|n| n.attribute("full-path"))
        .context("EPUB rootfile missing")?;
    let opf_path = safe_member("", opf_path)?;
    let opf = read_zip(&mut archive, &opf_path)?;
    let document = roxmltree::Document::parse(&opf).context("invalid EPUB OPF XML")?;
    let base = Path::new(&opf_path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    let mut manifest = std::collections::BTreeMap::<String, String>::new();
    let mut nav_path = None;
    let mut ncx_path = None;
    for item in document
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "item")
    {
        let (Some(id), Some(href)) = (item.attribute("id"), item.attribute("href")) else {
            continue;
        };
        let member = safe_member(base, href)?;
        if item
            .attribute("properties")
            .is_some_and(|p| p.split_whitespace().any(|word| word == "nav"))
        {
            nav_path = Some(member.clone());
        }
        if item.attribute("media-type") == Some("application/x-dtbncx+xml") {
            ncx_path = Some(member.clone());
        }
        manifest.insert(id.into(), member);
    }
    let mut pages = Vec::new();
    let mut toc = Vec::new();
    let mut member_pages = std::collections::BTreeMap::new();
    for item in document
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "itemref")
    {
        if item.attribute("linear") == Some("no") {
            continue;
        }
        let idref = item
            .attribute("idref")
            .context("EPUB spine itemref missing idref")?;
        let member = manifest
            .get(idref)
            .context("EPUB spine references missing manifest item")?;
        let xhtml = read_zip(&mut archive, member)?;
        let xml = roxmltree::Document::parse(&xhtml).context("invalid EPUB XHTML")?;
        let mut text = String::new();
        let mut title = None;
        for node in xml.descendants() {
            if node.is_element() {
                let name = node.tag_name().name();
                if name == "script" || name == "style" {
                    continue;
                }
                if matches!(
                    name,
                    "p" | "div"
                        | "h1"
                        | "h2"
                        | "h3"
                        | "h4"
                        | "h5"
                        | "h6"
                        | "li"
                        | "tr"
                        | "br"
                        | "section"
                        | "article"
                ) && !text.ends_with('\n')
                    && !text.is_empty()
                {
                    text.push('\n');
                }
                if (name == "td" || name == "th") && !text.ends_with(" | ") && !text.is_empty() {
                    text.push_str(" | ");
                }
                if title.is_none() && matches!(name, "h1" | "h2" | "h3") {
                    title = node.text().map(str::to_owned);
                }
            } else if node.is_text()
                && !node.ancestors().any(|ancestor| {
                    ancestor.is_element()
                        && matches!(ancestor.tag_name().name(), "script" | "style")
                })
            {
                text.push_str(node.text().unwrap_or(""));
            }
        }
        let page = (pages.len() + 1) as u32;
        member_pages.insert(member.clone(), page);
        if let Some(title) = title {
            toc.push(TocEntry {
                title,
                source: source.into(),
                page,
                level: 1,
            });
        }
        pages.push(Page {
            id: page,
            reference: PageRef {
                source: source.into(),
                page,
            },
            scanned: text.chars().filter(|c| !c.is_whitespace()).count() < 40,
            text,
        });
    }
    let mut navigation_toc = None;
    if let Some(nav) = nav_path {
        match read_epub_nav(&mut archive, &nav, &member_pages, source) {
            Ok(entries) if !entries.is_empty() => navigation_toc = Some(entries),
            Ok(_) => {}
            Err(e) => eprintln!("warning: ingest/{source}: optional EPUB nav ignored: {e}"),
        }
    }
    if navigation_toc.is_none()
        && let Some(ncx) = ncx_path
    {
        match read_epub_ncx(&mut archive, &ncx, &member_pages, source) {
            Ok(entries) if !entries.is_empty() => navigation_toc = Some(entries),
            Ok(_) => {}
            Err(e) => eprintln!("warning: ingest/{source}: optional EPUB NCX ignored: {e}"),
        }
    }
    if let Some(entries) = navigation_toc {
        toc = entries;
    }
    Ok((pages, toc))
}

fn read_epub_nav(
    archive: &mut zip::ZipArchive<fs::File>,
    path: &str,
    member_pages: &std::collections::BTreeMap<String, u32>,
    source: &str,
) -> Result<Vec<TocEntry>> {
    let xml = read_zip(archive, path)?;
    let document = roxmltree::Document::parse(&xml)?;
    let base = Path::new(path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    let mut toc = Vec::new();
    for nav in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "nav")
    {
        if !nav
            .attributes()
            .any(|attribute| attribute.name() == "type" && attribute.value() == "toc")
        {
            continue;
        }
        for link in nav
            .descendants()
            .filter(|node| node.is_element() && node.tag_name().name() == "a")
        {
            let Some(href) = link.attribute("href") else {
                continue;
            };
            let member = safe_member(base, href)?;
            let Some(page) = member_pages.get(&member) else {
                continue;
            };
            let title = link
                .descendants()
                .filter_map(|node| if node.is_text() { node.text() } else { None })
                .collect::<Vec<_>>()
                .join("")
                .trim()
                .to_owned();
            if !title.is_empty() {
                toc.push(TocEntry {
                    title,
                    source: source.into(),
                    page: *page,
                    level: 1,
                });
            }
        }
    }
    Ok(toc)
}

fn read_epub_ncx(
    archive: &mut zip::ZipArchive<fs::File>,
    path: &str,
    member_pages: &std::collections::BTreeMap<String, u32>,
    source: &str,
) -> Result<Vec<TocEntry>> {
    let xml = read_zip(archive, path)?;
    let document = roxmltree::Document::parse(&xml)?;
    let base = Path::new(path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    let mut toc = Vec::new();
    for point in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "navPoint")
    {
        let href = point
            .descendants()
            .find(|node| node.is_element() && node.tag_name().name() == "content")
            .and_then(|node| node.attribute("src"));
        let title = point
            .descendants()
            .find(|node| node.is_element() && node.tag_name().name() == "text")
            .and_then(|node| node.text());
        let (Some(href), Some(title)) = (href, title) else {
            continue;
        };
        let member = safe_member(base, href)?;
        if let Some(page) = member_pages.get(&member)
            && !title.trim().is_empty()
        {
            toc.push(TocEntry {
                title: title.trim().into(),
                source: source.into(),
                page: *page,
                level: 1,
            });
        }
    }
    Ok(toc)
}

fn read_zip(archive: &mut zip::ZipArchive<fs::File>, name: &str) -> Result<String> {
    let mut member = archive
        .by_name(name)
        .with_context(|| format!("EPUB member missing: {name}"))?;
    let mut bytes = Vec::new();
    member.read_to_end(&mut bytes)?;
    String::from_utf8(bytes).context("EPUB member is not UTF-8")
}

fn safe_member(base: &str, href: &str) -> Result<String> {
    let path = href.split(['#', '?']).next().unwrap_or("");
    let mut decoded = Vec::new();
    let raw = path.as_bytes();
    let mut index = 0;
    while index < raw.len() {
        if raw[index] == b'%' {
            ensure!(index + 2 < raw.len(), "invalid EPUB percent escape");
            let hi = (raw[index + 1] as char)
                .to_digit(16)
                .context("invalid EPUB percent escape")?;
            let lo = (raw[index + 2] as char)
                .to_digit(16)
                .context("invalid EPUB percent escape")?;
            decoded.push((hi * 16 + lo) as u8);
            index += 3;
        } else {
            decoded.push(raw[index]);
            index += 1;
        }
    }
    let path = String::from_utf8(decoded).context("EPUB href is not UTF-8")?;
    ensure!(!path.starts_with('/'), "absolute EPUB href refused");
    let mut parts = Vec::new();
    for part in format!("{base}/{path}").split('/') {
        match part {
            "" | "." => {}
            ".." => {
                ensure!(parts.pop().is_some(), "EPUB href escapes archive");
            }
            other => parts.push(other.to_owned()),
        }
    }
    Ok(parts.join("/"))
}

fn chunk_pages(pages: &[Page], source: &str, first_id: usize) -> Vec<Chunk> {
    let mut result = Vec::new();
    let mut text = String::new();
    let mut first = 0u32;
    let mut last = 0u32;
    let mut prior: Option<&Page> = None;
    for page in pages.iter().filter(|page| !page.scanned) {
        let chars = page.text.chars().count();
        if chars > MAX {
            if !text.is_empty() {
                push_chunk(
                    &mut result,
                    source,
                    first_id,
                    first,
                    last,
                    std::mem::take(&mut text),
                );
            }
            let all: Vec<char> = page.text.chars().collect();
            let mut start = 0;
            while start < all.len() {
                let mut end = (start + TARGET).min(all.len());
                if end < all.len()
                    && let Some(newline) = all[start..end].iter().rposition(|c| *c == '\n')
                    && newline > 0
                {
                    end = start + newline + 1;
                }
                push_chunk(
                    &mut result,
                    source,
                    first_id,
                    page.reference.page,
                    page.reference.page,
                    all[start..end].iter().collect(),
                );
                start = end;
            }
            prior = None;
            continue;
        }
        if !text.is_empty()
            && (text.chars().count() + chars > MAX
                || (text.chars().count() >= MIN && text.chars().count() + chars > TARGET))
        {
            push_chunk(
                &mut result,
                source,
                first_id,
                first,
                last,
                std::mem::take(&mut text),
            );
            first = 0;
            if let Some(previous) = prior
                && previous.text.chars().count() + chars <= MAX
            {
                text.push_str(&previous.text);
                first = previous.reference.page;
            }
        }
        if text.is_empty() {
            first = page.reference.page;
        }
        text.push_str(&page.text);
        last = page.reference.page;
        prior = Some(page);
    }
    if !text.is_empty() {
        push_chunk(&mut result, source, first_id, first, last, text);
    }
    result
}

fn push_chunk(
    result: &mut Vec<Chunk>,
    source: &str,
    first_id: usize,
    first: u32,
    last: u32,
    text: String,
) {
    let table_like = table_like(&text);
    result.push(Chunk {
        id: format!("c{:04}", first_id + result.len()),
        source: source.into(),
        first_page: first,
        last_page: last,
        text,
        table_like,
    });
}

fn table_like(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if lower.contains("stat block") || lower.contains("statblock") {
        return true;
    }
    let pipe = text
        .lines()
        .filter(|line| !line.trim().is_empty() && line.contains('|'))
        .count();
    if pipe >= 2 {
        return true;
    }
    text.lines()
        .filter(|line| {
            let columns = line
                .split("  ")
                .filter(|part| !part.trim().is_empty())
                .count();
            columns >= 3
        })
        .count()
        >= 2
}

fn hex(bytes: &[u8]) -> String {
    sha256(bytes)
}
