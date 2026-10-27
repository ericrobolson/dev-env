use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};
use tempfile::tempdir;
use wiki_writer::{project, types::Provider};

fn mock(bin: &Path, name: &str, body: &str) {
    let path = bin.join(name);
    fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn setup() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.md");
    fs::write(
        &source,
        "# Demo\nThe source contains enough text for a valid sample document.\n",
    )
    .unwrap();
    let project = project::init(dir.path(), "demo", &[source], Provider::Codex, "generic").unwrap();
    let bin = dir.path().join("bin");
    fs::create_dir(&bin).unwrap();
    mock(
        &bin,
        "curl",
        "printf 'curl\\n' >> \"$CAPTURE_DIR/calls\"\nfor arg do :; done\nprintf 'archive' > \"$arg\"",
    );
    mock(
        &bin,
        "shasum",
        "printf 'shasum\\n' >> \"$CAPTURE_DIR/calls\"\ncat >/dev/null\n[ \"${FAIL_CHECKSUM:-0}\" = 0 ]",
    );
    mock(
        &bin,
        "tar",
        "printf 'tar\\n' >> \"$CAPTURE_DIR/calls\"\n[ \"${FAIL_TAR:-0}\" = 0 ] || exit 1\nwhile [ \"$1\" != -C ]; do shift; done\nshift\nmkdir -p \"$1/conf\"\nprintf 'php' > \"$1/doku.php\"\nmkdir -p \"$1/data/pages\"",
    );
    mock(
        &bin,
        "wiki-writer",
        "printf '%s\\n' \"$*\" >> \"$CAPTURE_DIR/doctor\"",
    );
    (dir, project, bin)
}

fn install(
    project: &Path,
    bin: &Path,
    capture: &Path,
    fail_checksum: bool,
    fail_tar: bool,
) -> std::process::Output {
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    Command::new("sh")
        .arg("state/install.sh")
        .current_dir(project)
        .env("PATH", path)
        .env("CAPTURE_DIR", capture)
        .env("FAIL_CHECKSUM", if fail_checksum { "1" } else { "0" })
        .env("FAIL_TAR", if fail_tar { "1" } else { "0" })
        .output()
        .unwrap()
}

#[test]
fn checksum_precedes_extraction_and_failure_preserves_destination() {
    let (dir, project, bin) = setup();
    fs::create_dir(project.join("dokuwiki")).unwrap();
    let output = install(&project, &bin, dir.path(), true, false);
    assert!(!output.status.success());
    let calls = fs::read_to_string(dir.path().join("calls")).unwrap();
    assert!(calls.contains("shasum"));
    assert!(!calls.contains("tar"));
    assert!(project.join("dokuwiki").is_dir());
    assert!(!project.join("dokuwiki/doku.php").exists());
}

#[test]
fn successful_install_configures_read_only_local_wiki_and_repeat_preserves_edits() {
    let (dir, project, bin) = setup();
    let output = install(&project, &bin, dir.path(), false, false);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let local = fs::read_to_string(project.join("dokuwiki/conf/local.php")).unwrap();
    assert!(local.contains("$conf['title'] = 'demo';"));
    assert!(local.contains("$conf['useacl'] = 1;"));
    assert!(local.contains("$conf['authtype'] = 'authplain';"));
    assert!(local.contains("$conf['userewrite'] = 0;"));
    let acl = fs::read_to_string(project.join("dokuwiki/conf/acl.auth.php")).unwrap();
    assert!(acl.contains("* @ALL 1"));
    assert!(project.join("dokuwiki/conf/users.auth.php").exists());
    assert!(!project.join("dokuwiki/install.php").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("doctor"))
            .unwrap()
            .trim(),
        "doctor"
    );
    fs::write(project.join("dokuwiki/conf/local.php"), "user config").unwrap();
    let output = install(&project, &bin, dir.path(), false, false);
    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(project.join("dokuwiki/conf/local.php")).unwrap(),
        "user config"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("calls"))
            .unwrap()
            .matches("curl")
            .count(),
        1
    );
    let makefile = fs::read_to_string(project.join("Makefile")).unwrap();
    assert!(makefile.contains("php -S 127.0.0.1:8080 -t dokuwiki"));
    assert!(makefile.contains("wiki-writer render --project ."));
    assert!(makefile.contains("php dokuwiki/bin/indexer.php -c"));
}

#[test]
fn install_preserves_pages_rendered_before_dokuwiki_is_installed() {
    let (dir, project, bin) = setup();
    let page = project.join("dokuwiki/data/pages/start.txt");
    fs::create_dir_all(page.parent().unwrap()).unwrap();
    fs::write(&page, "====== Demo ======\n").unwrap();
    let output = install(&project, &bin, dir.path(), false, false);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(page).unwrap(), "====== Demo ======\n");
    assert!(project.join("dokuwiki/doku.php").exists());
}

#[test]
fn failed_tar_leaves_no_install_and_clean_keeps_sources() {
    let (dir, project, bin) = setup();
    let output = install(&project, &bin, dir.path(), false, true);
    assert!(!output.status.success());
    assert!(!project.join("dokuwiki").exists());
    let output = install(&project, &bin, dir.path(), false, false);
    assert!(output.status.success());
    let clean = Command::new("make")
        .arg("clean")
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(clean.status.success());
    assert!(!project.join("dokuwiki").exists());
    assert!(project.join("sources/s0001.md").exists());
    assert!(project.join("state/project.json").exists());
    let outside = dir.path().join("outside");
    fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, project.join("dokuwiki")).unwrap();
    let clean = Command::new("make")
        .arg("clean")
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(!clean.status.success());
    assert!(outside.is_dir());
}
