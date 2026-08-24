//! Structural gate: PR-triggered workflows must not grant Pages or OIDC
//! at workflow scope.
//!
//! Drives `scripts/check_workflow_permissions.sh` and asserts that
//! `ci_docs.yml` / `coverage.yml` keep those writes on trusted jobs.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_workflow(name: &str) -> String {
    let path = repo_root().join(".github/workflows").join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Body of a column-0 YAML key (inline remainder plus indented children).
fn top_level_block(src: &str, key: &str) -> String {
    let prefix = format!("{key}:");
    let mut lines = src.lines();
    let mut out = String::new();
    let mut collecting = false;
    for line in lines.by_ref() {
        if !collecting {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix(&prefix) {
                collecting = true;
                let rest = rest.trim();
                if !rest.is_empty() {
                    out.push_str(rest);
                    out.push('\n');
                }
            }
            continue;
        }
        if !line.is_empty()
            && !line.starts_with('#')
            && !line.starts_with(' ')
            && !line.starts_with('\t')
        {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn on_is_pr_triggered(on: &str) -> bool {
    on.lines().any(|line| {
        let t = line.trim();
        let bare = t
            .trim_start_matches('-')
            .trim()
            .trim_matches(['\'', '"', ',', ']']);
        bare == "pull_request"
            || bare == "pull_request_target"
            || t.starts_with("pull_request:")
            || t.starts_with("pull_request_target:")
            || t.starts_with("- pull_request")
    }) || on
        .split([',', '[', ']', ' ', '\t'])
        .any(|tok| matches!(tok, "pull_request" | "pull_request_target"))
}

fn perm_has_write(perms: &str, name: &str) -> bool {
    perms.lines().any(|line| {
        let t = line.trim().trim_start_matches(['"', '\'']).replace('"', "");
        t.starts_with(&format!("{name}:")) && t.contains("write")
    })
}

#[derive(Debug)]
struct Job {
    name: String,
    if_expr: Option<String>,
    permissions: String,
}

fn parse_jobs(src: &str) -> Vec<Job> {
    let jobs_block = top_level_block(src, "jobs");
    let mut jobs = Vec::new();
    let mut current: Option<Job> = None;
    let mut in_permissions = false;
    for line in jobs_block.lines() {
        if let Some(name) = job_header(line) {
            if let Some(job) = current.take() {
                jobs.push(job);
            }
            current = Some(Job {
                name,
                if_expr: None,
                permissions: String::new(),
            });
            in_permissions = false;
            continue;
        }
        let Some(job) = current.as_mut() else {
            continue;
        };
        if let Some(rest) = line.strip_prefix("    if:") {
            job.if_expr = Some(rest.trim().to_string());
            in_permissions = false;
            continue;
        }
        if line.starts_with("    permissions:") {
            in_permissions = true;
            let rest = line["    permissions:".len()..].trim();
            if !rest.is_empty() {
                job.permissions.push_str(rest);
                job.permissions.push('\n');
            }
            continue;
        }
        if in_permissions {
            if line.starts_with("      ")
                || line.trim().is_empty()
                || line.trim_start().starts_with('#')
            {
                job.permissions.push_str(line);
                job.permissions.push('\n');
            } else {
                in_permissions = false;
            }
        }
    }
    if let Some(job) = current {
        jobs.push(job);
    }
    jobs
}

fn job_header(line: &str) -> Option<String> {
    if !line.starts_with("  ") || line.starts_with("    ") {
        return None;
    }
    let t = line.trim();
    if t.starts_with('#') || !t.ends_with(':') {
        return None;
    }
    let name = t.trim_end_matches(':');
    if name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        Some(name.to_string())
    } else {
        None
    }
}

fn job_is_trusted_gate(if_expr: Option<&str>) -> bool {
    let Some(ife) = if_expr else {
        return false;
    };
    ife.contains("github.event_name != 'pull_request'")
        || ife.contains("github.event_name != \"pull_request\"")
        || ife.contains("github.event_name == 'push'")
        || ife.contains("github.event_name == \"push\"")
        || ife.contains("github.ref == 'refs/heads/main'")
        || ife.contains("github.ref == \"refs/heads/main\"")
        || ife.contains("startsWith(github.ref, 'refs/tags/")
}

#[test]
fn check_workflow_permissions_script_passes() {
    let script = repo_root().join("scripts/check_workflow_permissions.sh");
    assert!(
        script.is_file(),
        "missing {}; expected workflow-scope permission gate",
        script.display()
    );
    let status = Command::new("bash")
        .arg(&script)
        .current_dir(repo_root())
        .status()
        .expect("failed to spawn scripts/check_workflow_permissions.sh");
    assert!(
        status.success(),
        "scripts/check_workflow_permissions.sh failed (exit {:?})",
        status.code()
    );
}

#[test]
fn pr_workflows_do_not_grant_pages_or_oidc_at_workflow_scope() {
    let dir = repo_root().join(".github/workflows");
    let mut seen = BTreeMap::new();
    for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let entry = entry.expect("workflow dirent");
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "yml" && ext != "yaml" {
            continue;
        }
        let src =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let on = top_level_block(&src, "on");
        if !on_is_pr_triggered(&on) {
            continue;
        }
        let perms = top_level_block(&src, "permissions");
        assert!(
            !perm_has_write(&perms, "pages"),
            "{name}: workflow-scope pages: write on a pull_request workflow"
        );
        assert!(
            !perm_has_write(&perms, "id-token"),
            "{name}: workflow-scope id-token: write on a pull_request workflow"
        );
        seen.insert(name, perms);
    }
    assert!(
        seen.contains_key("ci_docs.yml"),
        "ci_docs.yml must stay pull_request-triggered"
    );
    assert!(
        seen.contains_key("coverage.yml"),
        "coverage.yml must stay pull_request-triggered"
    );
}

#[test]
fn ci_docs_keeps_pages_and_oidc_on_deploy_job() {
    let wf = read_workflow("ci_docs.yml");
    let scope = top_level_block(&wf, "permissions");
    assert!(
        !perm_has_write(&scope, "pages") && !perm_has_write(&scope, "id-token"),
        "ci_docs.yml workflow-scope permissions must not include pages/id-token write:\n{scope}"
    );
    assert!(
        wf.contains("actions/upload-pages-artifact"),
        "PR docs preview commenter reads the github-pages artifact"
    );
    let jobs = parse_jobs(&wf);
    let deploy = jobs
        .iter()
        .find(|j| j.name == "deploy")
        .expect("ci_docs.yml must keep a deploy job");
    assert!(
        perm_has_write(&deploy.permissions, "pages"),
        "deploy job must grant pages: write"
    );
    assert!(
        perm_has_write(&deploy.permissions, "id-token"),
        "deploy job must grant id-token: write"
    );
    assert!(
        job_is_trusted_gate(deploy.if_expr.as_deref()),
        "deploy job must be gated off pull_request (if: {:?})",
        deploy.if_expr
    );
    let build = jobs
        .iter()
        .find(|j| j.name == "build")
        .expect("ci_docs.yml must keep a build job");
    assert!(
        !perm_has_write(&build.permissions, "pages")
            && !perm_has_write(&build.permissions, "id-token"),
        "build job must not grant pages/id-token write"
    );
}

#[test]
fn coverage_keeps_oidc_on_trusted_upload_jobs() {
    let wf = read_workflow("coverage.yml");
    let scope = top_level_block(&wf, "permissions");
    assert!(
        !perm_has_write(&scope, "id-token"),
        "coverage.yml workflow-scope permissions must not include id-token write:\n{scope}"
    );
    assert!(
        wf.contains("use_oidc: true"),
        "coverage.yml must still upload via OIDC on trusted jobs"
    );
    assert!(
        wf.contains("id-token: write"),
        "coverage.yml must still grant id-token: write on trusted upload jobs"
    );
    let jobs = parse_jobs(&wf);
    let mut oidc_jobs = 0;
    for job in &jobs {
        if perm_has_write(&job.permissions, "id-token") {
            oidc_jobs += 1;
            assert!(
                job_is_trusted_gate(job.if_expr.as_deref()),
                "coverage job {} grants id-token: write without a trusted-event gate (if: {:?})",
                job.name,
                job.if_expr
            );
        }
    }
    assert!(
        oidc_jobs >= 1,
        "coverage.yml must grant id-token: write on at least one trusted upload job"
    );
}

#[test]
fn workflow_parser_sees_column_zero_permissions() {
    // Fixture shape matches ci_docs.yml / coverage.yml (mapping-form `on:`).
    let src = "\
name: Fixture
on:
  pull_request:
  push:
permissions:
  contents: read
  pages: write
  id-token: write
jobs:
  build:
    runs-on: ubuntu-latest
";
    let on = top_level_block(src, "on");
    let perms = top_level_block(src, "permissions");
    assert!(
        on_is_pr_triggered(&on),
        "fixture on: must parse as PR-triggered"
    );
    assert!(perm_has_write(&perms, "pages"));
    assert!(perm_has_write(&perms, "id-token"));
}
