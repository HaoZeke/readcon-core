//! Binding package versions stay in lockstep with Cargo.toml.
//!
//! `Cargo.toml` `[package].version` is canonical. Every language binding
//! reports the same version unless it is named in `INTENTIONAL_LAG` with a
//! comment that states why that binding is lagged.

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_repo(rel: &str) -> String {
    let p = repo_root().join(rel);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("missing {}: {e}", p.display()))
}

/// Labels that may legally differ from Cargo.toml package.version.
/// Empty: no language binding is intentionally lagged.
///
/// `packaging/wrapdb/readcon-core.wrap` is excluded (not listed in
/// `BINDINGS`) because it pins a published cxx tarball URL and hash;
/// that file updates after a GitHub Release, not on every crate bump.
const INTENTIONAL_LAG: &[&str] = &[];

#[derive(Clone, Copy)]
enum VersionKind {
    /// First `version = "x.y.z"` (skips `rust-version`).
    TomlQuoted,
    /// First `version: 'x.y.z'` (skips `meson_version`).
    MesonQuoted,
    /// First `version: x.y.z` (skips `cff-version`).
    CffBare,
    /// First `release = "x.y.z"`.
    PyRelease,
    /// `readcon-chemfiles==x.y.z` extra pin.
    ChemfilesExtra,
    /// `assert_eq!(VERSION, "x.y.z")` in `src/lib.rs`.
    LibRsAssert,
}

struct Binding {
    label: &'static str,
    path: &'static str,
    kind: VersionKind,
}

/// Every language binding plus the published stamps release-prep bumps.
const BINDINGS: &[Binding] = &[
    Binding {
        label: "pyproject.toml [project]",
        path: "pyproject.toml",
        kind: VersionKind::TomlQuoted,
    },
    Binding {
        label: "pyproject.toml chemfiles extra",
        path: "pyproject.toml",
        kind: VersionKind::ChemfilesExtra,
    },
    Binding {
        label: "pyproject.chemfiles.toml",
        path: "pyproject.chemfiles.toml",
        kind: VersionKind::TomlQuoted,
    },
    Binding {
        label: "meson.build",
        path: "meson.build",
        kind: VersionKind::MesonQuoted,
    },
    Binding {
        label: "pixi.toml",
        path: "pixi.toml",
        kind: VersionKind::TomlQuoted,
    },
    Binding {
        label: "julia/ReadCon/Project.toml",
        path: "julia/ReadCon/Project.toml",
        kind: VersionKind::TomlQuoted,
    },
    Binding {
        label: "fortran/ReadCon/fpm.toml",
        path: "fortran/ReadCon/fpm.toml",
        kind: VersionKind::TomlQuoted,
    },
    Binding {
        label: "CITATION.cff",
        path: "CITATION.cff",
        kind: VersionKind::CffBare,
    },
    Binding {
        label: "docs/source/conf.py",
        path: "docs/source/conf.py",
        kind: VersionKind::PyRelease,
    },
    Binding {
        label: "src/lib.rs VERSION assert",
        path: "src/lib.rs",
        kind: VersionKind::LibRsAssert,
    },
];

fn quoted_after<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = line.trim().strip_prefix(prefix)?;
    let rest = rest.trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    rest[1..].split(quote).next()
}

fn toml_quoted_version(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(v) = quoted_after(line, "version =") {
            return Some(v.to_string());
        }
    }
    None
}

fn meson_project_version(text: &str) -> Option<String> {
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("meson_version") {
            continue;
        }
        if let Some(v) = quoted_after(t, "version:") {
            return Some(v.to_string());
        }
    }
    None
}

fn cff_software_version(text: &str) -> Option<String> {
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("cff-version") {
            continue;
        }
        if let Some(rest) = t.strip_prefix("version:") {
            let v = rest.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn python_release(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(v) = quoted_after(line, "release =") {
            return Some(v.to_string());
        }
    }
    None
}

fn chemfiles_extra_pin(text: &str) -> Option<String> {
    const MARK: &str = "readcon-chemfiles==";
    let rest = text.split(MARK).nth(1)?;
    let ver: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if ver.is_empty() { None } else { Some(ver) }
}

fn lib_rs_version_assert(text: &str) -> Option<String> {
    const MARK: &str = "assert_eq!(VERSION, \"";
    let rest = text.split(MARK).nth(1)?;
    rest.split('"').next().map(str::to_string)
}

fn extract(kind: VersionKind, text: &str) -> Option<String> {
    match kind {
        VersionKind::TomlQuoted => toml_quoted_version(text),
        VersionKind::MesonQuoted => meson_project_version(text),
        VersionKind::CffBare => cff_software_version(text),
        VersionKind::PyRelease => python_release(text),
        VersionKind::ChemfilesExtra => chemfiles_extra_pin(text),
        VersionKind::LibRsAssert => lib_rs_version_assert(text),
    }
}

fn lockstep_mismatches(canonical: &str, found: &[(&str, String)], allow: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for (label, ver) in found {
        if allow.contains(label) {
            continue;
        }
        if ver.as_str() != canonical {
            out.push(format!("{label}: {ver} != Cargo.toml {canonical}"));
        }
    }
    out
}

#[test]
fn extractors_skip_adjacent_version_keys() {
    assert_eq!(
        toml_quoted_version("rust-version = \"1.88\"\nversion = \"0.14.7\"\n"),
        Some("0.14.7".into())
    );
    assert_eq!(
        meson_project_version("meson_version: '>=1.3.0',\n    version: '0.14.7',\n"),
        Some("0.14.7".into())
    );
    assert_eq!(
        cff_software_version("cff-version: 1.2.0\nversion: 0.14.7\n"),
        Some("0.14.7".into())
    );
}

/// Regression: a lagged Julia package version must fail the lockstep check.
#[test]
fn julia_project_toml_mismatch_fails_the_check() {
    let cargo = read_repo("Cargo.toml");
    let canonical = toml_quoted_version(&cargo).expect("Cargo.toml [package] version");
    let live = read_repo("julia/ReadCon/Project.toml");
    let live_ver = toml_quoted_version(&live).expect("julia/ReadCon/Project.toml version");
    let lagged = if live_ver == canonical {
        live.replace(
            &format!("version = \"{canonical}\""),
            "version = \"0.13.1\"",
        )
    } else {
        live
    };
    let found = toml_quoted_version(&lagged).expect("lagged julia version");
    assert_ne!(
        found, canonical,
        "fixture must differ from Cargo.toml package.version"
    );
    let mismatches = lockstep_mismatches(&canonical, &[("julia/ReadCon/Project.toml", found)], &[]);
    assert!(
        !mismatches.is_empty(),
        "check must fail when julia/ReadCon/Project.toml != Cargo.toml package.version"
    );
    assert!(
        mismatches[0].contains("julia/ReadCon/Project.toml"),
        "mismatch names the Julia manifest: {}",
        mismatches[0]
    );
}

#[test]
fn allowlist_skips_named_lag() {
    let mismatches = lockstep_mismatches(
        "0.14.7",
        &[("julia/ReadCon/Project.toml", "0.13.1".into())],
        &["julia/ReadCon/Project.toml"],
    );
    assert!(
        mismatches.is_empty(),
        "INTENTIONAL_LAG must suppress a named binding"
    );
}

#[test]
fn binding_manifests_match_cargo_package_version() {
    let cargo = read_repo("Cargo.toml");
    let canonical = toml_quoted_version(&cargo).expect("Cargo.toml [package] version");
    assert!(
        !canonical.is_empty(),
        "Cargo.toml package.version must be non-empty"
    );

    let mut found = Vec::new();
    for b in BINDINGS {
        let text = read_repo(b.path);
        let ver = extract(b.kind, &text)
            .unwrap_or_else(|| panic!("no version in {} ({})", b.path, b.label));
        found.push((b.label, ver));
    }

    let mismatches = lockstep_mismatches(&canonical, &found, INTENTIONAL_LAG);
    assert!(
        mismatches.is_empty(),
        "binding versions must match Cargo.toml {canonical}; drifted:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn julia_project_toml_matches_cargo_package_version() {
    let cargo =
        toml_quoted_version(&read_repo("Cargo.toml")).expect("Cargo.toml [package] version");
    let julia = toml_quoted_version(&read_repo("julia/ReadCon/Project.toml"))
        .expect("julia/ReadCon/Project.toml version");
    assert_eq!(
        julia, cargo,
        "julia/ReadCon/Project.toml version must equal Cargo.toml package.version"
    );
}
