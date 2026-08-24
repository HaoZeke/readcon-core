//! Structural gate: PR-triggered workflows do not grant Pages or OIDC writes.
//!
//! `ci_docs.yml` and `coverage.yml` both run on `pull_request` and execute
//! tooling from the checked-out head. Pages deploy and Codecov OIDC belong
//! only on trusted publication jobs (push / workflow_dispatch), not at
//! workflow scope where every PR job inherits them.

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workflows_dir() -> PathBuf {
    repo_root().join(".github/workflows")
}

fn read_workflow(name: &str) -> String {
    let path = workflows_dir().join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Text before the top-level `jobs:` key (workflow-scope YAML).
fn workflow_preamble(src: &str) -> &str {
    match src.find("\njobs:") {
        Some(idx) => &src[..=idx],
        None => src,
    }
}

fn normalize_active(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    Some(
        trimmed
            .replace(['"', '\''], "")
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect(),
    )
}

fn active_contains(block: &str, needle: &str) -> bool {
    block
        .lines()
        .filter_map(normalize_active)
        .any(|norm| norm.contains(needle))
}

fn triggers_on_pull_request(preamble: &str) -> bool {
    preamble.lines().filter_map(normalize_active).any(|norm| {
        norm == "pull_request:" || norm == "pull_request" || norm.starts_with("pull_request:")
    })
}

/// Job id plus body for each top-level job under `jobs:`.
fn jobs(src: &str) -> Vec<(String, String)> {
    let rest = src.split_once("\njobs:").map(|(_, r)| r).unwrap_or("");
    let mut out = Vec::new();
    let mut current: Option<(String, String)> = None;
    for line in rest.lines() {
        if let Some(name) = top_level_job_name(line) {
            if let Some(prev) = current.take() {
                out.push(prev);
            }
            current = Some((name, String::new()));
            continue;
        }
        if let Some((_, body)) = current.as_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some(prev) = current {
        out.push(prev);
    }
    out
}

fn top_level_job_name(line: &str) -> Option<String> {
    if !line.starts_with("  ") || line.starts_with("   ") {
        return None;
    }
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        return None;
    }
    let name = trimmed.strip_suffix(':')?;
    if name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        Some(name.to_string())
    } else {
        None
    }
}

fn job_excludes_pull_request(body: &str) -> bool {
    let has_if = body
        .lines()
        .filter_map(normalize_active)
        .any(|norm| norm.starts_with("if:"));
    if !has_if {
        return false;
    }
    let blob: String = body.lines().filter_map(normalize_active).collect();
    blob.contains("github.event_name!=pull_request")
        || blob.contains("github.event_name==push")
        || blob.contains("github.ref==refs/heads/main")
        || blob.contains("startsWith(github.ref,refs/tags/")
}

fn job_has_oidc_or_pages_write(body: &str) -> bool {
    active_contains(body, "id-token:write")
        || active_contains(body, "pages:write")
        || active_contains(body, "use_oidc:true")
}

#[test]
fn pr_triggered_workflows_omit_pages_and_oidc_at_workflow_scope() {
    let dir = workflows_dir();
    let mut scanned = 0usize;
    for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let entry = entry.expect("workflow dirent");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yml") {
            continue;
        }
        let src =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let preamble = workflow_preamble(&src);
        if !triggers_on_pull_request(preamble) {
            continue;
        }
        scanned += 1;
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("workflow");
        for perm in ["pages:write", "id-token:write"] {
            assert!(
                !active_contains(preamble, perm),
                "{name} grants {perm} at workflow scope while on.pull_request is set"
            );
        }
        assert!(
            !active_contains(preamble, "permissions:write-all"),
            "{name} grants write-all at workflow scope while on.pull_request is set"
        );
    }
    assert!(
        scanned >= 2,
        "expected to scan pull_request workflows under {}",
        dir.display()
    );
}

#[test]
fn ci_docs_keeps_pages_and_oidc_on_trusted_deploy_only() {
    let src = read_workflow("ci_docs.yml");
    assert!(
        triggers_on_pull_request(workflow_preamble(&src)),
        "ci_docs.yml must still validate pull requests"
    );

    let jobs = jobs(&src);
    let deploy = jobs
        .iter()
        .find(|(name, _)| name == "deploy")
        .map(|(_, body)| body.as_str())
        .expect("ci_docs.yml deploy job");
    assert!(
        active_contains(deploy, "pages:write"),
        "deploy job must request pages: write"
    );
    assert!(
        active_contains(deploy, "id-token:write"),
        "deploy job must request id-token: write"
    );
    assert!(
        job_excludes_pull_request(deploy),
        "deploy job must not run on pull_request"
    );

    let build = jobs
        .iter()
        .find(|(name, _)| name == "build")
        .map(|(_, body)| body.as_str())
        .expect("ci_docs.yml build job");
    assert!(
        !job_has_oidc_or_pages_write(build),
        "build job must stay read-only for PR documentation validation"
    );
}

#[test]
fn coverage_oidc_stays_on_trusted_publication_only() {
    let src = read_workflow("coverage.yml");
    assert!(
        triggers_on_pull_request(workflow_preamble(&src)),
        "coverage.yml must still generate coverage on pull_request"
    );
    assert!(
        !active_contains(workflow_preamble(&src), "id-token:write"),
        "coverage.yml must not grant id-token: write at workflow scope"
    );

    let mut oidc_jobs = 0usize;
    for (name, body) in jobs(&src) {
        if job_has_oidc_or_pages_write(&body) {
            assert!(
                job_excludes_pull_request(&body),
                "coverage.yml job {name} has Pages/OIDC writes but can run on pull_request"
            );
            oidc_jobs += 1;
        }
    }
    assert!(
        oidc_jobs >= 1,
        "coverage.yml must keep a trusted OIDC publication job"
    );

    for flag in ["rust", "python", "julia", "fortran"] {
        let needle = format!("flags: {flag}");
        assert!(
            src.lines()
                .any(|l| l.contains(&needle) && !l.trim_start().starts_with('#')),
            "coverage.yml missing active upload flags: {flag}"
        );
    }
    assert!(src.contains("use_oidc: true"));
    assert!(src.contains("codecov/codecov-action"));
}

#[test]
fn workflow_permission_gate_files_exist() {
    for rel in [
        ".github/workflows/ci_docs.yml",
        ".github/workflows/coverage.yml",
    ] {
        let path = repo_root().join(rel);
        assert!(path.is_file(), "missing {}", path.display());
    }
}
