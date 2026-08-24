//! Language-binding package versions stay in lockstep with Cargo.toml.
//!
//! Canonical source: `Cargo.toml` `[package].version`.
//! Bindings that may lag on purpose go in `VERSION_ALLOWLIST` with a reason.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Bindings that may lag Cargo.toml on purpose.
/// Each entry is `(relative path or label, reason)`. Empty: every source matches.
const VERSION_ALLOWLIST: &[(&str, &str)] = &[
    // ("julia/ReadCon/Project.toml", "independent Julia registry release"),
];

#[derive(Clone, Copy)]
enum Kind {
    /// First `version = "x"` assignment (Cargo, pixi, pyproject, Julia, fpm).
    TomlVersion,
    /// Meson `project(..., version: 'x', ...)`.
    MesonVersion,
    /// CITATION.cff software version (not `cff-version`).
    CffVersion,
    /// `pyproject.toml` extra pin `readcon-chemfiles==x`.
    ChemfilesExtraPin,
}

struct Source {
    path: &'static str,
    kind: Kind,
    label: &'static str,
}

const SOURCES: &[Source] = &[
    Source {
        path: "Cargo.toml",
        kind: Kind::TomlVersion,
        label: "Cargo.toml",
    },
    Source {
        path: "pyproject.toml",
        kind: Kind::TomlVersion,
        label: "pyproject.toml",
    },
    Source {
        path: "pyproject.toml",
        kind: Kind::ChemfilesExtraPin,
        label: "pyproject.toml[chemfiles]",
    },
    Source {
        path: "pyproject.chemfiles.toml",
        kind: Kind::TomlVersion,
        label: "pyproject.chemfiles.toml",
    },
    Source {
        path: "meson.build",
        kind: Kind::MesonVersion,
        label: "meson.build",
    },
    Source {
        path: "pixi.toml",
        kind: Kind::TomlVersion,
        label: "pixi.toml",
    },
    Source {
        path: "julia/ReadCon/Project.toml",
        kind: Kind::TomlVersion,
        label: "julia/ReadCon/Project.toml",
    },
    Source {
        path: "fortran/ReadCon/fpm.toml",
        kind: Kind::TomlVersion,
        label: "fortran/ReadCon/fpm.toml",
    },
    Source {
        path: "CITATION.cff",
        kind: Kind::CffVersion,
        label: "CITATION.cff",
    },
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn unquote(raw: &str) -> Option<String> {
    let s = raw.trim().trim_end_matches(',').trim();
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let a = bytes[0];
        let b = bytes[bytes.len() - 1];
        if (a == b'"' && b == b'"') || (a == b'\'' && b == b'\'') {
            return Some(s[1..s.len() - 1].to_string());
        }
    }
    None
}

fn toml_version(text: &str) -> Option<String> {
    for line in text.lines() {
        let t = line.trim();
        let Some(rest) = t.strip_prefix("version") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        if let Some(v) = unquote(rest) {
            return Some(v);
        }
    }
    None
}

fn meson_version(text: &str) -> Option<String> {
    for line in text.lines() {
        let t = line.trim();
        let Some(rest) = t.strip_prefix("version:") else {
            continue;
        };
        if let Some(v) = unquote(rest) {
            return Some(v);
        }
    }
    None
}

fn cff_version(text: &str) -> Option<String> {
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("cff-version:") {
            continue;
        }
        let Some(rest) = t.strip_prefix("version:") else {
            continue;
        };
        let rest = rest.trim();
        if let Some(v) = unquote(rest) {
            return Some(v);
        }
        if let Some(v) = rest.split_whitespace().next() {
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn chemfiles_extra_pin(text: &str) -> Option<String> {
    const NEEDLE: &str = "readcon-chemfiles==";
    let i = text.find(NEEDLE)?;
    let rest = &text[i + NEEDLE.len()..];
    let ver: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if ver.is_empty() { None } else { Some(ver) }
}

fn extract(text: &str, kind: Kind) -> Option<String> {
    match kind {
        Kind::TomlVersion => toml_version(text),
        Kind::MesonVersion => meson_version(text),
        Kind::CffVersion => cff_version(text),
        Kind::ChemfilesExtraPin => chemfiles_extra_pin(text),
    }
}

fn allowlisted(label: &str, path: &str) -> bool {
    VERSION_ALLOWLIST
        .iter()
        .any(|(key, _reason)| *key == label || *key == path)
}

/// Compare every binding version in `root` to Cargo.toml. Ok = cargo version.
fn check_tree(root: &Path) -> Result<String, Vec<String>> {
    let cargo_text = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|e| vec![format!("Cargo.toml: read failed: {e}")])?;
    let cargo = toml_version(&cargo_text)
        .ok_or_else(|| vec!["Cargo.toml: missing package version".to_string()])?;

    let mut errs = Vec::new();
    for src in SOURCES {
        if src.path == "Cargo.toml" && matches!(src.kind, Kind::TomlVersion) {
            continue;
        }
        if allowlisted(src.label, src.path) {
            continue;
        }
        let path = root.join(src.path);
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                errs.push(format!("{}: read failed: {e}", src.label));
                continue;
            }
        };
        match extract(&text, src.kind) {
            Some(found) if found == cargo => {}
            Some(found) => errs.push(format!(
                "{}: version {found} != Cargo.toml {cargo}",
                src.label
            )),
            None => errs.push(format!("{}: missing version", src.label)),
        }
    }
    if errs.is_empty() {
        Ok(cargo)
    } else {
        Err(errs)
    }
}

fn write_min_tree(root: &Path, cargo: &str, julia: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"readcon-core\"\nversion = \"{cargo}\"\n"),
    )
    .unwrap();
    fs::write(
        root.join("pyproject.toml"),
        format!(
            "[project]\nname = \"readcon\"\nversion = \"{cargo}\"\n\
             [project.optional-dependencies]\nchemfiles = [\"readcon-chemfiles=={cargo}\"]\n"
        ),
    )
    .unwrap();
    fs::write(
        root.join("pyproject.chemfiles.toml"),
        format!("[project]\nname = \"readcon-chemfiles\"\nversion = \"{cargo}\"\n"),
    )
    .unwrap();
    fs::write(
        root.join("meson.build"),
        format!("project(\n    'readcon-core',\n    version: '{cargo}',\n)\n"),
    )
    .unwrap();
    fs::write(
        root.join("pixi.toml"),
        format!("[workspace]\nname = \"readcon-core\"\nversion = \"{cargo}\"\n"),
    )
    .unwrap();
    fs::create_dir_all(root.join("julia/ReadCon")).unwrap();
    fs::write(
        root.join("julia/ReadCon/Project.toml"),
        format!("name = \"ReadCon\"\nversion = \"{julia}\"\n"),
    )
    .unwrap();
    fs::create_dir_all(root.join("fortran/ReadCon")).unwrap();
    fs::write(
        root.join("fortran/ReadCon/fpm.toml"),
        format!("name = \"ReadCon\"\nversion = \"{cargo}\"\n"),
    )
    .unwrap();
    fs::write(
        root.join("CITATION.cff"),
        format!("cff-version: 1.2.0\ntitle: readcon-core\nversion: {cargo}\n"),
    )
    .unwrap();
}

#[test]
fn extractors_read_each_binding_kind() {
    assert_eq!(
        toml_version("[package]\nversion = \"0.14.7\"\nrust-version = \"1.88\"\n").as_deref(),
        Some("0.14.7")
    );
    assert_eq!(
        meson_version("project(\n    version: '0.14.7',\n    meson_version: '>=1.3.0',\n)\n")
            .as_deref(),
        Some("0.14.7")
    );
    assert_eq!(
        cff_version("cff-version: 1.2.0\nversion: 0.14.7\n").as_deref(),
        Some("0.14.7")
    );
    assert_eq!(
        chemfiles_extra_pin("chemfiles = [\"readcon-chemfiles==0.14.7\"]\n").as_deref(),
        Some("0.14.7")
    );
}

#[test]
fn julia_project_toml_mismatch_fails() {
    let dir = tempfile::tempdir().unwrap();
    write_min_tree(dir.path(), "0.14.7", "0.13.1");
    let errs = check_tree(dir.path()).expect_err("julia lag must fail the check");
    assert!(
        errs.iter().any(|e| e.contains("julia/ReadCon/Project.toml")
            && e.contains("0.13.1")
            && e.contains("0.14.7")),
        "expected julia != cargo mismatch, got {errs:?}"
    );
    assert_eq!(
        errs.len(),
        1,
        "only julia should drift in this fixture: {errs:?}"
    );
}

#[test]
fn matching_binding_tree_passes() {
    let dir = tempfile::tempdir().unwrap();
    write_min_tree(dir.path(), "0.14.7", "0.14.7");
    let cargo = check_tree(dir.path()).expect("lockstep fixture must pass");
    assert_eq!(cargo, "0.14.7");
}

fn version_script() -> PathBuf {
    repo_root().join("scripts/check_version_consistency.sh")
}

fn run_version_script(root: &Path) -> std::process::Output {
    let script = version_script();
    assert!(script.is_file(), "missing {}", script.display());
    Command::new("bash")
        .arg(&script)
        .env("READCON_VERSION_ROOT", root)
        .output()
        .expect("failed to spawn scripts/check_version_consistency.sh")
}

#[test]
fn repo_bindings_match_cargo() {
    match check_tree(&repo_root()) {
        Ok(cargo) => assert!(
            !cargo.is_empty(),
            "Cargo.toml package version must be non-empty"
        ),
        Err(errs) => panic!(
            "binding versions drifted from Cargo.toml:\n{}",
            errs.join("\n")
        ),
    }
}

#[test]
fn script_repo_bindings_match_cargo() {
    let out = run_version_script(&repo_root());
    assert!(
        out.status.success(),
        "scripts/check_version_consistency.sh failed (exit {:?}):\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn script_julia_project_toml_mismatch_fails() {
    let dir = tempfile::tempdir().unwrap();
    write_min_tree(dir.path(), "0.14.7", "0.13.1");
    let out = run_version_script(dir.path());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "script must fail when julia/ReadCon/Project.toml != Cargo.toml"
    );
    assert!(
        stderr.contains("julia/ReadCon/Project.toml")
            && stderr.contains("0.13.1")
            && stderr.contains("0.14.7"),
        "expected julia != cargo on stderr, got {stderr}"
    );
}
