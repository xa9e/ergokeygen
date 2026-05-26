use std::process::Command;

use ergokeygen::{
    charset_by_name, generate, generate_stream, score_sequence, DedupeMode, GenerateOptions,
    GeneratedSequence, GenerationEngine, Layout, PreferHand, RhythmMode, Settings, Weights,
};

fn left_onehand_settings() -> Settings {
    Settings {
        prefer_hand: PreferHand::Left,
        mode: RhythmMode::OneHand,
        weights: Weights::default(),
    }
}

fn lower_options(
    min_len: usize,
    max_len: usize,
    limit: usize,
    beam: usize,
    engine: GenerationEngine,
    parallel: bool,
) -> GenerateOptions {
    GenerateOptions {
        min_len,
        max_len,
        limit,
        beam,
        charset: charset_by_name("lower").unwrap(),
        max_avg_cost: None,
        max_total_cost: None,
        dedupe: DedupeMode::Off,
        engine,
        parallel,
        parallel_threshold: if parallel { 1 } else { usize::MAX },
    }
}

fn rounded_4(value: f64) -> i64 {
    (value * 10_000.0).round() as i64
}

fn assert_same_generated(left: &[GeneratedSequence], right: &[GeneratedSequence]) {
    assert_eq!(left.len(), right.len());
    for (idx, (left, right)) in left.iter().zip(right).enumerate() {
        assert_eq!(left.text, right.text, "text mismatch at index {idx}");
        assert_eq!(
            rounded_4(left.total),
            rounded_4(right.total),
            "total mismatch for {}",
            left.text
        );
        assert_eq!(
            rounded_4(left.average),
            rounded_4(right.average),
            "average mismatch for {}",
            left.text
        );
    }
}

#[test]
fn current_rust_len4_top32_contract() {
    let expected = [
        ("asdf", 0.7675, 3.0701),
        ("fsdf", 1.1119, 4.4474),
        ("fdsa", 1.1681, 4.6726),
        ("fewa", 1.2019, 4.8076),
        ("asef", 1.2349, 4.9396),
        ("qsdf", 1.2365, 4.9461),
        ("fasd", 1.2706, 5.0824),
        ("sdfs", 1.2900, 5.1599),
        ("fsef", 1.2954, 5.1814),
        ("sefd", 1.3169, 5.2678),
        ("esdf", 1.3222, 5.2887),
        ("fdsf", 1.3242, 5.2967),
        ("dsdf", 1.3252, 5.3009),
        ("fewq", 1.3538, 5.4153),
        ("dfsd", 1.3628, 5.4512),
        ("dasd", 1.3640, 5.4561),
        ("fsdr", 1.3691, 5.4766),
        ("qwer", 1.3762, 5.5048),
        ("sdfa", 1.3899, 5.5597),
        ("sefs", 1.4022, 5.6090),
        ("asdr", 1.4057, 5.6227),
        ("efds", 1.4295, 5.7179),
        ("fase", 1.4384, 5.7536),
        ("asdg", 1.4606, 5.8424),
        ("sdff", 1.4625, 5.8502),
        ("sfds", 1.4691, 5.8763),
        ("dfdf", 1.4805, 5.9219),
        ("fesd", 1.4933, 5.9731),
        ("fser", 1.5039, 6.0157),
        ("esef", 1.5080, 6.0320),
        ("sefe", 1.5118, 6.0471),
        ("easd", 1.5285, 6.1140),
    ];

    let layout = Layout::ansi_qwerty();
    let settings = left_onehand_settings();
    let options = lower_options(
        4,
        4,
        expected.len(),
        4096,
        GenerationEngine::Reference,
        false,
    );
    let generated = generate(&layout, &settings, &options).unwrap();

    assert_eq!(generated.len(), expected.len());
    for (item, (text, average, total)) in generated.iter().zip(expected) {
        assert_eq!(item.text, text);
        assert_eq!(rounded_4(item.average), rounded_4(average));
        assert_eq!(rounded_4(item.total), rounded_4(total));
    }
}

#[test]
fn parallel_generation_matches_single_thread_reference() {
    let layout = Layout::ansi_qwerty();
    let settings = left_onehand_settings();
    let reference = generate(
        &layout,
        &settings,
        &lower_options(4, 5, 80, 512, GenerationEngine::Reference, false),
    )
    .unwrap();
    let accelerated = generate(
        &layout,
        &settings,
        &lower_options(4, 5, 80, 512, GenerationEngine::FastV1, true),
    )
    .unwrap();

    assert_same_generated(&reference, &accelerated);
}

#[test]
fn fast_generation_matches_reference_beyond_tail_buffer() {
    let layout = Layout::ansi_qwerty();
    let settings = left_onehand_settings();
    let reference = generate(
        &layout,
        &settings,
        &lower_options(4, 10, 80, 512, GenerationEngine::Reference, false),
    )
    .unwrap();
    let accelerated = generate(
        &layout,
        &settings,
        &lower_options(4, 10, 80, 512, GenerationEngine::FastV1, true),
    )
    .unwrap();

    assert_same_generated(&reference, &accelerated);
}

#[test]
fn stream_generation_matches_batch_for_single_depth() {
    let layout = Layout::ansi_qwerty();
    let settings = left_onehand_settings();
    let options = lower_options(4, 4, 64, 1024, GenerationEngine::FastV1, true);
    let batch = generate(&layout, &settings, &options).unwrap();
    let mut streamed = Vec::new();

    generate_stream(&layout, &settings, &options, |item| {
        streamed.push(item);
        Ok(())
    })
    .unwrap();

    assert_same_generated(&batch, &streamed);
}

#[test]
fn score_contract_for_key_model_examples() {
    let layout = Layout::ansi_qwerty();
    let settings = left_onehand_settings();
    let cases = [
        ("asdf", 0.7675, 3.0701),
        ("fdsa", 1.1681, 4.6726),
        ("awdf", 1.5289, 6.1155),
        ("fewa", 1.2019, 4.8076),
        ("qwer", 1.3762, 5.5048),
        ("zxcv", 2.6859, 10.7436),
        ("asdfasdf", 0.9481, 7.5845),
        ("awdfawdf", 1.8641, 14.9130),
    ];

    for (sequence, average, total) in cases {
        let report = score_sequence(&layout, &settings, sequence).unwrap();
        assert_eq!(
            rounded_4(report.average),
            rounded_4(average),
            "average mismatch for {sequence}"
        );
        assert_eq!(
            rounded_4(report.total),
            rounded_4(total),
            "total mismatch for {sequence}"
        );
    }
}

#[test]
fn cli_score_output_contract() {
    let output = Command::new(env!("CARGO_BIN_EXE_ergokeygen"))
        .args([
            "score",
            "asdf",
            "--prefer-hand",
            "left",
            "--mode",
            "onehand",
        ])
        .output()
        .expect("run ergokeygen score");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("sequence: asdf"));
    assert!(stdout.contains("total:    3.0701"));
    assert!(stdout.contains("average:  0.7675"));
}
