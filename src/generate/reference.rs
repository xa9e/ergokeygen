use std::cmp::Ordering;

use rayon::prelude::*;

use crate::model::compare_f64;
use crate::*;

use super::{GenerateOptions, GeneratedSequence, GenerationEngine};

pub(super) fn expand_next_states(
    states: &[TypingState],
    keys: &[Key],
    layout: &Layout,
    settings: &Settings,
    options: &GenerateOptions,
) -> Vec<TypingState> {
    let work = states.len().saturating_mul(keys.len());

    if options.engine == GenerationEngine::Reference {
        return expand_next_states_sequential(states, keys, layout, settings, work);
    }

    // This is the main CPU hot path. Every prefix can be expanded independently,
    // so Rayon gives near-embarrassingly-parallel work without changing the
    // model. We still do one shared retain/sort step afterwards, so quality and
    // ordering remain equivalent to the reference single-threaded mode.
    if options.parallel && work >= options.parallel_threshold {
        states
            .par_iter()
            .flat_map_iter(|state| {
                keys.iter()
                    .copied()
                    .map(move |key| state.push_compact(layout, key, settings))
            })
            .collect()
    } else {
        expand_next_states_sequential(states, keys, layout, settings, work)
    }
}

fn expand_next_states_sequential(
    states: &[TypingState],
    keys: &[Key],
    layout: &Layout,
    settings: &Settings,
    work: usize,
) -> Vec<TypingState> {
    let mut next_states = Vec::with_capacity(work);
    for state in states {
        for &key in keys {
            next_states.push(state.push_compact(layout, key, settings));
        }
    }
    next_states
}

pub(super) fn retain_best_states(states: &mut Vec<TypingState>, limit: usize) {
    if states.len() > limit {
        states.select_nth_unstable_by(limit, compare_states);
        states.truncate(limit);
    }
    states.sort_by(compare_states);
}

pub(crate) fn retain_best_generated(items: &mut Vec<GeneratedSequence>, limit: usize) {
    if items.len() > limit {
        items.select_nth_unstable_by(limit, compare_generated);
        items.truncate(limit);
    }
    items.sort_by(compare_generated);
}

fn compare_states(a: &TypingState, b: &TypingState) -> Ordering {
    compare_f64(a.average(), b.average())
        .then_with(|| compare_f64(a.score, b.score))
        .then_with(|| a.text.cmp(&b.text))
}

pub(crate) fn compare_generated(a: &GeneratedSequence, b: &GeneratedSequence) -> Ordering {
    compare_f64(a.average, b.average)
        .then_with(|| compare_f64(a.total, b.total))
        .then_with(|| a.text.cmp(&b.text))
}
