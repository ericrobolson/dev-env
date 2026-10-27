use anyhow::{Result, ensure};

pub fn template(name: &str) -> Result<(&'static str, u32)> {
    let body = match name {
        "survey" => include_str!("../prompts/survey.md"),
        "extract" => include_str!("../prompts/extract.md"),
        "audit" => include_str!("../prompts/audit.md"),
        "resolve_pair" => include_str!("../prompts/resolve_pair.md"),
        "resolve_cluster" => include_str!("../prompts/resolve_cluster.md"),
        "organize" => include_str!("../prompts/organize.md"),
        "write" => include_str!("../prompts/write.md"),
        "review" => include_str!("../prompts/review.md"),
        _ => anyhow::bail!("unknown prompt template: {name}"),
    };
    let version = body
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("version: "))
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| anyhow::anyhow!("invalid prompt version: {name}"))?;
    ensure!(version > 0, "prompt version must be positive");
    Ok((body, version))
}

pub fn profile(name: &str) -> Result<&'static str> {
    match name {
        "generic" => Ok(include_str!("../profiles/generic.yaml")),
        "ttrpg" => Ok(include_str!("../profiles/ttrpg.yaml")),
        _ => anyhow::bail!("unknown profile: {name}"),
    }
}
