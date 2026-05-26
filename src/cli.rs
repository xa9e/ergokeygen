use std::env;
use std::io::{self, BufWriter, Write};

use ergokeygen::{
    charset_by_name, generate, generate_stream, load_weights_config, score_sequence, DedupeMode,
    GenerateOptions, GenerationEngine, Hand, Layout, PreferHand, RhythmMode, Settings, Weights,
};

pub(crate) fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        print_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "gen" | "generate" => run_generate(&args[1..]),
        "score" => run_score(&args[1..]),
        "compare" => run_compare(&args[1..]),
        "layout" => run_layout(),
        other => Err(format!("unknown command: {other}")),
    }
}

fn run_generate(args: &[String]) -> Result<(), String> {
    let layout = Layout::ansi_qwerty();
    let mut settings = Settings {
        weights: Weights::default(),
        mode: RhythmMode::OneHand,
        prefer_hand: PreferHand::Any,
    };
    let mut options = GenerateOptions::default();
    let mut show_score = false;
    let mut stream = false;
    let mut config_path: Option<String> = None;
    let mut jobs: Option<usize> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--min" | "--min-len" => {
                options.min_len = parse_usize(next_value(args, &mut i)?, "min")?
            }
            "--max" | "--max-len" => {
                options.max_len = parse_usize(next_value(args, &mut i)?, "max")?
            }
            "--limit" => options.limit = parse_usize(next_value(args, &mut i)?, "limit")?,
            "--beam" => options.beam = parse_usize(next_value(args, &mut i)?, "beam")?,
            "--jobs" | "--threads" => {
                let parsed = parse_usize(next_value(args, &mut i)?, "jobs")?;
                if parsed == 0 {
                    return Err("jobs must be at least 1".to_string());
                }
                jobs = Some(parsed);
                options.parallel = true;
            }
            "--parallel" => options.parallel = true,
            "--single-thread" | "--no-parallel" => options.parallel = false,
            "--engine" => options.engine = GenerationEngine::parse(next_value(args, &mut i)?)?,
            "--reference" => {
                options.engine = GenerationEngine::Reference;
                options.parallel = false;
            }
            "--fast-v1" => options.engine = GenerationEngine::FastV1,
            "--parallel-threshold" => {
                options.parallel_threshold =
                    parse_usize(next_value(args, &mut i)?, "parallel-threshold")?
            }
            "--chars" | "--charset" => {
                options.charset = charset_by_name(next_value(args, &mut i)?)?
            }
            "--max-avg-cost" => {
                options.max_avg_cost = Some(parse_f64(next_value(args, &mut i)?, "max-avg-cost")?)
            }
            "--max-total-cost" => {
                options.max_total_cost =
                    Some(parse_f64(next_value(args, &mut i)?, "max-total-cost")?)
            }
            "--dedupe" => {
                options.dedupe = if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    DedupeMode::parse(next_value(args, &mut i)?)?
                } else {
                    DedupeMode::Fast
                };
            }
            "--dedupe-mode" => options.dedupe = DedupeMode::parse(next_value(args, &mut i)?)?,
            "--dedupe-exact" => options.dedupe = DedupeMode::Exact,
            "--no-dedupe" => options.dedupe = DedupeMode::Off,
            "--mode" | "--rhythm" => settings.mode = RhythmMode::parse(next_value(args, &mut i)?)?,
            "--prefer-hand" => settings.prefer_hand = PreferHand::parse(next_value(args, &mut i)?)?,
            "--config" => config_path = Some(next_value(args, &mut i)?.to_string()),
            "--show-score" => show_score = true,
            "--stream" => stream = true,
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            other => return Err(format!("unknown generate option: {other}")),
        }
        i += 1;
    }

    if let Some(path) = config_path {
        settings.weights = load_weights_config(&path, settings.weights.clone())?;
    }

    configure_rayon(jobs)?;

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    if stream {
        let mut emitted = 0usize;
        generate_stream(&layout, &settings, &options, |item| {
            let write_result = if show_score {
                writeln!(out, "{:.4}\t{:.4}\t{}", item.average, item.total, item.text)
            } else {
                writeln!(out, "{}", item.text)
            };
            write_result.map_err(|err| err.to_string())?;

            emitted += 1;
            if emitted.is_multiple_of(1024) {
                out.flush().map_err(|err| err.to_string())?;
            }
            Ok(())
        })?;
        out.flush().map_err(|err| err.to_string())?;
        return Ok(());
    }

    let generated = generate(&layout, &settings, &options)?;
    for item in generated {
        if show_score {
            writeln!(out, "{:.4}\t{:.4}\t{}", item.average, item.total, item.text)
                .map_err(|err| err.to_string())?;
        } else {
            writeln!(out, "{}", item.text).map_err(|err| err.to_string())?;
        }
    }
    out.flush().map_err(|err| err.to_string())?;

    Ok(())
}

fn run_score(args: &[String]) -> Result<(), String> {
    let layout = Layout::ansi_qwerty();
    let mut settings = Settings {
        weights: Weights::default(),
        mode: RhythmMode::OneHand,
        prefer_hand: PreferHand::Any,
    };
    let mut sequence: Option<String> = None;
    let mut explain = false;
    let mut config_path: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" | "--rhythm" => settings.mode = RhythmMode::parse(next_value(args, &mut i)?)?,
            "--prefer-hand" => settings.prefer_hand = PreferHand::parse(next_value(args, &mut i)?)?,
            "--config" => config_path = Some(next_value(args, &mut i)?.to_string()),
            "--explain" => explain = true,
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown score option: {value}"))
            }
            value => {
                if sequence.is_some() {
                    return Err("score accepts exactly one sequence".to_string());
                }
                sequence = Some(value.to_string());
            }
        }
        i += 1;
    }

    if let Some(path) = config_path {
        settings.weights = load_weights_config(&path, settings.weights.clone())?;
    }

    let sequence = sequence.ok_or_else(|| "missing sequence".to_string())?;
    let report = score_sequence(&layout, &settings, &sequence)?;

    println!("sequence: {}", report.sequence);
    println!("total:    {:.4}", report.total);
    println!("average:  {:.4}", report.average);
    if let Some(last) = report.steps.last() {
        println!("time:     {:.4}", last.press_time);
    }

    if explain {
        for (idx, step) in report.steps.iter().enumerate() {
            let flags = if step.flags.is_empty() {
                "-".to_string()
            } else {
                step.flags.join(",")
            };
            println!(
                "{:>3} '{}' total={:.4} static={:.4} dynamic={:.4} transition={:.4} rhythm={:.4} timing={:.4}/wait={:.4} cognitive={:.4} move_time={:.4} flags={}",
                idx + 1,
                step.typed,
                step.total,
                step.static_cost,
                step.dynamic_cost,
                step.transition_cost,
                step.rhythm_cost,
                step.timing_cost,
                step.timing_wait,
                step.cognitive_cost,
                step.movement_time,
                flags
            );
        }
    }

    Ok(())
}

fn run_compare(args: &[String]) -> Result<(), String> {
    let layout = Layout::ansi_qwerty();
    let mut settings = Settings {
        weights: Weights::default(),
        mode: RhythmMode::OneHand,
        prefer_hand: PreferHand::Any,
    };
    let mut sequences: Vec<String> = Vec::new();
    let mut config_path: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" | "--rhythm" => settings.mode = RhythmMode::parse(next_value(args, &mut i)?)?,
            "--prefer-hand" => settings.prefer_hand = PreferHand::parse(next_value(args, &mut i)?)?,
            "--config" => config_path = Some(next_value(args, &mut i)?.to_string()),
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown compare option: {value}"))
            }
            value => sequences.push(value.to_string()),
        }
        i += 1;
    }

    if sequences.len() != 2 {
        return Err("compare accepts exactly two sequences".to_string());
    }

    if let Some(path) = config_path {
        settings.weights = load_weights_config(&path, settings.weights.clone())?;
    }

    let left = score_sequence(&layout, &settings, &sequences[0])?;
    let right = score_sequence(&layout, &settings, &sequences[1])?;
    let winner = if left.average < right.average {
        &left.sequence
    } else {
        &right.sequence
    };

    println!(
        "left:  {} total={:.4} avg={:.4}",
        left.sequence, left.total, left.average
    );
    println!(
        "right: {} total={:.4} avg={:.4}",
        right.sequence, right.total, right.average
    );
    println!("winner: {}", winner);

    let totals = |steps: &[ergokeygen::StepCost]| -> (f64, f64, f64, f64, f64, f64, f64) {
        let mut static_cost = 0.0;
        let mut dynamic_cost = 0.0;
        let mut transition_cost = 0.0;
        let mut rhythm_cost = 0.0;
        let mut timing_cost = 0.0;
        let mut cognitive_cost = 0.0;
        for step in steps {
            static_cost += step.static_cost;
            dynamic_cost += step.dynamic_cost;
            transition_cost += step.transition_cost;
            rhythm_cost += step.rhythm_cost;
            timing_cost += step.timing_cost;
            cognitive_cost += step.cognitive_cost;
        }
        let time = steps.last().map(|step| step.press_time).unwrap_or(0.0);
        (
            static_cost,
            dynamic_cost,
            transition_cost,
            rhythm_cost,
            timing_cost,
            cognitive_cost,
            time,
        )
    };
    let l = totals(&left.steps);
    let r = totals(&right.steps);
    println!("delta left-right:");
    println!("  static     {:+.4}", l.0 - r.0);
    println!("  dynamic    {:+.4}", l.1 - r.1);
    println!("  transition {:+.4}", l.2 - r.2);
    println!("  rhythm     {:+.4}", l.3 - r.3);
    println!("  timing     {:+.4}", l.4 - r.4);
    println!("  cognitive  {:+.4}", l.5 - r.5);
    println!("  time       {:+.4}", l.6 - r.6);

    Ok(())
}

fn run_layout() -> Result<(), String> {
    let layout = Layout::ansi_qwerty();
    let chars = charset_by_name("full")?;

    for c in chars {
        if let Some(key) = layout.key(c) {
            let hand = match key.hand {
                Hand::Left => "left",
                Hand::Right => "right",
            };
            println!(
                "{:?}\tphysical={:?}\thand={}\tfinger={:?}\tx={:.2}\ty={:.2}\tshifted={}",
                key.typed, key.physical, hand, key.finger, key.x, key.y, key.shifted
            );
        }
    }

    Ok(())
}

fn configure_rayon(jobs: Option<usize>) -> Result<(), String> {
    if let Some(num_threads) = jobs {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()
            .map_err(|err| format!("failed to configure Rayon thread pool: {err}"))?;
    }
    Ok(())
}

fn next_value<'a>(args: &'a [String], index: &mut usize) -> Result<&'a str, String> {
    *index += 1;
    args.get(*index)
        .map(String::as_str)
        .ok_or_else(|| "missing option value".to_string())
}

fn parse_usize(raw: &str, name: &str) -> Result<usize, String> {
    raw.parse::<usize>()
        .map_err(|_| format!("invalid {name}: {raw}"))
}

fn parse_f64(raw: &str, name: &str) -> Result<f64, String> {
    raw.parse::<f64>()
        .map_err(|_| format!("invalid {name}: {raw}"))
}

mod usage;

pub(crate) use usage::print_usage;
