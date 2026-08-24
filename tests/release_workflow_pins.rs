//! Structural gate: release.yml pins every action to a 40-char SHA
//! and checksum-verifies cargo-dist / rustup downloads.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(rel: &str) -> String {
    std::fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

fn active_uses(yml: &str) -> Vec<(usize, String)> {
    yml.lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                return None;
            }
            let core = trimmed.trim_start_matches('-').trim();
            if core.starts_with("uses:") {
                Some((i + 1, trimmed.to_string()))
            } else {
                None
            }
        })
        .collect()
}

fn pin_after_at(line: &str) -> Option<&str> {
    let uses = line.split("uses:").nth(1)?;
    let after_at = uses.split_once('@')?.1.trim();
    Some(after_at.split_whitespace().next()?.trim())
}

fn is_hex40(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Same shape as `rg -n 'uses:.*@(v[0-9]|stable|main|master)'`.
fn matches_mutable_reproducer(line: &str) -> bool {
    let Some((_, after_uses)) = line.split_once("uses:") else {
        return false;
    };
    let Some((_, after_at)) = after_uses.split_once('@') else {
        return false;
    };
    let pin = after_at.split_whitespace().next().unwrap_or("");
    (pin.starts_with('v') && pin.chars().nth(1).is_some_and(|c| c.is_ascii_digit()))
        || pin == "stable"
        || pin == "main"
        || pin == "master"
}

fn is_pipe_to_shell(line: &str) -> bool {
    let t = line.trim();
    if t.starts_with('#') {
        return false;
    }
    let has_pipe_sh = t.contains('|')
        && (t.contains("| sh")
            || t.contains("|sh")
            || t.contains("| sh -s")
            || t.contains(" sh -s"));
    has_pipe_sh || t.contains("| iex") || t.contains("|iex")
}

#[test]
fn release_yml_action_pins_are_full_shas() {
    let yml = read_rel(".github/workflows/release.yml");
    let uses = active_uses(&yml);
    assert!(
        !uses.is_empty(),
        "release.yml must declare at least one GitHub Action"
    );
    let mut bad = Vec::new();
    for (lineno, line) in &uses {
        let pin = pin_after_at(line).unwrap_or("");
        if !is_hex40(pin) || !line.contains('#') {
            bad.push(format!("{lineno}: {line}"));
        }
    }
    assert!(
        bad.is_empty(),
        "release.yml uses: lines must be owner/repo@<40-hex> # version\n{}",
        bad.join("\n")
    );
}

#[test]
fn release_yml_reproducer_has_no_mutable_action_refs() {
    let yml = read_rel(".github/workflows/release.yml");
    let hits: Vec<_> = yml
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            let trimmed = line.trim();
            !trimmed.starts_with('#') && matches_mutable_reproducer(trimmed)
        })
        .map(|(i, line)| format!("{}: {line}", i + 1))
        .collect();
    assert!(
        hits.is_empty(),
        "rg 'uses:.*@(v[0-9]|stable|main|master)' .github/workflows/release.yml must be empty:\n{}",
        hits.join("\n")
    );
}

#[test]
fn release_yml_does_not_pipe_unverified_installers() {
    let yml = read_rel(".github/workflows/release.yml");
    let hits: Vec<_> = yml
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            is_pipe_to_shell(line)
                || line.contains("sh.rustup.rs")
                || line.contains("cargo-dist-installer.sh")
        })
        .map(|(i, line)| format!("{}: {line}", i + 1))
        .collect();
    assert!(
        hits.is_empty(),
        "release.yml must not pipe cargo-dist-installer.sh or rustup.sh into a shell:\n{}",
        hits.join("\n")
    );
}

#[test]
fn release_installers_verify_sha256() {
    let yml = read_rel(".github/workflows/release.yml");
    let script = repo_root().join("scripts/install-cargo-dist-ci.sh");
    assert!(
        yml.contains("install-cargo-dist-ci.sh") || yml.contains("sha256sum"),
        "release.yml must checksum-verify cargo-dist before executing it"
    );
    if script.is_file() {
        let body = std::fs::read_to_string(&script).expect("read install-cargo-dist-ci.sh");
        assert!(
            body.contains("0.28.0"),
            "install-cargo-dist-ci.sh must pin cargo-dist 0.28.0"
        );
        // x86_64-unknown-linux-gnu tarball from the v0.28.0 GitHub Release
        assert!(
            body.contains("c5da0fc4e782315e860bf5d1fb5f9a35e0e78c2d61f27662dfb096cf43de12d8"),
            "install-cargo-dist-ci.sh must pin the linux-gnu cargo-dist SHA-256"
        );
        assert!(
            !body.lines().any(is_pipe_to_shell),
            "install-cargo-dist-ci.sh must not pipe the installer into sh"
        );
        assert_checksum_function(&script, &body);
    } else {
        assert!(
            yml.contains("c5da0fc4e782315e860bf5d1fb5f9a35e0e78c2d61f27662dfb096cf43de12d8"),
            "release.yml must pin the cargo-dist 0.28.0 linux-gnu SHA-256 when no helper script exists"
        );
    }

    let has_rustup_checksum = yml.contains("rustup-init")
        && (yml.contains("sha256sum")
            || yml.contains("4acc9acc76d5079515b46346a485974457b5a79893cfb01112423c89aeb5aa10"));
    let has_pinned_toolchain = yml.lines().any(|line| {
        let trimmed = line.trim();
        !trimmed.starts_with('#')
            && trimmed.contains("rust-toolchain@")
            && pin_after_at(trimmed).is_some_and(is_hex40)
    });
    assert!(
        has_rustup_checksum || has_pinned_toolchain,
        "container Rust install must checksum rustup-init or use a SHA-pinned toolchain action"
    );
}

fn assert_checksum_function(path: &Path, body: &str) {
    assert!(
        body.contains("sha256") || body.contains("SHA256") || body.contains("shasum"),
        "{} must verify a SHA-256 before installing",
        path.display()
    );
}
