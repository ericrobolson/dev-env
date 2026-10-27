use crate::types::{Provider, Source};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Models {
    pub small: String,
    pub mid: String,
    pub large: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub project: String,
    pub agent: Provider,
    pub models: Models,
    pub profile: String,
    pub sources: Vec<Source>,
    pub jobs: usize,
    pub audit_sample_rate: f64,
    pub audit_failure_threshold: f64,
    pub timeout_seconds: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawModels {
    small: Option<String>,
    mid: Option<String>,
    large: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    version: u32,
    project: String,
    agent: Provider,
    sources: Vec<Source>,
    models: Option<RawModels>,
    profile: Option<String>,
    jobs: Option<usize>,
    audit_sample_rate: Option<f64>,
    audit_failure_threshold: Option<f64>,
    timeout_seconds: Option<u64>,
}

pub fn default_models(provider: Provider) -> Models {
    match provider {
        Provider::Claude => Models {
            small: "claude-haiku-5-5".into(),
            mid: "claude-sonnet-5-5".into(),
            large: "claude-opus-5-5".into(),
        },
        Provider::Codex | Provider::Fake => Models {
            small: "gpt-6-luna".into(),
            mid: "gpt-6.1-sol".into(),
            large: "gpt-6-astra".into(),
        },
    }
}

pub fn load(project: &Path) -> Result<Config> {
    let path = project.join("wikify.yaml");
    ensure!(
        !fs::symlink_metadata(&path)
            .context("config missing")?
            .file_type()
            .is_symlink(),
        "config symlink refused"
    );
    let raw: RawConfig = serde_yaml::from_slice(&fs::read(&path)?)?;
    ensure!(raw.version == 1, "unsupported config version");
    ensure!(raw.agent != Provider::Fake, "fake provider is test-only");
    ensure!(valid_slug(&raw.project), "invalid project name");
    let profile = raw.profile.unwrap_or_else(|| "generic".into());
    ensure!(
        profile == "generic" || profile == "ttrpg",
        "invalid profile"
    );
    let jobs = raw.jobs.unwrap_or(4);
    ensure!((1..=64).contains(&jobs), "jobs must be 1..64");
    let audit_sample_rate = raw.audit_sample_rate.unwrap_or(0.10);
    let audit_failure_threshold = raw.audit_failure_threshold.unwrap_or(0.20);
    ensure!(
        audit_sample_rate.is_finite() && (0.0..=1.0).contains(&audit_sample_rate),
        "invalid audit sample rate"
    );
    ensure!(
        audit_failure_threshold.is_finite() && (0.0..=1.0).contains(&audit_failure_threshold),
        "invalid audit failure threshold"
    );
    let timeout_seconds = raw.timeout_seconds.unwrap_or(300);
    ensure!(timeout_seconds > 0, "timeout must be positive");
    let mut models = default_models(raw.agent);
    if let Some(custom) = raw.models {
        if let Some(value) = custom.small {
            models.small = value;
        }
        if let Some(value) = custom.mid {
            models.mid = value;
        }
        if let Some(value) = custom.large {
            models.large = value;
        }
    }
    ensure!(
        [&models.small, &models.mid, &models.large]
            .iter()
            .all(|s| !s.trim().is_empty()),
        "empty model name"
    );
    for source in &raw.sources {
        if source.id.is_empty() || source.name.is_empty() || source.sha256.len() != 64 {
            bail!("invalid source metadata");
        }
        let expected = format!("sources/{}.", source.id);
        ensure!(source.path.starts_with(&expected), "invalid source path");
        let ext = &source.path[expected.len()..];
        ensure!(
            ["pdf", "epub", "txt", "md"].contains(&ext),
            "unsupported source extension"
        );
    }
    Ok(Config {
        version: raw.version,
        project: raw.project,
        agent: raw.agent,
        models,
        profile,
        sources: raw.sources,
        jobs,
        audit_sample_rate,
        audit_failure_threshold,
        timeout_seconds,
    })
}

fn valid_slug(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}
