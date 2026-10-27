use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs};
use tempfile::tempdir;
use wiki_writer::{
    config, project,
    render::{render, render_markdown},
    types::{
        Fact, FactDraft, ItemRecord, ItemStatus, Outline, PageKind, PageRef, PageSpec, Provider,
        Source,
    },
};

fn facts() -> (BTreeMap<String, Fact>, Vec<Source>) {
    let source = Source {
        id: "s0001".into(),
        name: "Book".into(),
        path: "sources/s0001.md".into(),
        sha256: "0".repeat(64),
    };
    let fact = Fact {
        id: "f0001".into(),
        chunk: "c0001".into(),
        draft: FactDraft {
            entity: "Copper".into(),
            entity_type: "item".into(),
            aliases: vec![],
            fields: BTreeMap::new(),
            text: "Weight".into(),
            quote: "Weight".into(),
            pages: vec![PageRef {
                source: "s0001".into(),
                page: 42,
            }],
            table_like: false,
        },
    };
    (BTreeMap::from([("f0001".into(), fact)]), vec![source])
}

#[test]
fn markdown_to_dokuwiki_formats_headings_links_styles_tables_and_citations() {
    let (facts, sources) = facts();
    let markdown = "# Heading\n\n**Bold** and *italic* with `code` [link](items:token) [^f0001] [^f0001].\n\n| A | B |\n|---|---|\n| x | y |\n";
    let output = render_markdown(markdown, &facts, &sources).unwrap();
    assert!(output.contains("====== Heading ======"));
    assert!(output.contains("**Bold**"));
    assert!(output.contains("//italic//"));
    assert!(output.contains("''code''"));
    assert!(output.contains("[[items:token|link]]"));
    assert_eq!(output.matches("((p. 42))").count(), 1);
    assert!(output.contains("^ A ^ B ^"));
    assert!(output.contains("| x | y |"));
}

#[test]
fn headings_lists_and_table_have_exact_dokuwiki_markup() {
    let (facts, sources) = facts();
    let markdown = "# One\n## Two\n### Three\n#### Four\n##### Five\n###### Six\n\n- outer\n  - inner\n\n1. ordered\n\n| A | B |\n|---|---|\n| x | y |\n";
    let output = render_markdown(markdown, &facts, &sources).unwrap();
    assert_eq!(
        output,
        "====== One ======\n\n===== Two =====\n\n==== Three ====\n\n=== Four ===\n\n== Five ==\n\n== Six ==\n\n  * outer\n    * inner\n\n  - ordered\n\n^ A ^ B ^\n| x | y |\n"
    );
}

#[test]
fn link_targets_cannot_inject_dokuwiki_markup() {
    let (facts, sources) = facts();
    let output = render_markdown("[Safe](https://example.test/a|b)", &facts, &sources).unwrap();
    assert!(
        output.contains("[[https://example.test/a%7Cb|Safe]]"),
        "{output}"
    );
    assert!(!output.contains("a|b|Safe"));
}

#[test]
fn unknown_citation_fails_and_code_keeps_literal_marker() {
    let (facts, sources) = facts();
    assert!(render_markdown("# A\nUnknown [^f9999]", &facts, &sources).is_err());
    let output = render_markdown("# A\n`[^f0001]` [^f0001]", &facts, &sources).unwrap();
    assert!(output.contains("f0001"), "{output}");
    assert_eq!(output.matches("((p. 42))").count(), 1);
}

#[test]
fn render_protects_wiki_edits_and_untracked_files() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.md");
    fs::write(
        &source,
        "# Demo\nThis source has enough text to be a real source for rendering.\n",
    )
    .unwrap();
    let project_dir =
        project::init(dir.path(), "demo", &[source], Provider::Codex, "generic").unwrap();
    let cfg = config::load(&project_dir).unwrap();
    let outline = Outline {
        pages: vec![PageSpec {
            id: "start".into(),
            title: "Home".into(),
            kind: PageKind::Index,
            entity_ids: vec![],
            fact_ids: vec![],
        }],
        gaps: vec![],
    };
    fs::write(
        project_dir.join("state/outline.yaml"),
        serde_yaml::to_string(&outline).unwrap(),
    )
    .unwrap();
    fs::write(project_dir.join("content/start.md"), "# Home\nWelcome.\n").unwrap();
    fs::create_dir_all(project_dir.join("state/items/write")).unwrap();
    fs::create_dir_all(project_dir.join("state/drafts")).unwrap();
    let draft = b"# Home\nWelcome.\n";
    fs::write(project_dir.join("state/drafts/start.md"), draft).unwrap();
    let digest: String = Sha256::digest(draft)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let record = ItemRecord {
        input_hash: "test".into(),
        output_hash: Some(digest.clone()),
        output_path: Some("state/drafts/start.md".into()),
        outputs: BTreeMap::from([("state/drafts/start.md".into(), digest)]),
        status: ItemStatus::Done,
        errors: vec![],
        escalated: false,
    };
    fs::write(
        project_dir.join("state/items/write/start.json"),
        serde_json::to_vec(&record).unwrap(),
    )
    .unwrap();
    fs::create_dir_all(project_dir.join("dokuwiki/data/pages")).unwrap();
    let first = render(&project_dir).unwrap();
    assert_eq!(first.written, 1);
    let second = render(&project_dir).unwrap();
    assert_eq!(second.unchanged, 1);
    let wiki = project_dir.join("dokuwiki/data/pages/start.txt");
    fs::write(&wiki, "user edit").unwrap();
    fs::write(project_dir.join("content/start.md"), "# Home\nChanged.\n").unwrap();
    let protected = render(&project_dir).unwrap();
    assert_eq!(protected.protected, vec!["start"]);
    assert_eq!(fs::read_to_string(wiki).unwrap(), "user edit");
    assert_eq!(cfg.project, "demo");
}

#[test]
fn citations_coalesce_ranges_and_name_sources_when_multiple_exist() {
    let (mut facts, mut sources) = facts();
    sources.push(Source {
        id: "s0002".into(),
        name: "Appendix".into(),
        path: "sources/s0002.md".into(),
        sha256: "1".repeat(64),
    });
    facts.get_mut("f0001").unwrap().draft.pages = vec![
        PageRef {
            source: "s0001".into(),
            page: 42,
        },
        PageRef {
            source: "s0001".into(),
            page: 43,
        },
        PageRef {
            source: "s0001".into(),
            page: 44,
        },
        PageRef {
            source: "s0002".into(),
            page: 2,
        },
    ];
    let output = render_markdown("# Sources\n[^f0001] [^f0001]", &facts, &sources).unwrap();
    assert_eq!(
        output
            .matches("((Book, pp. 42–44; Appendix, p. 2))")
            .count(),
        1
    );
}

#[test]
fn literal_nowiki_closer_is_split_and_table_delimiters_are_individually_escaped() {
    let (facts, sources) = facts();
    let output = render_markdown("# Escapes\nA*B |^ </nowiki>", &facts, &sources).unwrap();
    assert!(output.contains("A<nowiki>*</nowiki>B"));
    assert!(output.contains("<nowiki>|</nowiki><nowiki>^</nowiki>"));
    assert!(!output.contains("<nowiki></nowiki>"));
    assert!(output.contains("<nowiki><</nowiki><nowiki>/</nowiki>"));
}

#[test]
fn inline_code_does_not_create_wiki_links_or_end_code_span() {
    let (facts, sources) = facts();
    let output = render_markdown("`[[x]] and '' and %%`", &facts, &sources).unwrap();
    assert!(!output.contains("''[[x]]"));
    assert!(output.contains("<nowiki>[[</nowiki>x<nowiki>]]</nowiki>"));
    assert!(output.contains("<nowiki>''</nowiki>"));
    assert!(output.contains("<nowiki>%%</nowiki>"));
}

#[test]
fn render_refuses_symlinked_wiki_pages_directory() {
    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let source = dir.path().join("source.md");
    fs::write(
        &source,
        "# Demo\nThis source is long enough for project initialization.\n",
    )
    .unwrap();
    let project_dir =
        project::init(dir.path(), "demo", &[source], Provider::Codex, "generic").unwrap();
    let outline = Outline {
        pages: vec![PageSpec {
            id: "start".into(),
            title: "Home".into(),
            kind: PageKind::Index,
            entity_ids: vec![],
            fact_ids: vec![],
        }],
        gaps: vec![],
    };
    fs::write(
        project_dir.join("state/outline.yaml"),
        serde_yaml::to_string(&outline).unwrap(),
    )
    .unwrap();
    fs::write(project_dir.join("content/start.md"), "# Home\nWelcome.\n").unwrap();
    fs::create_dir_all(project_dir.join("state/items/write")).unwrap();
    fs::create_dir_all(project_dir.join("state/drafts")).unwrap();
    let draft = b"# Home\nWelcome.\n";
    fs::write(project_dir.join("state/drafts/start.md"), draft).unwrap();
    let digest: String = Sha256::digest(draft)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let record = ItemRecord {
        input_hash: "test".into(),
        output_hash: Some(digest.clone()),
        output_path: Some("state/drafts/start.md".into()),
        outputs: BTreeMap::from([("state/drafts/start.md".into(), digest)]),
        status: ItemStatus::Done,
        errors: vec![],
        escalated: false,
    };
    fs::write(
        project_dir.join("state/items/write/start.json"),
        serde_json::to_vec(&record).unwrap(),
    )
    .unwrap();
    fs::create_dir_all(project_dir.join("dokuwiki/data")).unwrap();
    std::os::unix::fs::symlink(outside.path(), project_dir.join("dokuwiki/data/pages")).unwrap();
    assert!(render(&project_dir).is_err());
    assert!(fs::read_dir(outside.path()).unwrap().next().is_none());
}
