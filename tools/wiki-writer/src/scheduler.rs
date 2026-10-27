use crate::{
    agent::{Agent, CallContext, CallError, FailureKind, Response},
    config::Models,
    types::{Provider, Stage, Tier},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerEntry {
    pub run: String,
    pub stage: Stage,
    pub item: String,
    pub model: String,
    pub attempt: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Option<f64>,
    pub outcome: String,
    pub duration_ms: u64,
    pub estimated_usage: bool,
}

pub fn ladder(start: Tier, ceiling: Tier) -> Vec<Tier> {
    match (start.min(ceiling), ceiling) {
        (Tier::Small, Tier::Small) => vec![Tier::Small, Tier::Small],
        (Tier::Small, Tier::Mid) => vec![Tier::Small, Tier::Small, Tier::Mid],
        (Tier::Small, Tier::Large) => vec![Tier::Small, Tier::Small, Tier::Mid, Tier::Large],
        (Tier::Mid, Tier::Mid) => vec![Tier::Mid],
        (Tier::Mid, Tier::Large) => vec![Tier::Mid, Tier::Large],
        (Tier::Large, Tier::Large) => vec![Tier::Large],
        _ => unreachable!(),
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub stage: Stage,
    pub item: String,
    pub start: Tier,
    pub prompt: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum JobOutcome {
    Done(String),
    Pending,
    Blocked(String),
}

#[derive(Debug, Clone)]
pub struct ScheduleReport {
    pub outcomes: BTreeMap<(Stage, String), JobOutcome>,
    pub charged_tokens: u64,
    pub rate_limits: u32,
    pub final_capacity: usize,
}

#[derive(Debug, Clone)]
pub struct ScheduleOptions {
    pub provider: Provider,
    pub fake_root: Option<PathBuf>,
    pub models: Models,
    pub ceiling: Tier,
    pub budget_tokens: u64,
    pub jobs: usize,
    pub timeout: Duration,
    pub run: String,
}

#[derive(Debug)]
struct Work {
    job: Job,
    slots: Vec<Tier>,
    slot: usize,
    rate_retries: u8,
    process_retries: u8,
    errors: Vec<String>,
    attempt: u32,
}

pub fn run_jobs(
    project: &Path,
    jobs: Vec<Job>,
    options: &ScheduleOptions,
    validate: &(impl Fn(&Job, &str) -> std::result::Result<(), String> + Sync),
    sleep: &(impl Fn(Duration) + Sync),
) -> Result<ScheduleReport> {
    ensure!((1..=64).contains(&options.jobs), "jobs must be 1..64");
    let ledger_path = project.join("state/ledger.jsonl");
    let previous = read_ledger(&ledger_path)?;
    let mut next_attempt = BTreeMap::<(Stage, String), u32>::new();
    for entry in previous {
        let key = (entry.stage, entry.item);
        next_attempt
            .entry(key)
            .and_modify(|n| *n = (*n).max(entry.attempt))
            .or_insert(entry.attempt);
    }
    let mut ready = VecDeque::new();
    for job in jobs {
        let attempt = *next_attempt
            .get(&(job.stage, job.item.clone()))
            .unwrap_or(&0);
        ready.push_back(Work {
            slots: ladder(job.start, options.ceiling),
            job,
            slot: 0,
            rate_retries: 0,
            process_retries: 0,
            errors: vec![],
            attempt,
        });
    }
    let mut report = ScheduleReport {
        outcomes: BTreeMap::new(),
        charged_tokens: 0,
        rate_limits: 0,
        final_capacity: options.jobs,
    };
    let mut capacity = options.jobs;
    let mut active = 0usize;
    let mut stop_rate = false;
    let mut delayed: Vec<(Instant, Work)> = Vec::new();
    std::thread::scope(|scope| -> Result<()> {
        let (tx, rx) =
            mpsc::channel::<(Work, std::result::Result<Response, CallError>, u64, String)>();
        while !ready.is_empty() || active > 0 || !delayed.is_empty() {
            let now = Instant::now();
            let mut index = 0;
            while index < delayed.len() {
                if delayed[index].0 <= now {
                    ready.push_back(delayed.swap_remove(index).1);
                } else {
                    index += 1;
                }
            }
            while !stop_rate
                && active < capacity
                && !ready.is_empty()
                && report.charged_tokens < options.budget_tokens
            {
                let mut work = ready.pop_front().unwrap();
                work.attempt += 1;
                let tier = work.slots[work.slot];
                let model = match tier {
                    Tier::Small => &options.models.small,
                    Tier::Mid => &options.models.mid,
                    Tier::Large => &options.models.large,
                }
                .clone();
                let prompt = if work.errors.is_empty() {
                    work.job.prompt.clone()
                } else {
                    format!(
                        "{}\n\nValidator errors to fix:\n{}",
                        work.job.prompt,
                        work.errors.join("\n")
                    )
                };
                let agent = if options.provider == Provider::Fake {
                    Agent::fake(
                        options.fake_root.clone().context("Fake root missing")?,
                        CallContext {
                            stage: work.job.stage,
                            item: work.job.item.clone(),
                            attempt: work.attempt,
                        },
                    )
                } else {
                    Agent::new(
                        options.provider,
                        options.timeout,
                        CallContext {
                            stage: work.job.stage,
                            item: work.job.item.clone(),
                            attempt: work.attempt,
                        },
                    )
                };
                let tx = tx.clone();
                scope.spawn(move || {
                    let started = Instant::now();
                    let result = agent.ask(&model, &prompt);
                    let _ = tx.send((work, result, started.elapsed().as_millis() as u64, prompt));
                });
                active += 1;
            }
            if active == 0 {
                if stop_rate || report.charged_tokens >= options.budget_tokens {
                    break;
                }
                if let Some((index, deadline)) = delayed
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, (deadline, _))| *deadline)
                    .map(|(index, (deadline, _))| (index, *deadline))
                {
                    sleep(deadline.saturating_duration_since(Instant::now()));
                    ready.push_back(delayed.swap_remove(index).1);
                    continue;
                }
                break;
            }
            let received =
                if let Some(deadline) = delayed.iter().map(|(deadline, _)| *deadline).min() {
                    match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                        Ok(value) => value,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(error) => return Err(error.into()),
                    }
                } else {
                    rx.recv()?
                };
            let (mut work, result, duration_ms, prompt) = received;
            active -= 1;
            let tier = work.slots[work.slot];
            let model = match tier {
                Tier::Small => &options.models.small,
                Tier::Mid => &options.models.mid,
                Tier::Large => &options.models.large,
            }
            .clone();
            let key = (work.job.stage, work.job.item.clone());
            let mut entry = LedgerEntry {
                run: options.run.clone(),
                stage: work.job.stage,
                item: work.job.item.clone(),
                model,
                attempt: work.attempt,
                input_tokens: 0,
                output_tokens: 0,
                cost_usd: None,
                outcome: String::new(),
                duration_ms,
                estimated_usage: false,
            };
            let mut retry = false;
            let mut retry_after = None;
            match result {
                Ok(response) => {
                    charge(
                        &mut entry,
                        response.input_tokens,
                        response.output_tokens,
                        response.usage_reported,
                        &prompt,
                        &response.text,
                    );
                    entry.cost_usd = response.cost_usd;
                    match validate(&work.job, &response.text) {
                        Ok(()) => {
                            entry.outcome = "ok".into();
                            report.outcomes.insert(key, JobOutcome::Done(response.text));
                        }
                        Err(reason) => {
                            eprintln!(
                                "warning: {:?}/{} attempt {} validation: {}",
                                work.job.stage,
                                work.job.item,
                                work.attempt,
                                reason.chars().take(500).collect::<String>()
                            );
                            entry.outcome = "bad_output".into();
                            work.errors.push(reason.clone());
                            if work.slot + 1 < work.slots.len() {
                                work.slot += 1;
                                retry = true;
                            } else {
                                report.outcomes.insert(key, JobOutcome::Blocked(reason));
                            }
                        }
                    }
                }
                Err(error) => {
                    charge(
                        &mut entry,
                        error.input_tokens,
                        error.output_tokens,
                        error.usage_reported,
                        &prompt,
                        &error.message,
                    );
                    entry.cost_usd = error.cost_usd;
                    match error.kind {
                        FailureKind::RateLimit => {
                            entry.outcome = "rate_limit".into();
                            report.rate_limits += 1;
                            if report.rate_limits.is_multiple_of(2) {
                                capacity = (capacity / 2).max(1);
                            }
                            if work.rate_retries < 6 {
                                let delay = 1u64 << work.rate_retries;
                                work.rate_retries += 1;
                                retry_after = Some(Duration::from_secs(delay));
                                retry = true;
                            } else {
                                stop_rate = true;
                                report.outcomes.insert(key, JobOutcome::Pending);
                            }
                        }
                        FailureKind::BadOutput => {
                            entry.outcome = "bad_output".into();
                            work.errors.push(error.message.clone());
                            if work.slot + 1 < work.slots.len() {
                                work.slot += 1;
                                retry = true;
                            } else {
                                report
                                    .outcomes
                                    .insert(key, JobOutcome::Blocked(error.message));
                            }
                        }
                        FailureKind::Other => {
                            entry.outcome = if error.message.starts_with("timeout") {
                                "timeout"
                            } else {
                                "other_error"
                            }
                            .into();
                            if work.process_retries == 0 {
                                work.process_retries = 1;
                                retry = true;
                            } else if work.slot + 1 < work.slots.len() {
                                let old = work.slots[work.slot];
                                work.slot += 1;
                                while work.slot < work.slots.len() && work.slots[work.slot] == old {
                                    work.slot += 1;
                                }
                                if work.slot < work.slots.len() {
                                    work.process_retries = 0;
                                    retry = true;
                                } else {
                                    report
                                        .outcomes
                                        .insert(key, JobOutcome::Blocked(error.message));
                                }
                            } else {
                                report
                                    .outcomes
                                    .insert(key, JobOutcome::Blocked(error.message));
                            }
                        }
                    }
                }
            }
            report.charged_tokens = report
                .charged_tokens
                .saturating_add(entry.input_tokens + entry.output_tokens);
            append_ledger(&ledger_path, &entry)?;
            if retry {
                if let Some(delay) = retry_after {
                    delayed.push((Instant::now() + delay, work));
                } else {
                    // Finish a retried item before starting more fresh jobs so
                    // token-limited runs can persist completed items.
                    ready.push_front(work);
                }
            }
        }
        Ok(())
    })?;
    for work in ready {
        report
            .outcomes
            .entry((work.job.stage, work.job.item))
            .or_insert(JobOutcome::Pending);
    }
    for (_, work) in delayed {
        report
            .outcomes
            .entry((work.job.stage, work.job.item))
            .or_insert(JobOutcome::Pending);
    }
    report.final_capacity = capacity;
    Ok(report)
}

fn charge(
    entry: &mut LedgerEntry,
    input: u64,
    output: u64,
    reported: bool,
    prompt: &str,
    response: &str,
) {
    entry.estimated_usage = !reported;
    entry.input_tokens = if reported {
        input
    } else {
        prompt.chars().count().div_ceil(4) as u64
    };
    entry.output_tokens = if reported {
        output
    } else {
        response.chars().count().div_ceil(4) as u64
    };
}

fn read_ledger(path: &Path) -> Result<Vec<LedgerEntry>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.into()),
    };
    let complete = bytes
        .iter()
        .rposition(|b| *b == b'\n')
        .map(|i| i + 1)
        .unwrap_or(0);
    if complete < bytes.len() {
        fs::write(path, &bytes[..complete])?;
    }
    bytes[..complete]
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).context("invalid ledger line"))
        .collect()
}

fn append_ledger(path: &Path, entry: &LedgerEntry) -> Result<()> {
    fs::create_dir_all(path.parent().context("ledger parent missing")?)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, entry)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
