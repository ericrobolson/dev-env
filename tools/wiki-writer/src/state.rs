use crate::types::{Fact, FactDraft, ItemRecord, ItemStatus, PageRef, Tier};
use anyhow::{Context, Result, ensure};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Component, Path},
};

pub(crate) fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    atomic_write_inner(path, bytes, || Ok(()))
}

fn atomic_write_inner(
    path: &Path,
    bytes: &[u8],
    before_rename: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let parent = path.parent().context("output has no parent")?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.flush()?;
    temp.as_file().sync_all()?;
    before_rename()?;
    temp.persist(path).map_err(|e| e.error)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn injected_failure_before_rename_keeps_prior_complete_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("value");
        atomic_write(&path, b"old").unwrap();
        assert!(atomic_write_inner(&path, b"new", || anyhow::bail!("injected failure")).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"old");
    }

    #[test]
    fn fact_registry_reuses_id_and_never_recycles_removed_keys() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("state")).unwrap();
        let draft = FactDraft {
            entity: "Copper token".into(),
            entity_type: "item".into(),
            aliases: vec!["token".into()],
            fields: BTreeMap::new(),
            text: "12 grams".into(),
            quote: "12 grams".into(),
            pages: vec![PageRef {
                source: "s0001".into(),
                page: 1,
            }],
            table_like: false,
        };
        let first = identify_facts(
            dir.path(),
            vec![
                ("c0002".into(), "s0001".into(), draft.clone()),
                ("c0001".into(), "s0001".into(), draft.clone()),
            ],
        )
        .unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].id, "f0001");
        assert_eq!(first[0].chunk, "c0001");
        let same = identify_facts(
            dir.path(),
            vec![("c0003".into(), "s0001".into(), draft.clone())],
        )
        .unwrap();
        assert_eq!(same[0].id, "f0001");
        let mut changed = draft;
        changed.text = "24 grams".into();
        let new =
            identify_facts(dir.path(), vec![("c0004".into(), "s0001".into(), changed)]).unwrap();
        assert_eq!(new[0].id, "f0002");
    }
}

pub fn input_hash(template_name: &str, version: u32, input: &[u8], ceiling: Tier) -> String {
    sha256(
        &serde_json::to_vec(&(template_name, version, input, ceiling))
            .expect("serializable hash input"),
    )
}

pub fn is_done(project: &Path, record: &ItemRecord, hash: &str) -> bool {
    if record.status != ItemStatus::Done || record.input_hash != hash || record.outputs.is_empty() {
        return false;
    }
    record.outputs.iter().all(|(path, digest)| {
        let Ok(target) = safe_path(project, Path::new(path)) else {
            return false;
        };
        if let Ok(meta) = fs::symlink_metadata(&target) {
            if !meta.is_file() || meta.file_type().is_symlink() {
                return false;
            }
        } else {
            return false;
        }
        fs::read(target)
            .map(|bytes| sha256(&bytes) == *digest)
            .unwrap_or(false)
    })
}

pub(crate) fn is_blocked(record: &ItemRecord, hash: &str) -> bool {
    record.status == ItemStatus::Blocked && record.input_hash == hash
}

pub fn lock(project: &Path) -> Result<File> {
    let state = project.join("state");
    if let Ok(meta) = fs::symlink_metadata(&state) {
        ensure!(!meta.file_type().is_symlink(), "state symlink refused");
    }
    fs::create_dir_all(&state)?;
    let path = state.join("lock");
    if let Ok(meta) = fs::symlink_metadata(&path) {
        ensure!(!meta.file_type().is_symlink(), "lock symlink refused");
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    file.try_lock_exclusive().context("project is locked")?;
    Ok(file)
}

pub(crate) fn safe_path(project: &Path, relative: &Path) -> Result<std::path::PathBuf> {
    ensure!(!relative.is_absolute(), "absolute project path refused");
    let mut target = project.to_path_buf();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            anyhow::bail!("path traversal refused");
        };
        target.push(part);
        if let Ok(meta) = fs::symlink_metadata(&target) {
            ensure!(
                !meta.file_type().is_symlink(),
                "symlink project path refused: {}",
                target.display()
            );
        }
    }
    Ok(target)
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FactIdentity {
    source: String,
    pages: Vec<PageRef>,
    entity_type: String,
    entity: String,
    text: String,
    quote: String,
    fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisteredFact {
    identity: FactIdentity,
    id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FactRegistry {
    next: u64,
    entries: BTreeMap<String, RegisteredFact>,
}

pub(crate) fn identify_facts(
    project: &Path,
    drafts: Vec<(String, String, FactDraft)>,
) -> Result<Vec<Fact>> {
    let path = project.join("state/fact_ids.json");
    let mut registry = if path.exists() {
        serde_json::from_slice::<FactRegistry>(&fs::read(&path)?)?
    } else {
        FactRegistry {
            next: 1,
            entries: BTreeMap::new(),
        }
    };
    let mut keyed = Vec::new();
    for (chunk, source, draft) in drafts {
        let mut pages = draft.pages.clone();
        pages.sort();
        pages.dedup();
        let identity = FactIdentity {
            source,
            pages,
            entity_type: normalize(&draft.entity_type),
            entity: normalize(&draft.entity),
            text: normalize(&draft.text),
            quote: normalize(&draft.quote),
            fields: draft
                .fields
                .iter()
                .map(|(key, value)| (key.clone(), normalize(value)))
                .collect(),
        };
        let digest = sha256(&serde_json::to_vec(&identity)?);
        keyed.push((chunk, digest, identity, draft));
    }
    keyed.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    let mut merged = BTreeMap::<String, Fact>::new();
    for (chunk, digest, identity, draft) in keyed {
        let id = if let Some(found) = registry.entries.get(&digest) {
            ensure!(found.identity == identity, "fact identity digest collision");
            found.id.clone()
        } else {
            let id = format!("f{:04}", registry.next);
            registry.next += 1;
            registry.entries.insert(
                digest,
                RegisteredFact {
                    identity,
                    id: id.clone(),
                },
            );
            id
        };
        match merged.entry(id.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(Fact { id, chunk, draft });
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let fact = entry.get_mut();
                if chunk < fact.chunk {
                    fact.chunk = chunk;
                }
                fact.draft.aliases.extend(draft.aliases);
                fact.draft.aliases.sort();
                fact.draft.aliases.dedup();
                fact.draft.table_like |= draft.table_like;
            }
        }
    }
    atomic_write(&path, &serde_json::to_vec_pretty(&registry)?)?;
    Ok(merged.into_values().collect())
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
