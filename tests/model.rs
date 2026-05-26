use ergokeygen::{
    charset_by_name, generate, generate_stream, score_sequence, DedupeMode, GenerateOptions,
    GenerationEngine, Layout, PreferHand, RhythmMode, Settings, Weights,
};

fn left_settings() -> Settings {
    Settings {
        prefer_hand: PreferHand::Left,
        mode: RhythmMode::OneHand,
        weights: Weights::default(),
    }
}

fn lower_options(min_len: usize, max_len: usize, limit: usize, beam: usize) -> GenerateOptions {
    GenerateOptions {
        min_len,
        max_len,
        limit,
        beam,
        charset: charset_by_name("lower").unwrap(),
        max_avg_cost: None,
        max_total_cost: None,
        dedupe: DedupeMode::Off,
        engine: GenerationEngine::Reference,
        parallel: false,
        parallel_threshold: usize::MAX,
    }
}

fn avg(sequence: &str) -> f64 {
    let layout = Layout::ansi_qwerty();
    score_sequence(&layout, &left_settings(), sequence)
        .unwrap()
        .average
}

#[test]
fn asdf_is_best_short_left_home_sweep() {
    let layout = Layout::ansi_qwerty();
    let generated = generate(&layout, &left_settings(), &lower_options(4, 4, 10, 4096)).unwrap();
    assert_eq!(generated[0].text, "asdf");
}

#[test]
fn repeated_key_is_worse_than_adjacent_finger_rhythm() {
    assert!(avg("asdf") < avg("fddf"));
    assert!(avg("asdf") < avg("dffd"));
    assert!(avg("fdsa") < avg("fddf"));
    assert!(avg("fd") < avg("ff"));
    assert!(avg("sd") < avg("dd"));
}

#[test]
fn finger_inertia_pushes_bounce_out_of_top_candidates() {
    let layout = Layout::ansi_qwerty();
    let generated = generate(&layout, &left_settings(), &lower_options(4, 4, 30, 4096)).unwrap();
    assert!(!generated.iter().any(|item| item.text == "fddf"));
    assert!(!generated.iter().any(|item| item.text == "dffd"));
}

#[test]
fn same_finger_travel_is_expensive() {
    assert!(avg("frfr") > avg("asdf"));
    assert!(avg("fr") > avg("fd"));
}

#[test]
fn finger_recovery_is_timing_based() {
    let layout = Layout::ansi_qwerty();
    let settings = left_settings();
    let recovered = score_sequence(&layout, &settings, "asdfa").unwrap();
    let rushed = score_sequence(&layout, &settings, "afa").unwrap();
    assert_eq!(recovered.steps.last().unwrap().timing_wait, 0.0);
    assert!(rushed.steps.last().unwrap().timing_wait > 0.5);
    assert!(avg("asdfasdf") < avg("fdfdfdfd"));
    assert!(avg("asdfasdf") < avg("ffff"));
}

#[test]
fn mixed_row_motor_program_is_worse_than_uniform_sweep() {
    assert!(avg("asdf") < avg("awdf"));
    assert!(avg("qwer") < avg("awdf"));
    assert!(avg("asdfasdf") < avg("awdfawdf"));
}

#[test]
fn long_gap_adjacent_roll_does_not_fake_a_sweep() {
    assert!(avg("asdf") < avg("asdg"));
    assert!(avg("qwer") < avg("asdg"));
}

#[test]
fn forward_sweep_beats_reverse_sweep() {
    assert!(avg("asdf") < avg("fdsa"));
    assert!(avg("qwer") < avg("rewq"));
}

#[test]
fn prefix_direction_should_align_with_following_sweep() {
    assert!(avg("dfasdfasdf") < avg("fdasdfasdf"));
    let layout = Layout::ansi_qwerty();
    let report = score_sequence(&layout, &left_settings(), "fdasdf").unwrap();
    assert!(report
        .steps
        .iter()
        .any(|step| step.flags.contains(&"pair-direction-change")));
}

#[test]
fn fewaq_variants_are_fdsa_like_split_reverse_sweeps() {
    assert!(avg("fewa") < avg("awdf"));
    assert!(avg("fewq") < avg("awdf") + 0.05);
    assert!(avg("fewas") < avg("awdf"));
    let layout = Layout::ansi_qwerty();
    let report = score_sequence(&layout, &left_settings(), "fewa").unwrap();
    assert!(report
        .steps
        .iter()
        .any(|step| step.flags.contains(&"upper-reverse-axis-relief")));
    assert!(report
        .steps
        .iter()
        .any(|step| step.flags.contains(&"upper-reverse-split-sweep")));
}

#[test]
fn index_stretch_keys_are_expensive() {
    assert!(avg("asdg") > avg("asdf"));
    assert!(avg("asdfb") > avg("asdfv"));
}

#[test]
fn bottom_row_lock_prefers_bottom_continuation_over_top_jump() {
    assert!(avg("zxcvz") < avg("zxcvq"));
    assert!(avg("zxcvc") < avg("zxcvq"));
}

#[test]
fn onehand_mode_penalizes_cross_hand_switches() {
    assert!(avg("fj") > avg("fd"));
}

#[test]
fn shifted_rightward_reach_is_expensive() {
    assert!(avg("F") > avg("D"));
}

#[test]
fn fast_and_exact_dedupe_do_not_emit_duplicates_for_duplicate_input() {
    let layout = Layout::ansi_qwerty();
    let settings = left_settings();
    let mut options = lower_options(2, 2, 20, 128);
    options.charset = "aassddff".chars().collect();
    options.engine = GenerationEngine::FastV1;

    options.dedupe = DedupeMode::Off;
    let undeduped = generate(&layout, &settings, &options).unwrap();
    assert_eq!(undeduped.len(), 16);

    options.dedupe = DedupeMode::Exact;
    let exact = generate(&layout, &settings, &options).unwrap();
    assert_eq!(exact.len(), 16);

    let mut streamed = Vec::new();
    options.dedupe = DedupeMode::Fast;
    generate_stream(&layout, &settings, &options, |item| {
        streamed.push(item.text);
        Ok(())
    })
    .unwrap();
    streamed.sort();
    streamed.dedup();
    assert_eq!(streamed.len(), 16);
}
