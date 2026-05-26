mod dedupe;
mod options;
mod reference;

pub use options::{DedupeMode, GenerateOptions, GeneratedSequence, GenerationEngine};

use crate::charset::sanitize_charset_as_keys;
use crate::fast::{generate_fast, generate_stream_fast};
use crate::*;

pub(crate) use dedupe::DedupeFilter;
pub(crate) use reference::{compare_generated, retain_best_generated};
use reference::{expand_next_states, retain_best_states};

pub fn generate(
    layout: &Layout,
    settings: &Settings,
    options: &GenerateOptions,
) -> Result<Vec<GeneratedSequence>, String> {
    if options.min_len == 0 || options.max_len < options.min_len {
        return Err("invalid length range".to_string());
    }
    if options.limit == 0 || options.beam == 0 {
        return Ok(Vec::new());
    }

    let keys = sanitize_charset_as_keys(layout, &options.charset);
    if keys.is_empty() {
        return Err("charset has no supported chars".to_string());
    }

    if options.engine == GenerationEngine::FastV1 {
        return generate_fast(layout, settings, options, &keys);
    }

    let len_count = options.max_len.saturating_sub(options.min_len) + 1;
    let expected = options.beam.saturating_mul(len_count);
    let mut states = vec![TypingState::new(layout)];
    let mut results: Vec<GeneratedSequence> = Vec::with_capacity(options.limit.min(expected));
    let mut dedupe = DedupeFilter::new(options.dedupe);

    for depth in 1..=options.max_len {
        let mut next_states = expand_next_states(&states, &keys, layout, settings, options);

        retain_best_states(&mut next_states, options.beam);
        states = next_states;

        if depth >= options.min_len {
            for state in &states {
                let average = state.average();
                if options.max_avg_cost.is_some_and(|max| average > max) {
                    continue;
                }
                if options.max_total_cost.is_some_and(|max| state.score > max) {
                    continue;
                }
                if !dedupe.accept(&state.text) {
                    continue;
                }
                results.push(GeneratedSequence {
                    text: state.text.clone(),
                    total: state.score,
                    average,
                });
            }

            // Do not repeatedly sort the whole accumulated result set when the
            // user asks for effectively all generated candidates. If `limit` is
            // small, keep only the current best window to cap memory and CPU.
            if results.len() > options.limit {
                retain_best_generated(&mut results, options.limit);
            }
        }
    }

    results.sort_by(compare_generated);
    if results.len() > options.limit {
        results.truncate(options.limit);
    }
    Ok(results)
}

/// Streaming generator optimized for pipes (`ergokeygen gen --stream | hashcat`).
///
/// It emits each completed depth immediately instead of waiting for all lengths
/// and then globally sorting them. This trades exact cross-length ordering for
/// much lower first-line latency. The scoring model and beam contents are the
/// same as in `generate`; only final global ordering is relaxed.
pub fn generate_stream<F>(
    layout: &Layout,
    settings: &Settings,
    options: &GenerateOptions,
    mut emit: F,
) -> Result<(), String>
where
    F: FnMut(GeneratedSequence) -> Result<(), String>,
{
    if options.min_len == 0 || options.max_len < options.min_len {
        return Err("invalid length range".to_string());
    }
    if options.limit == 0 || options.beam == 0 {
        return Ok(());
    }

    let keys = sanitize_charset_as_keys(layout, &options.charset);
    if keys.is_empty() {
        return Err("charset has no supported chars".to_string());
    }

    if options.engine == GenerationEngine::FastV1 {
        return generate_stream_fast(layout, settings, options, &keys, emit);
    }

    let mut emitted = 0usize;
    let mut states = vec![TypingState::new(layout)];
    let mut dedupe = DedupeFilter::new(options.dedupe);

    for depth in 1..=options.max_len {
        let mut next_states = expand_next_states(&states, &keys, layout, settings, options);

        retain_best_states(&mut next_states, options.beam);
        states = next_states;

        if depth >= options.min_len {
            for state in &states {
                if emitted >= options.limit {
                    return Ok(());
                }
                let average = state.average();
                if options.max_avg_cost.is_some_and(|max| average > max) {
                    continue;
                }
                if options.max_total_cost.is_some_and(|max| state.score > max) {
                    continue;
                }
                if !dedupe.accept(&state.text) {
                    continue;
                }
                emit(GeneratedSequence {
                    text: state.text.clone(),
                    total: state.score,
                    average,
                })?;
                emitted += 1;
            }
        }
    }

    Ok(())
}
