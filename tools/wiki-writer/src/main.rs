use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use wiki_writer::{
    pipeline::{self, RunOptions},
    project,
    state::atomic_write,
    status,
    types::{ItemRecord, ItemStatus, Provider, Stage, Tier},
};

#[derive(Parser)]
#[command(name = "wiki-writer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Init {
        project: String,
        #[arg(required = true)]
        sources: Vec<PathBuf>,
        #[arg(long)]
        agent: ProviderArg,
        #[arg(long, default_value = "generic")]
        profile: ProfileArg,
    },
    Run {
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(short = 'j', value_parser = parse_jobs)]
        jobs: Option<usize>,
        #[arg(long, conflicts_with = "only")]
        until: Option<StageArg>,
        #[arg(long, conflicts_with = "until")]
        only: Option<StageArg>,
        #[arg(long, default_value = "large")]
        ceiling: TierArg,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        budget_tokens: Option<u64>,
    },
    Status {
        #[arg(long)]
        project: Option<PathBuf>,
    },
    Redo {
        stage: StageArg,
        #[arg(long)]
        item: Option<String>,
        #[arg(long)]
        project: Option<PathBuf>,
    },
    Lint {
        #[arg(long)]
        project: Option<PathBuf>,
    },
    Render {
        #[arg(long)]
        project: Option<PathBuf>,
    },
    Doctor,
}

#[derive(Clone, Copy, ValueEnum)]
enum ProviderArg {
    Claude,
    Codex,
}
#[derive(Clone, Copy, ValueEnum)]
enum ProfileArg {
    Generic,
    Ttrpg,
}
#[derive(Clone, Copy, ValueEnum)]
enum TierArg {
    Small,
    Mid,
    Large,
}
#[derive(Clone, Copy, ValueEnum)]
enum StageArg {
    Ingest,
    Survey,
    Extract,
    Audit,
    Resolve,
    Organize,
    Write,
    Link,
    Review,
}

impl From<ProviderArg> for Provider {
    fn from(value: ProviderArg) -> Self {
        match value {
            ProviderArg::Claude => Self::Claude,
            ProviderArg::Codex => Self::Codex,
        }
    }
}
impl From<TierArg> for Tier {
    fn from(value: TierArg) -> Self {
        match value {
            TierArg::Small => Self::Small,
            TierArg::Mid => Self::Mid,
            TierArg::Large => Self::Large,
        }
    }
}
impl From<StageArg> for Stage {
    fn from(value: StageArg) -> Self {
        match value {
            StageArg::Ingest => Self::Ingest,
            StageArg::Survey => Self::Survey,
            StageArg::Extract => Self::Extract,
            StageArg::Audit => Self::Audit,
            StageArg::Resolve => Self::Resolve,
            StageArg::Organize => Self::Organize,
            StageArg::Write => Self::Write,
            StageArg::Link => Self::Link,
            StageArg::Review => Self::Review,
        }
    }
}

fn parse_jobs(value: &str) -> Result<usize, String> {
    let jobs: usize = value.parse().map_err(|_| "jobs must be 1..64".to_owned())?;
    if (1..=64).contains(&jobs) {
        Ok(jobs)
    } else {
        Err("jobs must be 1..64".into())
    }
}

fn main() {
    match execute(Cli::parse()) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("error: {error:#}");
            std::process::exit(1);
        }
    }
}

fn execute(cli: Cli) -> anyhow::Result<i32> {
    match cli.command {
        Command::Init {
            project: name,
            sources,
            agent,
            profile,
        } => {
            let profile = match profile {
                ProfileArg::Generic => "generic",
                ProfileArg::Ttrpg => "ttrpg",
            };
            let path = project::init(
                &std::env::current_dir()?,
                &name,
                &sources,
                agent.into(),
                profile,
            )?;
            println!("Initialized {}", path.display());
            Ok(0)
        }
        Command::Run {
            project,
            jobs,
            until,
            only,
            ceiling,
            yes,
            budget_tokens,
        } => {
            let project = project_path(project)?;
            let options = RunOptions {
                jobs,
                until: until.map(Into::into),
                only: only.map(Into::into),
                ceiling: ceiling.into(),
                yes,
                budget_tokens,
            };
            let report = pipeline::run(&project, &options)?;
            status::print(&project)?;
            if report.needs_acceptance {
                Ok(4)
            } else if report.pending > 0 || report.blocked > 0 {
                Ok(3)
            } else {
                Ok(0)
            }
        }
        Command::Status { project } => {
            status::print(&project_path(project)?)?;
            Ok(0)
        }
        Command::Redo {
            stage,
            item,
            project,
        } => {
            redo(&project_path(project)?, stage.into(), item.as_deref())?;
            Ok(0)
        }
        Command::Lint { project } => {
            let report = wiki_writer::link::lint(&project_path(project)?)?;
            println!(
                "broken links: {}, orphans: {}, missing citations: {}, gaps: {}",
                report.broken_links.len(),
                report.orphans.len(),
                report.missing_citations.len(),
                report.gaps.len()
            );
            Ok(
                if report.broken_links.is_empty() && report.missing_citations.is_empty() {
                    0
                } else {
                    1
                },
            )
        }
        Command::Render { project } => {
            let project = project_path(project)?;
            let _lock = wiki_writer::state::lock(&project)?;
            let report = wiki_writer::render::render(&project)?;
            println!(
                "written: {}, unchanged: {}, protected: {}",
                report.written,
                report.unchanged,
                report.protected.len()
            );
            Ok(0)
        }
        Command::Doctor => {
            let cwd = std::env::current_dir()?;
            let project = if cwd.join("wikify.yaml").exists() {
                Some(cwd.as_path())
            } else {
                None
            };
            wiki_writer::doctor::doctor(project)?;
            Ok(0)
        }
    }
}

fn project_path(path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let canonical = fs::canonicalize(path.unwrap_or(std::env::current_dir()?))?;
    anyhow::ensure!(
        canonical.join("wikify.yaml").is_file(),
        "project has no wikify.yaml: {}",
        canonical.display()
    );
    Ok(canonical)
}

fn redo(project: &Path, stage: Stage, item: Option<&str>) -> anyhow::Result<()> {
    let _lock = wiki_writer::state::lock(project)?;
    let directory = project.join("state/items").join(status::stage_name(stage));
    let mut paths = Vec::new();
    if let Some(item) = item {
        anyhow::ensure!(
            !item.contains("..") && !item.starts_with('/'),
            "invalid redo item"
        );
        let path = directory
            .join(item.replace(':', "/"))
            .with_extension("json");
        anyhow::ensure!(path.exists(), "unknown redo item {item}");
        paths.push(path);
    } else {
        let mut stack = vec![directory.clone()];
        while let Some(dir) = stack.pop() {
            if !dir.exists() {
                continue;
            }
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    stack.push(entry.path());
                } else if entry.path().extension().and_then(|ext| ext.to_str()) == Some("json") {
                    paths.push(entry.path());
                }
            }
        }
        anyhow::ensure!(
            !paths.is_empty(),
            "no redo items for {}",
            status::stage_name(stage)
        );
    }
    let forced_path = project.join("state/forced.json");
    let mut forced: BTreeSet<String> = match fs::read(&forced_path) {
        Ok(bytes) => serde_json::from_slice(&bytes).context("invalid state/forced.json")?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeSet::new(),
        Err(error) => return Err(error.into()),
    };
    for path in paths {
        let mut record: ItemRecord = serde_json::from_slice(&fs::read(&path)?)?;
        record.input_hash.clear();
        record.status = ItemStatus::Pending;
        atomic_write(&path, &serde_json::to_vec_pretty(&record)?)?;
        let item = path
            .strip_prefix(&directory)?
            .with_extension("")
            .to_string_lossy()
            .replace('/', ":");
        forced.insert(format!("{}/{}", status::stage_name(stage), item));
    }
    atomic_write(&forced_path, &serde_json::to_vec_pretty(&forced)?)?;
    Ok(())
}
