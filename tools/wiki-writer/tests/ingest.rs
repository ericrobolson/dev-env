use std::{fs, io::Write, os::unix::fs::PermissionsExt, path::PathBuf};
use tempfile::tempdir;
use wiki_writer::{config, ingest, project, types::Provider};

#[test]
fn init_copies_multiple_sources_with_stable_distinct_ids() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("one")).unwrap();
    fs::create_dir(dir.path().join("two")).unwrap();
    let one = dir.path().join("one/source.md");
    let two = dir.path().join("two/source.md");
    fs::write(
        &one,
        "# One\nA long enough description of one source with meaningful information.\n",
    )
    .unwrap();
    fs::write(
        &two,
        "# Two\nA long enough description of another source with meaningful information.\n",
    )
    .unwrap();
    let project_dir =
        project::init(dir.path(), "demo", &[one, two], Provider::Codex, "generic").unwrap();
    assert_eq!(
        fs::canonicalize(&project_dir).unwrap(),
        fs::canonicalize(dir.path().join("wikis/demo")).unwrap()
    );
    assert!(project_dir.join("sources/s0001.md").exists());
    assert!(project_dir.join("sources/s0002.md").exists());
    let cfg = config::load(&project_dir).unwrap();
    assert_eq!(cfg.sources.len(), 2);
    assert_eq!(cfg.sources[0].name, "source.md");
    assert_eq!(cfg.sources[1].name, "source.md");
    assert!(
        project::init(
            dir.path(),
            "demo",
            &[PathBuf::from("one/source.md")],
            Provider::Codex,
            "generic"
        )
        .is_err()
    );
}

#[test]
fn text_headings_local_pages_and_scanned_skips_are_deterministic() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.md");
    fs::write(
        &source,
        format!("# Short\nsmall\n# Long\n{}\n", "A".repeat(45)),
    )
    .unwrap();
    let project_dir =
        project::init(dir.path(), "demo", &[source], Provider::Claude, "generic").unwrap();
    let cfg = config::load(&project_dir).unwrap();
    let chunks = ingest::ingest(&project_dir, &cfg).unwrap();
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].text.contains(&"A".repeat(45)));
    assert!(!chunks[0].text.contains("small"));
    let pages: Vec<serde_json::Value> =
        serde_json::from_slice(&fs::read(project_dir.join("state/ingest/pages.json")).unwrap())
            .unwrap();
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0]["reference"]["page"], 1);
    assert_eq!(pages[1]["reference"]["page"], 2);
    assert_eq!(pages[0]["scanned"], true);
    assert_eq!(pages[1]["scanned"], false);
    assert!(project_dir.join("state/ingest/pages/0001.txt").exists());
    assert!(project_dir.join("state/ingest/pages/0002.txt").exists());
    let page_path = project_dir.join("state/ingest/pages/0002.txt");
    let modified = fs::metadata(&page_path).unwrap().modified().unwrap();
    let again = ingest::ingest(&project_dir, &cfg).unwrap();
    assert_eq!(chunks, again);
    assert_eq!(
        fs::metadata(&page_path).unwrap().modified().unwrap(),
        modified
    );
    assert!(project_dir.join("state/items/ingest/s0001.json").exists());
}

#[test]
fn source_manifest_rejects_added_sources_and_init_rejects_duplicates() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("one.txt");
    fs::write(
        &source,
        "content with enough meaningful text to form a page and a chunk",
    )
    .unwrap();
    assert!(
        project::init(
            dir.path(),
            "bad",
            &[source.clone(), source.clone()],
            Provider::Codex,
            "generic"
        )
        .is_err()
    );
    let project_dir =
        project::init(dir.path(), "good", &[source], Provider::Codex, "generic").unwrap();
    let mut cfg = config::load(&project_dir).unwrap();
    cfg.sources.push(cfg.sources[0].clone());
    assert!(ingest::ingest(&project_dir, &cfg).is_err());
}

fn make_epub(path: &std::path::Path, escape: bool) {
    let file = fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    for (name, body) in [
        (
            "META-INF/container.xml",
            r#"<container><rootfiles><rootfile full-path="OEBPS/book.opf"/></rootfiles></container>"#,
        ),
        (
            "OEBPS/book.opf",
            r#"<package><manifest><item id="one" href="Text/chapter.xhtml"/><item id="nav" href="nav.xhtml" properties="nav"/></manifest><spine><itemref idref="one"/></spine></package>"#,
        ),
        (
            "OEBPS/Text/chapter.xhtml",
            r#"<html><body><h1>Chapter</h1><p>Copper is 12 grams &amp; silver is 24 grams, a useful comparison.</p><p>Additional facts make this page clearly longer than forty characters.</p></body></html>"#,
        ),
        (
            "OEBPS/nav.xhtml",
            r#"<html xmlns:epub="http://www.idpf.org/2007/ops"><body><nav epub:type="toc"><ol><li><a href="Text/chapter.xhtml">Navigation Title</a></li></ol></nav></body></html>"#,
        ),
    ] {
        zip.start_file(name, opts).unwrap();
        zip.write_all(body.as_bytes()).unwrap();
    }
    if escape {
        zip.start_file("../outside.txt", opts).unwrap();
        zip.write_all(b"bad").unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn epub_spine_text_and_nav_title_are_used_without_extraction() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("book.epub");
    make_epub(&source, false);
    let project_dir =
        project::init(dir.path(), "epub", &[source], Provider::Codex, "generic").unwrap();
    let chunks = ingest::ingest(&project_dir, &config::load(&project_dir).unwrap()).unwrap();
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].text.contains("12 grams & silver"));
    let toc: Vec<serde_json::Value> =
        serde_json::from_slice(&fs::read(project_dir.join("state/ingest/toc.json")).unwrap())
            .unwrap();
    assert_eq!(toc[0]["title"], "Navigation Title");
    assert!(!project_dir.join("outside.txt").exists());
}

#[test]
fn epub_uses_ncx_when_nav_has_no_toc_entries() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("book.epub");
    let file = fs::File::create(&source).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    for (name, body) in [
        (
            "META-INF/container.xml",
            r#"<container><rootfiles><rootfile full-path="OEBPS/book.opf"/></rootfiles></container>"#,
        ),
        (
            "OEBPS/book.opf",
            r#"<package><manifest><item id="one" href="chapter.xhtml"/><item id="nav" href="nav.xhtml" properties="nav"/><item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/></manifest><spine><itemref idref="one"/></spine></package>"#,
        ),
        (
            "OEBPS/chapter.xhtml",
            r#"<html><body><h1>Chapter heading</h1><p>This is enough chapter content to make a useful page with clear factual details.</p></body></html>"#,
        ),
        (
            "OEBPS/nav.xhtml",
            r#"<html><body><nav><ol></ol></nav></body></html>"#,
        ),
        (
            "OEBPS/toc.ncx",
            r#"<ncx><navMap><navPoint><navLabel><text>NCX title</text></navLabel><content src="chapter.xhtml"/></navPoint></navMap></ncx>"#,
        ),
    ] {
        zip.start_file(name, opts).unwrap();
        zip.write_all(body.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
    let project_dir =
        project::init(dir.path(), "epub", &[source], Provider::Codex, "generic").unwrap();
    ingest::ingest(&project_dir, &config::load(&project_dir).unwrap()).unwrap();
    let toc: Vec<serde_json::Value> =
        serde_json::from_slice(&fs::read(project_dir.join("state/ingest/toc.json")).unwrap())
            .unwrap();
    assert_eq!(toc[0]["title"], "NCX title");
}

#[test]
fn epub_rejects_zip_traversal_without_writing_outside() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("book.epub");
    make_epub(&source, true);
    let project_dir =
        project::init(dir.path(), "epub", &[source], Provider::Codex, "generic").unwrap();
    let chunks = ingest::ingest(&project_dir, &config::load(&project_dir).unwrap()).unwrap();
    assert!(chunks.is_empty());
    assert!(!dir.path().join("outside.txt").exists());
    let gaps: Vec<String> =
        serde_json::from_slice(&fs::read(project_dir.join("state/gaps.json")).unwrap()).unwrap();
    assert!(gaps[0].contains("unsafe EPUB path"));
}

#[test]
fn oversized_unicode_page_splits_without_dropping_text_and_flags_table() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("long.txt");
    let text = format!("# Long\n{}\n| a | b |\n| c | d |\n", "é".repeat(50_000));
    fs::write(&source, &text).unwrap();
    let project_dir =
        project::init(dir.path(), "long", &[source], Provider::Codex, "generic").unwrap();
    let chunks = ingest::ingest(&project_dir, &config::load(&project_dir).unwrap()).unwrap();
    assert!(chunks.len() >= 3);
    assert_eq!(
        chunks
            .iter()
            .map(|chunk| chunk.text.as_str())
            .collect::<String>(),
        text
    );
    assert!(
        chunks
            .iter()
            .all(|chunk| chunk.text.chars().count() <= 24_000)
    );
    assert!(chunks.iter().any(|chunk| chunk.table_like));
}

#[test]
fn pdf_uses_page_commands_and_outline_destinations() {
    use lopdf::{Document, Object, dictionary};
    let dir = tempdir().unwrap();
    let source = dir.path().join("book with spaces.pdf");
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let page_one = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages_id });
    let page_two = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages_id });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page_one.into(), page_two.into()], "Count" => 2,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        }),
    );
    let outline = doc.add_object(
        dictionary! { "Title" => Object::string_literal("Second page"),
        "Dest" => vec![Object::Reference(page_two), Object::Name(b"Fit".to_vec())] },
    );
    let outlines = doc.add_object(dictionary! { "Type" => "Outlines", "First" => outline });
    let catalog = doc.add_object(
        dictionary! { "Type" => "Catalog", "Pages" => pages_id, "Outlines" => outlines },
    );
    doc.trailer.set("Root", catalog);
    doc.save(&source).unwrap();
    let bin = dir.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let info = bin.join("pdfinfo");
    fs::write(&info, "#!/bin/sh\nprintf 'Pages: 2\\n'\n").unwrap();
    fs::set_permissions(&info, fs::Permissions::from_mode(0o755)).unwrap();
    let text = bin.join("pdftotext");
    fs::write(&text, "#!/bin/sh\ncase \" $* \" in *' -raw '*) ;; *) exit 9;; esac\nprintf 'This is a long enough PDF page to contain actual factual content for this source.\\n\\fAnother long enough PDF page to contain actual factual content for this source.\\n\\f'\n").unwrap();
    fs::set_permissions(&text, fs::Permissions::from_mode(0o755)).unwrap();
    let old_path = std::env::var_os("PATH").unwrap_or_default();
    unsafe {
        std::env::set_var(
            "PATH",
            format!("{}:{}", bin.display(), old_path.to_string_lossy()),
        );
    }
    let project_dir =
        project::init(dir.path(), "pdf", &[source], Provider::Codex, "generic").unwrap();
    let chunks = ingest::ingest(&project_dir, &config::load(&project_dir).unwrap()).unwrap();
    unsafe {
        std::env::set_var("PATH", old_path);
    }
    assert_eq!(chunks.len(), 2);
    assert_eq!(
        chunks
            .iter()
            .map(|c| (c.first_page, c.last_page))
            .collect::<Vec<_>>(),
        [(1, 1), (2, 2)]
    );
    let toc: Vec<serde_json::Value> =
        serde_json::from_slice(&fs::read(project_dir.join("state/ingest/toc.json")).unwrap())
            .unwrap();
    assert_eq!(toc[0]["title"], "Second page");
    assert_eq!(toc[0]["page"], 2);
}

#[test]
fn changed_copied_source_reextracts_only_that_source() {
    let dir = tempdir().unwrap();
    let first = dir.path().join("first.md");
    let second = dir.path().join("second.md");
    fs::write(&first, format!("# First\n{}", "a".repeat(45))).unwrap();
    fs::write(&second, format!("# Second\n{}", "b".repeat(45))).unwrap();
    let project_dir = project::init(
        dir.path(),
        "demo",
        &[first, second],
        Provider::Codex,
        "generic",
    )
    .unwrap();
    let cfg = config::load(&project_dir).unwrap();
    ingest::ingest(&project_dir, &cfg).unwrap();
    let second_cache = project_dir.join("state/ingest/sources/s0002.json");
    let before = fs::metadata(&second_cache).unwrap().modified().unwrap();
    let original_digest = cfg.sources[0].sha256.clone();
    fs::write(
        project_dir.join("sources/s0001.md"),
        format!("# First\n{}", "c".repeat(45)),
    )
    .unwrap();
    let chunks = ingest::ingest(&project_dir, &cfg).unwrap();
    assert!(chunks[0].text.contains(&"c".repeat(45)));
    assert_eq!(
        fs::metadata(&second_cache).unwrap().modified().unwrap(),
        before
    );
    assert_eq!(
        config::load(&project_dir).unwrap().sources[0].sha256,
        original_digest
    );
    assert!(project_dir.join("state/items/ingest/global.json").exists());
}

#[test]
fn ingest_refuses_symlinked_generated_directory() {
    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let source = dir.path().join("source.md");
    fs::write(&source, format!("# Source\n{}", "text ".repeat(20))).unwrap();
    let project_dir =
        project::init(dir.path(), "demo", &[source], Provider::Codex, "generic").unwrap();
    std::os::unix::fs::symlink(outside.path(), project_dir.join("state/ingest")).unwrap();
    assert!(ingest::ingest(&project_dir, &config::load(&project_dir).unwrap()).is_err());
    assert!(fs::read_dir(outside.path()).unwrap().next().is_none());
}
