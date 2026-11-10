//! CPU model string used when a bench records one.
//!
//! The value is the `model name` line from `/proc/cpuinfo`, with the
//! trademark marks removed. A Xeon line is the plain name the records use.

use std::fs;
use std::path::Path;

/// The hardware string the records write.
///
/// The kernel prints trademark marks. The records use one plain name.
pub fn normalize_cpu(raw: &str) -> String {
    let cleaned = raw.replace("(R)", "").replace("(TM)", "").replace('®', " ");
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.contains("Xeon") {
        return "Intel Xeon Processor".to_string();
    }
    if cleaned.is_empty() {
        "unknown".to_string()
    } else {
        cleaned
    }
}

/// CPU model from `path`, or `unknown` when the file or the line is absent.
pub fn cpu_model_at(path: &Path) -> String {
    let Ok(text) = fs::read_to_string(path) else {
        return "unknown".to_string();
    };
    for line in text.lines() {
        if line.to_ascii_lowercase().starts_with("model name") {
            if let Some((_, value)) = line.split_once(':') {
                let value = value.trim();
                if !value.is_empty() {
                    return normalize_cpu(value);
                }
            }
        }
    }
    "unknown".to_string()
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
            super::normalize_cpu("Intel(R) Xeon(R) Processor"),
            "Intel Xeon Processor"
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
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn cpu_model_is_not_a_hostname_token() {
        let cpu = cpu_model();
        assert!(!cpu.is_empty());
        match fs::read_to_string("/etc/hostname") {
            Ok(text) => {
                let node = text.trim();
                if !node.is_empty() {
                    assert!(cpu.split('-').all(|token| token != node));
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => panic!("reading /etc/hostname failed: {err}"),
        }
    }
}
