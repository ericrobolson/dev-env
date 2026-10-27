use crate::{
    config::{Config, default_models},
    state::{atomic_write, sha256},
    types::{Provider, Source},
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceManifest {
    pub sources: Vec<(String, String)>,
}

pub fn init(
    cwd: &Path,
    name: &str,
    sources: &[PathBuf],
    provider: Provider,
    profile: &str,
) -> Result<PathBuf> {
    ensure!(valid_name(name), "invalid project name: {name}");
    ensure!(
        matches!(provider, Provider::Claude | Provider::Codex),
        "Fake provider is test-only"
    );
    ensure!(
        matches!(profile, "generic" | "ttrpg"),
        "invalid profile: {profile}"
    );
    ensure!(!sources.is_empty(), "at least one source is required");
    let cwd = fs::canonicalize(cwd).context("invalid current directory")?;
    let mut resolved = Vec::new();
    let mut seen = BTreeSet::new();
    for input in sources {
        let absolute = if input.is_absolute() {
            input.clone()
        } else {
            cwd.join(input)
        };
        let canonical = fs::canonicalize(&absolute)
            .with_context(|| format!("source {} is unavailable", input.display()))?;
        ensure!(
            seen.insert(canonical.clone()),
            "duplicate source: {}",
            input.display()
        );
        let metadata = fs::metadata(&canonical)?;
        ensure!(
            metadata.is_file() && metadata.len() > 0,
            "source is not a nonempty regular file: {}",
            input.display()
        );
        let extension = canonical
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        ensure!(
            ["pdf", "epub", "txt", "md"].contains(&extension.as_str()),
            "unsupported source extension: {}",
            input.display()
        );
        let name = input
            .file_name()
            .and_then(|s| s.to_str())
            .context("source name is not UTF-8")?
            .to_owned();
        // Read now so an unreadable source fails before any project staging work.
        let bytes = fs::read(&canonical)?;
        resolved.push((canonical, name, extension, bytes));
    }
    let wikis = cwd.join("wikis");
    if let Ok(meta) = fs::symlink_metadata(&wikis) {
        ensure!(
            meta.is_dir() && !meta.file_type().is_symlink(),
            "wikis path must be a directory"
        );
    }
    fs::create_dir_all(&wikis)?;
    let destination = wikis.join(name);
    ensure!(
        !destination.exists() && fs::symlink_metadata(&destination).is_err(),
        "project already exists: {}",
        destination.display()
    );
    let staging = tempfile::Builder::new()
        .prefix(".wiki-writer-")
        .tempdir_in(&wikis)?;
    for directory in ["sources", "content", "state"] {
        fs::create_dir(staging.path().join(directory))?;
    }
    let mut entries = Vec::new();
    for (index, (_path, original_name, ext, bytes)) in resolved.into_iter().enumerate() {
        let id = format!("s{:04}", index + 1);
        let path = format!("sources/{id}.{ext}");
        atomic_write(&staging.path().join(&path), &bytes)?;
        let digest = sha256(&bytes);
        entries.push(Source {
            id,
            name: original_name,
            path,
            sha256: digest,
        });
    }
    let config = Config {
        version: 1,
        project: name.into(),
        agent: provider,
        models: default_models(provider),
        profile: profile.into(),
        sources: entries.clone(),
        jobs: 4,
        audit_sample_rate: 0.10,
        audit_failure_threshold: 0.20,
        timeout_seconds: 300,
    };
    atomic_write(
        &staging.path().join("wikify.yaml"),
        serde_yaml::to_string(&config)?.as_bytes(),
    )?;
    let manifest = SourceManifest {
        sources: entries
            .iter()
            .map(|s| (s.id.clone(), s.path.clone()))
            .collect(),
    };
    atomic_write(
        &staging.path().join("state/project.json"),
        &serde_json::to_vec_pretty(&manifest)?,
    )?;
    atomic_write(
        &staging.path().join("Makefile"),
        include_str!("../templates/Makefile").as_bytes(),
    )?;
    atomic_write(
        &staging.path().join("state/install.sh"),
        include_str!("../templates/install.sh")
            .replace("@@PROJECT@@", name)
            .as_bytes(),
    )?;
    atomic_write(
        &staging.path().join(".gitignore"),
        b"/sources/\n/dokuwiki/\n/state/ingest/\n",
    )?;
    let keep = staging.keep();
    if let Err(e) = fs::rename(&keep, &destination) {
        let _ = fs::remove_dir_all(&keep);
        bail!("failed to publish project: {e}");
    }
    Ok(destination)
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

pub(crate) fn verify_manifest(project: &Path, config: &Config) -> Result<()> {
    let manifest: SourceManifest =
        serde_json::from_slice(&fs::read(project.join("state/project.json"))?)?;
    let current: Vec<_> = config
        .sources
        .iter()
        .map(|source| (source.id.clone(), source.path.clone()))
        .collect();
    ensure!(
        manifest.sources == current,
        "source manifest changed; adding, removing or reordering sources is unsupported"
    );
    ensure!(
        config.project == project.file_name().and_then(|s| s.to_str()).unwrap_or(""),
        "config project name mismatch"
    );
    Ok(())
}
