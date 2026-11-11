// Copyright (c) readcon-core developers
//! The `readcon-core` binary: inspect, rewrite, and convert.

use std::process::Command;

fn bin() -> Command {
    let exe = std::env::current_exe().unwrap();
    let mut dir = exe.parent().unwrap().to_path_buf();
    loop {
        let candidate = dir.join("readcon-core");
        if candidate.is_file() {
            return Command::new(candidate);
        }
        assert!(dir.pop(), "missing binary above {}", exe.display());
    }
}

fn output_text(out: &std::process::Output) -> (String, String) {
    (
        String::from_utf8(out.stdout.clone()).unwrap(),
        String::from_utf8(out.stderr.clone()).unwrap(),
    )
}

#[test]
fn usage_errors_name_the_missing_piece() {
    let none = bin().output().unwrap();
    assert_eq!(none.status.code(), Some(2));
    let (_, err) = output_text(&none);
    assert!(err.contains("Usage:"), "{err}");

    let help = bin().arg("--help").output().unwrap();
    assert!(help.status.success());
    let (_, err) = output_text(&help);
    assert!(err.contains("convert"), "{err}");

    let short = bin().arg("-h").output().unwrap();
    assert!(short.status.success());
    assert!(output_text(&short).1.contains("Usage:"));

    let extra = bin().args(["a.con", "b.con", "c.con"]).output().unwrap();
    assert_eq!(extra.status.code(), Some(2));

    let missing = bin().arg("no-such-readcon.con").output().unwrap();
    assert_eq!(missing.status.code(), Some(1));
    assert!(output_text(&missing).1.contains("not found"));

    let convert_short = bin().args(["convert", "only-one"]).output().unwrap();
    assert_eq!(convert_short.status.code(), Some(2));
    assert!(output_text(&convert_short).1.contains("convert"));
}

#[test]
fn inspect_rewrite_and_a_full_device_are_reported() {
    let dir = std::env::temp_dir().join(format!("readcon-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir(&dir).unwrap();
    let out_path = dir.join("out.con");
    let inspect = bin()
        .args(["resources/test/tiny_cuh2.con", out_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(inspect.status.success(), "{}", output_text(&inspect).1);
    let (stdout, _) = output_text(&inspect);
    assert!(stdout.contains("Successfully parsed 1 valid frames."), "{stdout}");
    assert!(stdout.contains("Last atom:"), "{stdout}");
    assert!(stdout.contains("Successfully wrote"), "{stdout}");
    let written = std::fs::read_to_string(&out_path).unwrap();
    assert!(written.contains("con_spec_version"), "{written}");

    let bare = bin().arg("resources/test/tiny_cuh2.con").output().unwrap();
    assert!(bare.status.success());
    assert!(output_text(&bare).0.contains("Total atoms:"));

    let blocked = bin()
        .args(["resources/test/tiny_cuh2.con", dir.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(blocked.status.code(), Some(1));
    assert!(output_text(&blocked).1.contains("Failed to create"));

    let full = bin()
        .args(["resources/test/tiny_cuh2.con", "/dev/full"])
        .output()
        .unwrap();
    assert_eq!(full.status.code(), Some(1), "{}", output_text(&full).0);
    assert!(output_text(&full).1.contains("Error writing"));

    let wide = dir.join("wide.con");
    let one = std::fs::read("resources/test/tiny_cuh2.con").unwrap();
    let mut many = Vec::new();
    while many.len() < 64 * 1024 {
        many.extend_from_slice(&one);
    }
    std::fs::write(&wide, &many).unwrap();
    let full_wide = bin()
        .args([wide.to_str().unwrap(), "/dev/full"])
        .output()
        .unwrap();
    assert_eq!(full_wide.status.code(), Some(1));
    assert!(output_text(&full_wide).1.contains("Error writing"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn convert_reads_native_con_and_refuses_xyz_without_chemfiles() {
    let dir = std::env::temp_dir().join(format!("readcon-cli-convert-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir(&dir).unwrap();
    let out_path = dir.join("out.con");
    let native = bin()
        .args([
            "convert",
            "resources/test/tiny_multi_cuh2.con",
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(native.status.success(), "{}", output_text(&native).1);
    let (stdout, _) = output_text(&native);
    assert!(stdout.contains("native CON"), "{stdout}");
    assert!(stdout.contains("2 frame"), "{stdout}");

    let foreign = bin()
        .args([
            "convert",
            "resources/test/water_min.xyz",
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let (_, foreign_err) = output_text(&foreign);
    if cfg!(feature = "chemfiles") {
        assert!(
            foreign.status.success(),
            "xyz convert failed: {foreign_err}"
        );
        let written = std::fs::read_to_string(&out_path).unwrap();
        assert!(written.contains("H2O") || written.contains("O"), "{written}");
    } else {
        assert_eq!(foreign.status.code(), Some(1));
        assert!(foreign_err.contains("Error:"), "{foreign_err}");
    }

    let missing = dir.join("no-such.xyz");
    let failed = bin()
        .args([
            "convert",
            missing.to_str().unwrap(),
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(failed.status.code(), Some(1));
    assert!(output_text(&failed).1.contains("Error:"), "{:?}", output_text(&failed));

    let empty = dir.join("empty.con");
    std::fs::write(&empty, b"").unwrap();
    let no_frames = bin().arg(&empty).output().unwrap();
    assert_eq!(no_frames.status.code(), Some(1));
    assert!(output_text(&no_frames).1.contains("No valid frames"));

    let truncated = dir.join("cut.con");
    let bytes = std::fs::read("resources/test/tiny_cuh2.con").unwrap();
    std::fs::write(&truncated, &bytes[..40]).unwrap();
    let cut = bin().arg(&truncated).output().unwrap();
    assert_eq!(cut.status.code(), Some(1));
    let (_, err) = output_text(&cut);
    assert!(
        err.contains("Discarding an incomplete frame") || err.contains("No valid frames"),
        "{err}"
    );

    let zero = dir.join("zero.con");
    std::fs::write(
        &zero,
        "user\n{\"con_spec_version\":2}\n10.0 10.0 10.0\n90.0 90.0 90.0\n\n\n0\n\n\n",
    )
    .unwrap();
    let zero_out = bin().arg(&zero).output().unwrap();
    assert!(
        zero_out.status.success(),
        "{}",
        output_text(&zero_out).1
    );
    let (stdout, _) = output_text(&zero_out);
    assert!(stdout.contains("Total atoms: 0"), "{stdout}");
    assert!(!stdout.contains("Last atom:"), "{stdout}");

    std::fs::remove_dir_all(&dir).unwrap();
}
