use crate::types::{Provider, Stage};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Write},
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const LIMIT: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub text: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Option<f64>,
    pub usage_reported: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    RateLimit,
    BadOutput,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallError {
    pub kind: FailureKind,
    pub message: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Option<f64>,
    pub usage_reported: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallContext {
    pub stage: Stage,
    pub item: String,
    pub attempt: u32,
}

#[derive(Debug, Clone)]
pub struct Agent {
    provider: Provider,
    timeout: Duration,
    context: CallContext,
    fake_root: Option<PathBuf>,
}

impl Agent {
    pub fn new(provider: Provider, timeout: Duration, context: CallContext) -> Self {
        Self {
            provider,
            timeout,
            context,
            fake_root: None,
        }
    }

    pub fn fake(root: PathBuf, context: CallContext) -> Self {
        Self {
            provider: Provider::Fake,
            timeout: Duration::ZERO,
            context,
            fake_root: Some(root),
        }
    }

    pub fn ask(&self, model: &str, prompt: &str) -> Result<Response, CallError> {
        if self.provider == Provider::Fake {
            return self.fake_ask(model);
        }
        self.process_ask(model, prompt)
    }

    fn fake_ask(&self, model: &str) -> Result<Response, CallError> {
        let stage = serde_json::to_value(self.context.stage)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        let path = self
            .fake_root
            .as_ref()
            .unwrap()
            .join(stage)
            .join(&self.context.item)
            .join(model)
            .join(format!("{}.json", self.context.attempt));
        let bytes = std::fs::read(&path).map_err(|e| {
            err(
                FailureKind::Other,
                format!("missing Fake fixture {}: {e}", path.display()),
            )
        })?;
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fixture {
            response: Option<Response>,
            error: Option<CallError>,
        }
        let fixture: Fixture = serde_json::from_slice(&bytes).map_err(|e| {
            err(
                FailureKind::BadOutput,
                format!("invalid Fake fixture {}: {e}", path.display()),
            )
        })?;
        match (fixture.response, fixture.error) {
            (Some(response), None) => Ok(response),
            (None, Some(error)) => Err(error),
            _ => Err(err(
                FailureKind::BadOutput,
                format!(
                    "Fake fixture {} must contain one response or error",
                    path.display()
                ),
            )),
        }
    }

    fn process_ask(&self, model: &str, prompt: &str) -> Result<Response, CallError> {
        let dir = tempfile::tempdir().map_err(|e| err(FailureKind::Other, e.to_string()))?;
        let mut cmd = match self.provider {
            Provider::Claude => Command::new("claude"),
            Provider::Codex => Command::new("codex"),
            Provider::Fake => unreachable!(),
        };
        match self.provider {
            Provider::Claude => {
                cmd.args([
                    "-p",
                    "--output-format",
                    "stream-json",
                    "--verbose",
                    "--model",
                    model,
                    "--tools",
                    "",
                    "--strict-mcp-config",
                    "--mcp-config",
                    "{\"mcpServers\":{}}",
                    "--safe-mode",
                    "--disable-slash-commands",
                    "--permission-prompts",
                    "none",
                    "--no-session-persistence",
                ]);
            }
            Provider::Codex => {
                cmd.args([
                    "exec",
                    "--json",
                    "-m",
                    model,
                    "--sandbox",
                    "read-only",
                    "--ephemeral",
                    "--ignore-user-config",
                    "--ignore-rules",
                    "--skip-git-repo-check",
                    "-c",
                    "approval_policy=\"never\"",
                    "-c",
                    "web_search=\"disabled\"",
                ]);
                for feature in [
                    "shell_tool",
                    "unified_exec",
                    "apps",
                    "plugins",
                    "multi_agent",
                    "browser_use",
                    "browser_use_external",
                    "computer_use",
                    "in_app_browser",
                    "code_mode",
                    "code_mode_host",
                    "image_generation",
                    "view_image",
                    "hooks",
                    "goals",
                    "sleep_tool",
                    "skill_search",
                ] {
                    cmd.args(["--disable", feature]);
                }
                cmd.args(["-c", "project_doc_max_bytes=0", "-"]);
            }
            Provider::Fake => unreachable!(),
        }
        cmd.current_dir(dir.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        unsafe {
            cmd.pre_exec(|| {
                if libc::setpgid(0, 0) == -1 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(())
                }
            });
        }
        let mut child = cmd.spawn().map_err(|e| {
            err(
                FailureKind::Other,
                format!("{} spawn failed: {e}", provider_name(self.provider)),
            )
        })?;
        let pid = child.id() as i32;
        let mut stdin = child.stdin.take().unwrap();
        let input = prompt.as_bytes().to_vec();
        let writer = thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
        let (tx, rx) = mpsc::channel();
        let stdout = child.stdout.take().unwrap();
        let tx_out = tx.clone();
        let out_thread = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut total = 0usize;
            loop {
                let mut bytes = Vec::new();
                match reader.read_until(b'\n', &mut bytes) {
                    Ok(0) => break,
                    Ok(n) => {
                        total += n;
                        if total > LIMIT {
                            let _ = tx_out.send(StreamEvent::Overflow);
                            break;
                        }
                        let _ = tx_out.send(StreamEvent::Line(bytes));
                    }
                    Err(e) => {
                        let _ = tx_out.send(StreamEvent::ReadError(e.to_string()));
                        break;
                    }
                }
            }
            let _ = tx_out.send(StreamEvent::End);
        });
        drop(tx);
        let stderr = child.stderr.take().unwrap();
        let err_thread = thread::spawn(move || {
            let mut bytes = Vec::new();
            let result = stderr.take((LIMIT + 1) as u64).read_to_end(&mut bytes);
            (bytes, result)
        });
        let started = Instant::now();
        let mut parsed = Parsed::default();
        let mut failure = None;
        let status;
        loop {
            while let Ok(event) = rx.try_recv() {
                consume(event, self.provider, &mut parsed, &mut failure);
            }
            if failure.is_some() {
                kill_group(pid);
                status = child
                    .wait()
                    .map_err(|e| err(FailureKind::Other, e.to_string()))?;
                break;
            }
            if let Some(done) = child
                .try_wait()
                .map_err(|e| err(FailureKind::Other, e.to_string()))?
            {
                status = done;
                break;
            }
            if started.elapsed() >= self.timeout {
                if let Some(done) = child
                    .try_wait()
                    .map_err(|e| err(FailureKind::Other, e.to_string()))?
                {
                    status = done;
                } else {
                    kill_group(pid);
                    status = child
                        .wait()
                        .map_err(|e| err(FailureKind::Other, e.to_string()))?;
                    failure = Some(err(
                        FailureKind::Other,
                        format!("timeout after {} seconds", self.timeout.as_secs()),
                    ));
                }
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        // A helper may outlive the CLI while still holding our pipe descriptors.
        // Reap the isolated process group before joining the reader threads.
        kill_group(pid);
        let _ = writer.join();
        let _ = out_thread.join();
        while let Ok(event) = rx.try_recv() {
            consume(event, self.provider, &mut parsed, &mut failure);
        }
        let (stderr_bytes, stderr_read) = err_thread
            .join()
            .map_err(|_| err(FailureKind::Other, "stderr reader panicked".into()))?;
        if stderr_bytes.len() > LIMIT {
            failure = Some(err(FailureKind::BadOutput, "stderr exceeds 8 MiB".into()));
        }
        if stderr_read.is_err() {
            failure = Some(err(FailureKind::Other, "stderr read failed".into()));
        }
        if let Some(mut e) = failure {
            parsed.apply_usage(&mut e);
            return Err(e);
        }
        if !status.success() {
            let stderr = String::from_utf8_lossy(&stderr_bytes);
            let message = format!(
                "{} exited with {}: {}",
                provider_name(self.provider),
                status,
                stderr.chars().take(4096).collect::<String>()
            );
            let mut e = err(classify(&message), message);
            parsed.apply_usage(&mut e);
            return Err(e);
        }
        parsed.finish(self.provider, model)
    }
}

enum StreamEvent {
    Line(Vec<u8>),
    Overflow,
    ReadError(String),
    End,
}
fn consume(
    event: StreamEvent,
    provider: Provider,
    parsed: &mut Parsed,
    failure: &mut Option<CallError>,
) {
    match event {
        StreamEvent::Line(bytes) if failure.is_none() => {
            if let Err(e) = parsed.feed(provider, &bytes) {
                *failure = Some(e);
            }
        }
        StreamEvent::Overflow => {
            *failure = Some(err(FailureKind::BadOutput, "stdout exceeds 8 MiB".into()))
        }
        StreamEvent::ReadError(e) => {
            *failure = Some(err(FailureKind::Other, format!("stdout read failed: {e}")))
        }
        _ => {}
    }
}

#[derive(Default)]
struct Parsed {
    text: String,
    messages: BTreeMap<String, String>,
    final_seen: bool,
    input: u64,
    output: u64,
    cost: Option<f64>,
    usage_reported: bool,
}

impl Parsed {
    fn feed(&mut self, provider: Provider, line: &[u8]) -> Result<(), CallError> {
        if line.iter().all(u8::is_ascii_whitespace) {
            return Ok(());
        }
        let value: Value = serde_json::from_slice(line)
            .map_err(|e| err(FailureKind::BadOutput, format!("invalid JSONL: {e}")))?;
        let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
        let item_kind = value
            .get("item")
            .and_then(|v| v.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let content_kinds = value
            .pointer("/message/content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|part| part.get("type").and_then(Value::as_str));
        if [kind, item_kind].into_iter().chain(content_kinds).any(|k| {
            k.contains("tool_")
                || k.contains("command_execution")
                || k.contains("web_search")
                || k.contains("file_search")
                || k.contains("function_call")
                || k.contains("browser")
                || k.contains("computer")
        }) {
            return Err(err(FailureKind::BadOutput, "tool event rejected".into()));
        }
        match provider {
            Provider::Claude if kind == "result" => {
                if self.final_seen {
                    return Err(err(
                        FailureKind::BadOutput,
                        "duplicate Claude result".into(),
                    ));
                }
                self.final_seen = true;
                self.text = value
                    .get("result")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .into();
                if let Some(usage) = value.get("usage") {
                    self.input = usage
                        .get("input_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        + usage
                            .get("cache_read_input_tokens")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                        + usage
                            .get("cache_creation_input_tokens")
                            .and_then(Value::as_u64)
                            .unwrap_or(0);
                    self.output = usage
                        .get("output_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    self.usage_reported = true;
                }
                self.cost = value.get("total_cost_usd").and_then(Value::as_f64);
                if value.get("is_error").and_then(Value::as_bool) == Some(true) {
                    let mut e = err(classify(&self.text), self.text.clone());
                    self.apply_usage(&mut e);
                    return Err(e);
                }
            }
            Provider::Codex if kind == "item.completed" => {
                if self.final_seen {
                    return Err(err(
                        FailureKind::BadOutput,
                        "message after Codex completion".into(),
                    ));
                }
                let item = value
                    .get("item")
                    .ok_or_else(|| err(FailureKind::BadOutput, "missing item".into()))?;
                if item.get("type").and_then(Value::as_str) == Some("agent_message") {
                    let id = item
                        .get("id")
                        .and_then(Value::as_str)
                        .ok_or_else(|| err(FailureKind::BadOutput, "missing item ID".into()))?;
                    let text = item.get("text").and_then(Value::as_str).ok_or_else(|| {
                        err(FailureKind::BadOutput, "missing message text".into())
                    })?;
                    if let Some(old) = self.messages.get(id) {
                        if old != text {
                            return Err(err(
                                FailureKind::BadOutput,
                                "conflicting duplicate message".into(),
                            ));
                        }
                    } else {
                        self.messages.insert(id.into(), text.into());
                        self.text.clear();
                        self.text.push_str(text);
                    }
                }
            }
            Provider::Codex if kind == "turn.completed" => {
                if self.final_seen {
                    return Err(err(
                        FailureKind::BadOutput,
                        "duplicate Codex completion".into(),
                    ));
                }
                self.final_seen = true;
                if let Some(usage) = value.get("usage") {
                    self.input = usage
                        .get("input_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    self.output = usage
                        .get("output_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    self.usage_reported = true;
                }
            }
            Provider::Codex if kind == "turn.failed" || kind == "error" => {
                let msg = value
                    .get("message")
                    .and_then(Value::as_str)
                    .or_else(|| {
                        value
                            .get("error")
                            .and_then(|e| e.get("message"))
                            .and_then(Value::as_str)
                    })
                    .unwrap_or("Codex error");
                let code = value
                    .get("code")
                    .and_then(Value::as_str)
                    .or_else(|| {
                        value
                            .get("error")
                            .and_then(|e| e.get("code"))
                            .and_then(Value::as_str)
                    })
                    .unwrap_or("");
                let mut e = err(classify(&format!("{code} {msg}")), msg.into());
                self.apply_usage(&mut e);
                return Err(e);
            }
            _ => {}
        }
        Ok(())
    }

    fn apply_usage(&self, e: &mut CallError) {
        e.input_tokens = self.input;
        e.output_tokens = self.output;
        e.cost_usd = self.cost;
        e.usage_reported = self.usage_reported;
    }

    fn finish(self, provider: Provider, model: &str) -> Result<Response, CallError> {
        if !self.final_seen || self.text.is_empty() {
            let mut e = err(
                FailureKind::BadOutput,
                "missing final text or completion".into(),
            );
            self.apply_usage(&mut e);
            return Err(e);
        }
        Ok(Response {
            text: self.text,
            model: model.into(),
            input_tokens: self.input,
            output_tokens: self.output,
            cost_usd: if provider == Provider::Codex {
                None
            } else {
                self.cost
            },
            usage_reported: self.usage_reported,
        })
    }
}

fn provider_name(provider: Provider) -> &'static str {
    match provider {
        Provider::Claude => "claude",
        Provider::Codex => "codex",
        Provider::Fake => "fake",
    }
}
fn kill_group(pid: i32) {
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
    }
}
fn classify(message: &str) -> FailureKind {
    let lower = message.to_ascii_lowercase();
    if ["rate_limit", "rate limit", "429", "overloaded", "overload"]
        .iter()
        .any(|word| lower.contains(word))
    {
        FailureKind::RateLimit
    } else {
        FailureKind::Other
    }
}
fn err(kind: FailureKind, message: String) -> CallError {
    CallError {
        kind,
        message,
        input_tokens: 0,
        output_tokens: 0,
        cost_usd: None,
        usage_reported: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn claude_cache_usage_and_duplicate_final() {
        let mut parsed = Parsed::default();
        let line = br#"{"type":"result","result":"hello","usage":{"input_tokens":2,"cache_read_input_tokens":3,"cache_creation_input_tokens":4,"output_tokens":5},"total_cost_usd":0.1}"#;
        parsed.feed(Provider::Claude, line).unwrap();
        assert_eq!(parsed.input, 9);
        assert_eq!(parsed.output, 5);
        assert_eq!(parsed.cost, Some(0.1));
        assert_eq!(
            parsed.feed(Provider::Claude, line).unwrap_err().kind,
            FailureKind::BadOutput
        );
    }
    #[test]
    fn codex_dedupes_messages_and_requires_completion() {
        let mut parsed = Parsed::default();
        let line =
            br#"{"type":"item.completed","item":{"type":"agent_message","id":"a","text":"hello"}}"#;
        parsed.feed(Provider::Codex, line).unwrap();
        parsed.feed(Provider::Codex, line).unwrap();
        assert_eq!(parsed.text, "hello");
        assert!(parsed.finish(Provider::Codex, "m").is_err());
    }
    #[test]
    fn codex_uses_last_completed_agent_message() {
        let mut parsed = Parsed::default();
        parsed.feed(Provider::Codex, br#"{"type":"item.completed","item":{"type":"agent_message","id":"a","text":"Here is the JSON:"}}"#).unwrap();
        parsed.feed(Provider::Codex, br#"{"type":"item.completed","item":{"type":"agent_message","id":"b","text":"{\"ok\":true}"}}"#).unwrap();
        parsed
            .feed(
                Provider::Codex,
                br#"{"type":"turn.completed","usage":{"input_tokens":1,"output_tokens":1}}"#,
            )
            .unwrap();
        assert_eq!(
            parsed.finish(Provider::Codex, "m").unwrap().text,
            r#"{"ok":true}"#
        );
    }
}
