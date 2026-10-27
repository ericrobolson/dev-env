use crate::config;
use anyhow::{Result, bail};
use std::{env, path::Path, process::Command};

pub fn doctor(project: Option<&Path>) -> Result<()> {
    let mut missing = Vec::new();
    for executable in ["pdfinfo", "pdftotext", "php"] {
        if !available(executable) {
            missing.push(executable.to_owned());
        }
    }
    if let Some(project) = project {
        let config = config::load(project)?;
        let agent = match config.agent {
            crate::types::Provider::Claude => "claude",
            crate::types::Provider::Codex => "codex",
            crate::types::Provider::Fake => unreachable!(),
        };
        if !available(agent) {
            missing.push(agent.into());
        }
    } else if !available("claude") && !available("codex") {
        missing.push("claude or codex".into());
    }
    if !missing.is_empty() {
        bail!("missing dependencies: {}", missing.join(", "));
    }
    println!("Dependencies available.");
    Ok(())
}

fn available(executable: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    for directory in env::split_paths(&path) {
        let candidate = directory.join(executable);
        if candidate.is_file() {
            let output = if matches!(executable, "php" | "pdfinfo" | "pdftotext") {
                Command::new(&candidate).arg("-v").output()
            } else {
                Command::new(&candidate).arg("--version").output()
            };
            if output.is_ok_and(|result| result.status.success()) {
                return true;
            }
        }
    }
    false
}
