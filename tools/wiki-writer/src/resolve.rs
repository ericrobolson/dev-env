use crate::types::{Entity, Fact};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairVerdict {
    Yes,
    No,
    Unsure,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairDecision {
    pub verdict: PairVerdict,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterDecision {
    pub groups: Vec<Vec<String>>,
    pub evidence: Vec<String>,
}

pub fn group_exact(facts: &[Fact]) -> Vec<Entity> {
    let mut groups = BTreeMap::<(String, String), Vec<&Fact>>::new();
    for fact in facts {
        groups
            .entry((
                normalize(&fact.draft.entity_type),
                normalize(&fact.draft.entity),
            ))
            .or_default()
            .push(fact);
    }
    let mut entities = Vec::new();
    for ((entity_type, _), mut group) in groups {
        group.sort_by(|a, b| a.id.cmp(&b.id));
        let first = group[0];
        let mut aliases: Vec<String> = group
            .iter()
            .flat_map(|fact| fact.draft.aliases.clone())
            .collect();
        aliases.sort();
        aliases.dedup();
        let mut fact_ids: Vec<String> = group.iter().map(|fact| fact.id.clone()).collect();
        fact_ids.sort();
        fact_ids.dedup();
        entities.push(Entity {
            id: first.id.replacen('f', "e", 1),
            name: first.draft.entity.clone(),
            entity_type,
            aliases,
            fact_ids,
        });
    }
    entities.sort_by(|a, b| a.id.cmp(&b.id));
    entities
}

pub fn candidates(entities: &[Entity]) -> Vec<(String, String)> {
    let mut sorted = entities.to_vec();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));
    let mut pairs = Vec::new();
    for (index, left) in sorted.iter().enumerate() {
        for right in sorted.iter().skip(index + 1) {
            if left.entity_type != right.entity_type {
                continue;
            }
            let names_left = std::iter::once(&left.name).chain(left.aliases.iter());
            let names_right: Vec<_> = std::iter::once(&right.name)
                .chain(right.aliases.iter())
                .collect();
            let similarity = names_left
                .flat_map(|a| {
                    names_right
                        .iter()
                        .map(move |b| strsim::jaro_winkler(&normalize(a), &normalize(b)))
                })
                .fold(0.0f64, f64::max);
            if similarity >= 0.90 {
                pairs.push((left.id.clone(), right.id.clone()));
            }
        }
    }
    pairs
}

pub fn validate_pair(decision: &PairDecision, left: &Entity, right: &Entity) -> Vec<String> {
    let left_ids: BTreeSet<_> = left.fact_ids.iter().map(String::as_str).collect();
    let right_ids: BTreeSet<_> = right.fact_ids.iter().map(String::as_str).collect();
    let mut errors = Vec::new();
    for evidence in &decision.evidence {
        if !left_ids.contains(evidence.as_str()) && !right_ids.contains(evidence.as_str()) {
            errors.push(format!("unknown pair evidence {evidence}"));
        }
    }
    if decision.verdict != PairVerdict::Unsure {
        if !decision
            .evidence
            .iter()
            .any(|id| left_ids.contains(id.as_str()))
        {
            errors.push("pair evidence missing left facts".into());
        }
        if !decision
            .evidence
            .iter()
            .any(|id| right_ids.contains(id.as_str()))
        {
            errors.push("pair evidence missing right facts".into());
        }
    }
    errors
}

pub fn validate_cluster(
    decision: &ClusterDecision,
    members: &[Entity],
    no_pairs: &[(String, String)],
) -> Vec<String> {
    let known: BTreeMap<_, _> = members
        .iter()
        .map(|member| (member.id.as_str(), member))
        .collect();
    let mut seen = BTreeSet::new();
    let mut errors = Vec::new();
    for group in &decision.groups {
        if group.is_empty() {
            errors.push("empty cluster group".into());
        }
        for id in group {
            if !known.contains_key(id.as_str()) {
                errors.push(format!("unknown cluster entity {id}"));
            }
            if !seen.insert(id.as_str()) {
                errors.push(format!("duplicate cluster entity {id}"));
            }
        }
        if group.len() > 1 {
            for id in group {
                if let Some(member) = known.get(id.as_str())
                    && !decision
                        .evidence
                        .iter()
                        .any(|fact| member.fact_ids.contains(fact))
                {
                    errors.push(format!("cluster evidence missing entity {id}"));
                }
            }
            for (left, right) in no_pairs {
                if group.contains(left) && group.contains(right) {
                    errors.push(format!("cluster contradicts no pair {left}_{right}"));
                }
            }
        }
    }
    for id in known.keys() {
        if !seen.contains(id) {
            errors.push(format!("cluster omits entity {id}"));
        }
    }
    let facts: BTreeSet<_> = members
        .iter()
        .flat_map(|member| member.fact_ids.iter().map(String::as_str))
        .collect();
    for evidence in &decision.evidence {
        if !facts.contains(evidence.as_str()) {
            errors.push(format!("unknown cluster evidence {evidence}"));
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
