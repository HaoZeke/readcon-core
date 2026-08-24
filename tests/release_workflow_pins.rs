//! Structural gate: release.yml actions are SHA-pinned and installers verified.
//!
//! Drives `scripts/check_release_workflow_pins.sh` so a `dist generate`
//! that restores mutable tags or curl|sh installers fails `cargo test`.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn release_workflow_action_pins_are_shas() {
    let script = repo_root().join("scripts/check_release_workflow_pins.sh");
    assert!(
        script.is_file(),
        "missing {}; expected release.yml pin gate",
        script.display()
    );
    let status = Command::new("bash")
        .arg(&script)
        .current_dir(repo_root())
        .status()
        .expect("failed to spawn scripts/check_release_workflow_pins.sh");
    assert!(
        status.success(),
        "scripts/check_release_workflow_pins.sh failed (exit {:?})",
        status.code()
    );
}

#[test]
fn release_yml_has_no_mutable_action_tags() {
    let wf = std::fs::read_to_string(repo_root().join(".github/workflows/release.yml"))
        .expect("read release.yml");
    let mutable = regex_mutable_uses(&wf);
    assert!(
        mutable.is_empty(),
        "release.yml still has mutable uses: tags:\n{}",
        mutable.join("\n")
    );
}

fn regex_mutable_uses(wf: &str) -> Vec<String> {
    wf.lines()
        .filter(|line| {
            let l = line.trim();
            if !l.contains("uses:") {
                return false;
            }
            l.contains("@v")
                || l.contains("@stable")
                || l.contains("@main")
                || l.contains("@master")
        })
        .filter(|line| {
            // SHA pins may mention a version only in the comment after #.
            let before_comment = line.split('#').next().unwrap_or(line);
            before_comment.contains("@v")
                || before_comment.contains("@stable")
                || before_comment.contains("@main")
                || before_comment.contains("@master")
        })
        .map(str::to_string)
        .collect()
}
