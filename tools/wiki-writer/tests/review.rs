use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs};
use tempfile::tempdir;
use wiki_writer::{
    review::{Finding, Review, sample_pages, validate_review},
    types::{ItemRecord, ItemStatus, Outline, PageKind, PageSpec, Stage},
    write::current_write_pages,
};

#[test]
fn review_samples_five_evenly_and_accepts_only_known_targeted_redo() {
    let pages: Vec<_> = (1..=11)
        .map(|index| PageSpec {
            id: format!("topics:p{index:02}"),
            title: format!("Page {index}"),
            kind: PageKind::Leaf,
            entity_ids: vec![],
            fact_ids: vec![],
        })
        .collect();
    let sample = sample_pages(&pages);
    assert_eq!(
        sample
            .iter()
            .map(|page| page.id.as_str())
            .collect::<Vec<_>>(),
        [
            "topics:p01",
            "topics:p03",
            "topics:p06",
            "topics:p08",
            "topics:p11"
        ]
    );
    let outline = Outline {
        pages,
        gaps: vec![],
    };
    let good = Review {
        findings: vec![Finding {
            stage: Stage::Write,
            item: "topics:p01".into(),
            reason: "Missing detail".into(),
        }],
    };
    assert!(validate_review(&good, &outline, &["c0001".into()]).is_empty());
    let bad = Review {
        findings: vec![Finding {
            stage: Stage::Audit,
            item: "c0001".into(),
            reason: "Invalid".into(),
        }],
    };
    assert!(!validate_review(&bad, &outline, &["c0001".into()]).is_empty());
    let bad = Review {
        findings: vec![Finding {
            stage: Stage::Extract,
            item: "c9999".into(),
            reason: "".into(),
        }],
    };
    assert!(!validate_review(&bad, &outline, &["c0001".into()]).is_empty());
}

#[test]
fn reviewable_pages_require_a_successful_intact_write() {
    let dir = tempdir().unwrap();
    let project = dir.path();
    let outline = Outline {
        pages: ["start", "topics:missing"]
            .into_iter()
            .map(|id| PageSpec {
                id: id.into(),
                title: id.into(),
                kind: PageKind::Leaf,
                entity_ids: vec![],
                fact_ids: vec![],
            })
            .collect(),
        gaps: vec![],
    };
    let draft = project.join("state/drafts/start.md");
    fs::create_dir_all(draft.parent().unwrap()).unwrap();
    fs::write(&draft, "# Start\n").unwrap();
    let digest: String = Sha256::digest(b"# Start\n")
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let record = ItemRecord {
        input_hash: "current".into(),
        output_hash: Some(digest.clone()),
        output_path: Some("state/drafts/start.md".into()),
        outputs: BTreeMap::from([("state/drafts/start.md".into(), digest)]),
        status: ItemStatus::Done,
        errors: vec![],
        escalated: false,
    };
    let path = project.join("state/items/write/start.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
    let pages = current_write_pages(project, &outline).unwrap();
    assert_eq!(
        pages.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
        ["start"]
    );
    fs::remove_file(draft).unwrap();
    assert!(current_write_pages(project, &outline).unwrap().is_empty());
}
