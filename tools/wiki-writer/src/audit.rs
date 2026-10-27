use crate::{
    types::{Chunk, FactDraft, Notes, Schema},
    validate::grounding,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replacement {
    pub id: String,
    pub fact: FactDraft,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Audit {
    pub remove: Vec<String>,
    pub replace: Vec<Replacement>,
    pub add: Vec<FactDraft>,
    pub reasons: Vec<String>,
}

pub fn sample(chunks: &[Chunk], rate: f64) -> Vec<String> {
    let count = ((rate * chunks.len() as f64).ceil() as usize).min(chunks.len());
    let mut keyed: Vec<_> = chunks
        .iter()
        .map(|chunk| {
            (
                Sha256::digest(chunk.id.as_bytes()).to_vec(),
                chunk.id.clone(),
            )
        })
        .collect();
    keyed.sort();
    let mut selected: Vec<_> = keyed.into_iter().take(count).map(|(_, id)| id).collect();
    selected.sort();
    selected
}

pub fn apply(
    notes: &Notes,
    audit: &Audit,
    chunk: &Chunk,
    schema: &Schema,
) -> Result<Vec<FactDraft>> {
    let known: BTreeSet<_> = notes.facts.iter().map(|fact| fact.id.as_str()).collect();
    let mut used = BTreeSet::new();
    for id in &audit.remove {
        ensure!(known.contains(id.as_str()), "unknown audit remove ID {id}");
        ensure!(
            used.insert(id.as_str()),
            "duplicate audit operation for {id}"
        );
    }
    for replacement in &audit.replace {
        ensure!(
            known.contains(replacement.id.as_str()),
            "unknown audit replacement ID {}",
            replacement.id
        );
        ensure!(
            used.insert(replacement.id.as_str()),
            "duplicate audit operation for {}",
            replacement.id
        );
    }
    let mut result: Vec<FactDraft> = notes
        .facts
        .iter()
        .filter(|fact| !used.contains(fact.id.as_str()))
        .map(|fact| fact.draft.clone())
        .collect();
    let mut replacements = audit.replace.clone();
    replacements.sort_by(|a, b| a.id.cmp(&b.id));
    result.extend(replacements.into_iter().map(|replacement| replacement.fact));
    result.extend(audit.add.clone());
    for (index, draft) in result.iter().enumerate() {
        let errors = grounding(draft, chunk, schema);
        ensure!(
            errors.is_empty(),
            "audit fact {index} ungrounded: {}",
            errors.join("; ")
        );
    }
    Ok(result)
}
