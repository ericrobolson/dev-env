use std::os::unix::fs::PermissionsExt;
use std::{fs, process::Command};
use tempfile::tempdir;

#[test]
fn binary_rejects_invalid_usage_and_initializes_project() {
    let binary = env!("CARGO_BIN_EXE_wiki-writer");
    let bad = Command::new(binary)
        .args(["init", "demo"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    let bad = Command::new(binary)
        .args(["run", "--only", "extract", "--until", "review"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("source.md"),
        "# Demo\nThe copper token weighs 12 grams and the silver token weighs 24 grams.\n",
    )
    .unwrap();
    let init = Command::new(binary)
        .current_dir(dir.path())
        .args(["init", "demo", "source.md", "--agent", "codex"])
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    assert!(dir.path().join("wikis/demo/wikify.yaml").exists());
    let status = Command::new(binary)
        .current_dir(dir.path().join("wikis/demo"))
        .arg("status")
        .output()
        .unwrap();
    assert!(status.status.success());
    assert!(String::from_utf8_lossy(&status.stdout).contains("stage"));
    assert!(String::from_utf8_lossy(&status.stdout).contains("unknown"));
    let printed = String::from_utf8_lossy(&status.stdout);
    assert!(printed.contains("ingest 0 1 0"), "{printed}");
    assert!(printed.contains("waiting for ingest"), "{printed}");
}

#[test]
fn run_needing_acceptance_exits_four_without_model_calls() {
    let binary = env!("CARGO_BIN_EXE_wiki-writer");
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("source.md"),
        "# Rules\nA copper token weighs 12 grams.\n",
    )
    .unwrap();
    assert!(
        Command::new(binary)
            .current_dir(dir.path())
            .args(["init", "demo", "source.md", "--agent", "codex"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let project = dir.path().join("wikis/demo");
    let acceptance = Command::new(binary)
        .current_dir(&project)
        .arg("run")
        .output()
        .unwrap();
    assert_eq!(acceptance.status.code(), Some(4));
    assert!(!project.join("state/ledger.jsonl").exists());
}

#[test]
fn redo_refuses_corrupt_forced_state_without_changing_record() {
    let binary = env!("CARGO_BIN_EXE_wiki-writer");
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("source.md"),
        "# Rules\nA copper token weighs 12 grams.\n",
    )
    .unwrap();
    assert!(
        Command::new(binary)
            .current_dir(dir.path())
            .args(["init", "demo", "source.md", "--agent", "codex"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let project = dir.path().join("wikis/demo");
    let _ = Command::new(binary)
        .current_dir(&project)
        .args(["run", "--until", "ingest"])
        .output()
        .unwrap();
    let record = project.join("state/items/ingest/s0001.json");
    let before = fs::read(&record).unwrap();
    let forced = project.join("state/forced.json");
    fs::write(&forced, b"{broken").unwrap();
    let redo = Command::new(binary)
        .current_dir(&project)
        .args(["redo", "ingest", "--item", "s0001"])
        .output()
        .unwrap();
    assert!(!redo.status.success());
    assert_eq!(fs::read(&record).unwrap(), before);
    assert_eq!(fs::read(&forced).unwrap(), b"{broken");
}

#[test]
fn doctor_rejects_executables_whose_version_command_fails() {
    let binary = env!("CARGO_BIN_EXE_wiki-writer");
    let dir = tempdir().unwrap();
    for name in ["pdfinfo", "pdftotext", "php", "codex"] {
        let path = dir.path().join(name);
        let status = if name == "php" { 1 } else { 0 };
        fs::write(&path, format!("#!/bin/sh\nexit {status}\n")).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
    }
    let output = Command::new(binary)
        .arg("doctor")
        .current_dir(dir.path())
        .env("PATH", dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("php"));
}

#[test]
fn status_lists_scanned_pages_after_ingest() {
    let binary = env!("CARGO_BIN_EXE_wiki-writer");
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("source.md"),
        format!("# Short\nsmall\n# Long\n{}\n", "a".repeat(50)),
    )
    .unwrap();
    let init = Command::new(binary)
        .current_dir(dir.path())
        .args(["init", "demo", "source.md", "--agent", "codex"])
        .output()
        .unwrap();
    assert!(init.status.success());
    let project = dir.path().join("wikis/demo");
    let run = Command::new(binary)
        .current_dir(&project)
        .args(["run", "--until", "ingest"])
        .output()
        .unwrap();
    assert_eq!(
        run.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let output = Command::new(binary)
        .current_dir(&project)
        .arg("status")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(printed.contains("scanned: s0001 page 1"), "{printed}");
    assert!(printed.contains("survey 0 1 0"), "{printed}");
}

#[test]
fn status_marks_changed_source_ingest_stale() {
    let binary = env!("CARGO_BIN_EXE_wiki-writer");
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("source.md"),
        format!("# Rules\n{}", "old text ".repeat(12)),
    )
    .unwrap();
    assert!(
        Command::new(binary)
            .current_dir(dir.path())
            .args(["init", "demo", "source.md", "--agent", "codex"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let project = dir.path().join("wikis/demo");
    let _ = Command::new(binary)
        .current_dir(&project)
        .args(["run", "--until", "ingest"])
        .output()
        .unwrap();
    fs::write(
        project.join("sources/s0001.md"),
        format!("# Rules\n{}", "new text ".repeat(12)),
    )
    .unwrap();
    let output = Command::new(binary)
        .current_dir(&project)
        .arg("status")
        .output()
        .unwrap();
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(printed.contains("ingest 0 1 0"), "{printed}");
    assert!(printed.contains("waiting for ingest"), "{printed}");
}
