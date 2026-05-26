use std::process::{Command, Output};

fn ergokeygen(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ergokeygen"))
        .args(args)
        .output()
        .expect("run ergokeygen")
}

fn stdout(output: Output) -> String {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("stdout is utf8")
}

#[test]
fn cli_generation_is_unlimited_by_default() {
    let output = stdout(ergokeygen(&[
        "gen", "--min", "1", "--max", "2", "--beam", "16", "--chars", "ab",
    ]));

    assert_eq!(output.lines().count(), 6);
}

#[test]
fn cli_generation_limit_still_caps_output() {
    let output = stdout(ergokeygen(&[
        "gen", "--min", "1", "--max", "2", "--limit", "3", "--beam", "16", "--chars", "ab",
    ]));

    assert_eq!(output.lines().count(), 3);
}

#[test]
fn cli_fast_generation_handles_lengths_beyond_tail_buffer() {
    let output = stdout(ergokeygen(&[
        "gen", "--min", "4", "--max", "10", "--limit", "20", "--beam", "512", "--chars", "lower",
    ]));

    assert_eq!(output.lines().count(), 20);
}

#[test]
fn cli_fast_dedupe_uses_full_text_for_long_sequences() {
    let output = stdout(ergokeygen(&[
        "gen", "--min", "9", "--max", "9", "--beam", "512", "--chars", "ab", "--dedupe", "fast",
    ]));

    assert_eq!(output.lines().count(), 512);
}

#[test]
fn cli_fast_generation_matches_reference_for_small_contract() {
    let common = [
        "gen",
        "--min",
        "4",
        "--max",
        "4",
        "--limit",
        "24",
        "--beam",
        "1024",
        "--chars",
        "lower",
        "--prefer-hand",
        "left",
        "--mode",
        "onehand",
        "--show-score",
    ];

    let mut reference_args = common.to_vec();
    reference_args.extend(["--engine", "reference", "--single-thread"]);
    let reference = stdout(ergokeygen(&reference_args));

    let mut fast_args = common.to_vec();
    fast_args.extend(["--engine", "fast-v1", "--parallel-threshold", "1"]);
    let fast = stdout(ergokeygen(&fast_args));

    assert_eq!(fast, reference);
}

#[test]
fn cli_config_profile_changes_scoring_output() {
    let base = stdout(ergokeygen(&[
        "score",
        "awdf",
        "--prefer-hand",
        "left",
        "--mode",
        "onehand",
    ]));
    let configured = stdout(ergokeygen(&[
        "score",
        "awdf",
        "--prefer-hand",
        "left",
        "--mode",
        "onehand",
        "--config",
        "profiles/left-ring-strict.ekg",
    ]));

    assert!(configured.contains("sequence: awdf"));
    assert_ne!(configured, base);
}

#[test]
fn cli_rejects_invalid_thread_count() {
    let output = ergokeygen(&["gen", "--jobs", "0"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("jobs must be at least 1"));
}
