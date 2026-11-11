// Copyright (c) readcon-core developers
//! CPU model string used when a bench records one.
//!
//! The value is the `model name` line from `/proc/cpuinfo`, with the
//! trademark marks removed, plus the kernel family, model, and stepping.

use std::fs;
use std::path::Path;

/// Drop trademark marks and collapse whitespace. The model name stays.
pub fn normalize_cpu(raw: &str) -> String {
    let cleaned = raw.replace("(R)", "").replace("(TM)", "").replace('®', " ");
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        "unknown".to_string()
    } else {
        cleaned
    }
}

/// Model name plus `family`, `model`, and `stepping` when those fields exist.
pub fn cpu_record(model: &str, family: &str, model_id: &str, stepping: &str) -> String {
    let name = normalize_cpu(model);
    let mut parts = Vec::new();
    if !family.is_empty() {
        parts.push(format!("family {family}"));
    }
    if !model_id.is_empty() {
        parts.push(format!("model {model_id}"));
    }
    if !stepping.is_empty() {
        parts.push(format!("stepping {stepping}"));
    }
    if parts.is_empty() {
        name
    } else if name == "unknown" {
        parts.join(" ")
    } else {
        format!("{name} {}", parts.join(" "))
    }
}

fn field<'a>(text: &'a str, key: &str) -> &'a str {
    for line in text.lines() {
        let Some((left, value)) = line.split_once(':') else {
            continue;
        };
        if left.trim().eq_ignore_ascii_case(key) {
            return value.trim();
        }
    }
    ""
}

/// CPU model from `path`, or `unknown` when the file or the line is absent.
pub fn cpu_model_at(path: &Path) -> String {
    let Ok(text) = fs::read_to_string(path) else {
        return "unknown".to_string();
    };
    let model = field(&text, "model name");
    let family = field(&text, "cpu family");
    let model_id = field(&text, "model");
    let stepping = field(&text, "stepping");
    if model.is_empty() && family.is_empty() && model_id.is_empty() && stepping.is_empty() {
        return "unknown".to_string();
    }
    cpu_record(model, family, model_id, stepping)
}

/// CPU model from `/proc/cpuinfo`, or `unknown` when that line is absent.
pub fn cpu_model() -> String {
    cpu_model_at(Path::new("/proc/cpuinfo"))
}

#[cfg(test)]
mod tests {
    use super::cpu_model;
    use std::fs;

    #[test]
    fn cpu_model_value() {
        println!("CPU_MODEL={}", cpu_model());
    }

    #[test]
    fn normalize_cpu_matches_the_record_name() {
        assert_eq!(
            super::normalize_cpu("Intel(R) Xeon(R) Platinum 8480+"),
            "Intel Xeon Platinum 8480+"
        );
        assert_eq!(
            super::cpu_record("Intel Xeon Platinum 8480+", "6", "143", "8"),
            "Intel Xeon Platinum 8480+ family 6 model 143 stepping 8"
        );
        assert_eq!(super::normalize_cpu("AMD EPYC 7R13"), "AMD EPYC 7R13");
        assert_eq!(super::normalize_cpu("  "), "unknown");
        let dir = std::env::temp_dir();
        let missing = dir.join("readcon-no-such-cpuinfo");
        assert_eq!(super::cpu_model_at(&missing), "unknown");
        let path = dir.join("readcon-cpuinfo-blank");
        fs::write(&path, "model name\t:   \n").unwrap();
        assert_eq!(super::cpu_model_at(&path), "unknown");
        fs::write(&path, "processor\t: 0\n").unwrap();
        assert_eq!(super::cpu_model_at(&path), "unknown");
        fs::write(&path, "model name\t: AMD EPYC 7R13\n").unwrap();
        assert_eq!(super::cpu_model_at(&path), "AMD EPYC 7R13");
        fs::write(&path, "cpu family\t: 6\n").unwrap();
        assert_eq!(super::cpu_model_at(&path), "family 6");
        fs::write(&path, "model\t: 143\n").unwrap();
        assert_eq!(super::cpu_model_at(&path), "model 143");
        fs::write(&path, "stepping\t: 8\n").unwrap();
        assert_eq!(super::cpu_model_at(&path), "stepping 8");
        fs::write(&path, "not a field\nmodel name\t: AMD EPYC 7R13\n").unwrap();
        assert_eq!(super::cpu_model_at(&path), "AMD EPYC 7R13");
        assert_eq!(
            super::cpu_record("", "6", "207", ""),
            "family 6 model 207"
        );
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn cpu_model_is_not_a_hostname_token() {
        let cpu = cpu_model();
        assert!(!cpu.is_empty());
        let missing = std::env::temp_dir().join("readcon-hostname-missing");
        let _ = fs::remove_file(&missing);
        let dir = std::env::temp_dir().join("readcon-hostname-dir");
        let _ = fs::create_dir(&dir);
        let empty = std::env::temp_dir().join("readcon-hostname-empty");
        fs::write(&empty, "\n").unwrap();
        let host = fs::read_to_string("/etc/hostname").unwrap();
        let node = host.trim();
        assert!(!node.is_empty());
        assert!(cpu.split('-').all(|token| token != node));
        let missing_err = fs::read_to_string(&missing).unwrap_err();
        assert_eq!(missing_err.kind(), std::io::ErrorKind::NotFound);
        let dir_err = fs::read_to_string(&dir).unwrap_err();
        assert_eq!(dir_err.kind(), std::io::ErrorKind::IsADirectory);
        let empty_text = fs::read_to_string(&empty).unwrap();
        assert!(empty_text.trim().is_empty());
    }
}
