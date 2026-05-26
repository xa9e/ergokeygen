use std::cmp::Ordering;

use rayon::prelude::*;

use super::scoring::build_transition_base_table;
use super::scoring::{build_compact_key_table, precompute_keys, PrecomputedKey};
use super::state::{
    build_expanded_soa, compute_build_timing, reconstruct_text_soa, score_expansion_soa,
    BeamArrays, KeyLookupTable, ParentLink,
};
use crate::generate::{compare_generated, DedupeFilter, DedupeMode};
use crate::model::compare_f64;
use crate::*;

/// Pending result before text reconstruction.
struct PendingResult {
    depth: usize,
    key_byte: u8,
    parent_idx: u32,
    text_len: u8,
    total: f64,
    average: f64,
}

fn compare_pending(a: &PendingResult, b: &PendingResult) -> Ordering {
    compare_f64(a.average, b.average)
        .then_with(|| compare_f64(a.total, b.total))
        .then_with(|| a.text_len.cmp(&b.text_len))
}

// ── FastV1 generation dispatch (SoA) ─────────────────────────────────

pub(crate) fn generate_fast(
    layout: &Layout,
    settings: &Settings,
    options: &GenerateOptions,
    keys: &[Key],
) -> Result<Vec<GeneratedSequence>, String> {
    let len_count = options.max_len.saturating_sub(options.min_len) + 1;
    let expected = options.beam.saturating_mul(len_count);
    let mut arrays = BeamArrays::with_capacity(options.beam);
    arrays.push_initial(layout);
    let mut pending: Vec<PendingResult> = Vec::with_capacity(options.limit.min(expected));
    let mut dedupe = DedupeFilter::new(options.dedupe);
    let pckeys = precompute_keys(keys, layout, &settings.weights, &settings.prefer_hand);
    let key_lookup = KeyLookupTable::build(&pckeys);
    let ck_table = build_compact_key_table(keys);
    let trans_table = build_transition_base_table(keys, &pckeys, &settings.weights);
    let mut parent_links: Vec<Vec<ParentLink>> = Vec::new();
    let needs_dedupe_text = options.dedupe != DedupeMode::Off;

    for depth in 1..=options.max_len {
        let mut next_arrays = expand_next_states_soa(
            &arrays,
            keys,
            &pckeys,
            settings,
            options,
            &ck_table,
            &trans_table,
            &key_lookup,
        );
        retain_best_soa(&mut next_arrays, options.beam);
        let links: Vec<ParentLink> = (0..arrays.len())
            .map(|i| arrays.to_parent_link(i))
            .collect();
        parent_links.push(links);
        arrays.clear();
        std::mem::swap(&mut arrays, &mut next_arrays);
        drop(next_arrays);

        if depth >= options.min_len {
            for i in 0..arrays.len() {
                let average = arrays.average(i);
                if options.max_avg_cost.is_some_and(|max| average > max) {
                    continue;
                }
                let sd = &arrays.scoring[i];
                if options.max_total_cost.is_some_and(|max| sd.score > max) {
                    continue;
                }
                if needs_dedupe_text {
                    let text = reconstruct_text_soa(
                        arrays.key_byte[i],
                        arrays.parent_idx[i],
                        sd.text_len,
                        &parent_links,
                        depth,
                    );
                    if !dedupe.accept(&text) {
                        continue;
                    }
                }
                pending.push(PendingResult {
                    depth,
                    key_byte: arrays.key_byte[i],
                    parent_idx: arrays.parent_idx[i],
                    text_len: sd.text_len,
                    total: sd.score,
                    average,
                });
                if pending.len() > options.limit.saturating_mul(2) {
                    retain_best_pending(&mut pending, options.limit);
                }
            }
        }
    }

    retain_best_pending(&mut pending, options.limit);
    let mut results: Vec<GeneratedSequence> = Vec::with_capacity(pending.len());
    for p in &pending {
        let text =
            reconstruct_text_soa(p.key_byte, p.parent_idx, p.text_len, &parent_links, p.depth);
        results.push(GeneratedSequence {
            text,
            total: p.total,
            average: p.average,
        });
    }
    results.sort_by(compare_generated);
    if results.len() > options.limit {
        results.truncate(options.limit);
    }
    Ok(results)
}

pub(crate) fn generate_stream_fast<F>(
    layout: &Layout,
    settings: &Settings,
    options: &GenerateOptions,
    keys: &[Key],
    mut emit: F,
) -> Result<(), String>
where
    F: FnMut(GeneratedSequence) -> Result<(), String>,
{
    let mut emitted = 0usize;
    let mut arrays = BeamArrays::with_capacity(options.beam);
    arrays.push_initial(layout);
    let mut dedupe = DedupeFilter::new(options.dedupe);
    let pckeys = precompute_keys(keys, layout, &settings.weights, &settings.prefer_hand);
    let key_lookup = KeyLookupTable::build(&pckeys);
    let ck_table = build_compact_key_table(keys);
    let trans_table = build_transition_base_table(keys, &pckeys, &settings.weights);
    let mut parent_links: Vec<Vec<ParentLink>> = Vec::new();

    for depth in 1..=options.max_len {
        let mut next_arrays = expand_next_states_soa(
            &arrays,
            keys,
            &pckeys,
            settings,
            options,
            &ck_table,
            &trans_table,
            &key_lookup,
        );
        retain_best_soa(&mut next_arrays, options.beam);
        let links: Vec<ParentLink> = (0..arrays.len())
            .map(|i| arrays.to_parent_link(i))
            .collect();
        parent_links.push(links);
        arrays = next_arrays;

        if depth >= options.min_len {
            let mut indices: Vec<usize> = (0..arrays.len()).collect();
            indices.sort_by(|&a, &b| compare_states_soa(&arrays, a, b));
            for i in indices {
                if emitted >= options.limit {
                    return Ok(());
                }
                let average = arrays.average(i);
                if options.max_avg_cost.is_some_and(|max| average > max) {
                    continue;
                }
                let sd = &arrays.scoring[i];
                if options.max_total_cost.is_some_and(|max| sd.score > max) {
                    continue;
                }
                let text = reconstruct_text_soa(
                    arrays.key_byte[i],
                    arrays.parent_idx[i],
                    sd.text_len,
                    &parent_links,
                    depth,
                );
                if !dedupe.accept(&text) {
                    continue;
                }
                emit(GeneratedSequence {
                    text,
                    total: sd.score,
                    average,
                })?;
                emitted += 1;
            }
        }
    }
    Ok(())
}

// ── Histogram-based beam selection ───────────────────────────────────

fn histogram_select_indices(scores: &[f64], beam: usize, n_keys: usize) -> Vec<usize> {
    if scores.len() > 100_000 {
        return parallel_histogram_select(scores, beam, n_keys);
    }
    sequential_histogram_select(scores, beam, n_keys)
}

fn sequential_histogram_select(scores: &[f64], beam: usize, n_keys: usize) -> Vec<usize> {
    let n = scores.len();
    if n <= beam {
        return (0..n).collect();
    }

    let mut min_score = f64::INFINITY;
    let mut max_score = f64::NEG_INFINITY;
    for &s in scores {
        if s < min_score {
            min_score = s;
        }
        if s > max_score {
            max_score = s;
        }
    }

    if min_score == max_score {
        return (0..beam).collect();
    }

    const NUM_BINS: usize = 2048;
    let mut hist = [0usize; NUM_BINS];
    let scale = NUM_BINS as f64 / (max_score - min_score);
    for &s in scores {
        let bin = ((s - min_score) * scale) as usize;
        let bin = bin.min(NUM_BINS - 1);
        hist[bin] += 1;
    }

    let mut cumulative = 0usize;
    let mut boundary_bin = NUM_BINS - 1;
    for (i, &count) in hist.iter().enumerate() {
        cumulative += count;
        if cumulative >= beam {
            boundary_bin = i;
            break;
        }
    }

    let below_boundary: usize = hist[..boundary_bin].iter().sum();
    let needed_from_boundary = beam - below_boundary;

    let mut below_indices = Vec::with_capacity(below_boundary);
    let mut boundary_indices = Vec::new();

    for (i, &s) in scores.iter().enumerate() {
        let bin = ((s - min_score) * scale) as usize;
        let bin = bin.min(NUM_BINS - 1);
        if bin < boundary_bin {
            below_indices.push(i);
        } else if bin == boundary_bin {
            boundary_indices.push(i);
        }
    }

    if boundary_indices.len() > needed_from_boundary {
        boundary_indices.select_nth_unstable_by(needed_from_boundary, |&a: &usize, &b: &usize| {
            compare_f64(scores[a], scores[b])
                .then_with(|| (a / n_keys).cmp(&(b / n_keys)))
                .then_with(|| (a % n_keys).cmp(&(b % n_keys)))
        });
        boundary_indices.truncate(needed_from_boundary);
    }

    below_indices.extend(boundary_indices);
    below_indices
}

fn parallel_histogram_select(scores: &[f64], beam: usize, n_keys: usize) -> Vec<usize> {
    use rayon::prelude::*;
    const NUM_BINS: usize = 2048;
    let n = scores.len();
    if n <= beam {
        return (0..n).collect();
    }

    let (min_score, max_score) = scores
        .par_iter()
        .fold(
            || (f64::INFINITY, f64::NEG_INFINITY),
            |(min, max), &s| (min.min(s), max.max(s)),
        )
        .reduce(
            || (f64::INFINITY, f64::NEG_INFINITY),
            |a, b| (a.0.min(b.0), a.1.max(b.1)),
        );

    if min_score == max_score {
        return (0..beam).collect();
    }

    let scale = NUM_BINS as f64 / (max_score - min_score);

    let local_hists: Vec<[usize; NUM_BINS]> = scores
        .par_chunks(16384)
        .map(|chunk| {
            let mut hist = [0usize; NUM_BINS];
            for &s in chunk {
                let bin = ((s - min_score) * scale) as usize;
                let bin = bin.min(NUM_BINS - 1);
                hist[bin] += 1;
            }
            hist
        })
        .collect();

    let mut hist = [0usize; NUM_BINS];
    for local in &local_hists {
        for (i, &count) in local.iter().enumerate() {
            hist[i] += count;
        }
    }

    let mut cumulative = 0usize;
    let mut boundary_bin = NUM_BINS - 1;
    for (i, &count) in hist.iter().enumerate() {
        cumulative += count;
        if cumulative >= beam {
            boundary_bin = i;
            break;
        }
    }

    let below_boundary: usize = hist[..boundary_bin].iter().sum();
    let needed_from_boundary = beam - below_boundary;

    let chunk_size = 16384;
    let local_results: Vec<(Vec<usize>, Vec<usize>)> = scores
        .par_chunks(chunk_size)
        .enumerate()
        .map(|(chunk_idx, chunk)| {
            let offset = chunk_idx * chunk_size;
            let mut below = Vec::new();
            let mut boundary = Vec::new();
            for (j, &s) in chunk.iter().enumerate() {
                let i = offset + j;
                let bin = ((s - min_score) * scale) as usize;
                let bin = bin.min(NUM_BINS - 1);
                if bin < boundary_bin {
                    below.push(i);
                } else if bin == boundary_bin {
                    boundary.push(i);
                }
            }
            (below, boundary)
        })
        .collect();

    let mut below_indices = Vec::with_capacity(below_boundary);
    let mut boundary_indices = Vec::with_capacity(hist[boundary_bin]);
    for (below, boundary) in &local_results {
        below_indices.extend_from_slice(below);
        boundary_indices.extend_from_slice(boundary);
    }

    if boundary_indices.len() > needed_from_boundary {
        boundary_indices.select_nth_unstable_by(needed_from_boundary, |&a: &usize, &b: &usize| {
            compare_f64(scores[a], scores[b])
                .then_with(|| (a / n_keys).cmp(&(b / n_keys)))
                .then_with(|| (a % n_keys).cmp(&(b % n_keys)))
        });
        boundary_indices.truncate(needed_from_boundary);
    }

    below_indices.extend(boundary_indices);
    below_indices
}

// ── SoA expansion ────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn expand_next_states_soa(
    arrays: &BeamArrays,
    keys: &[Key],
    pckeys: &[PrecomputedKey],
    settings: &Settings,
    options: &GenerateOptions,
    ck_table: &[Option<super::key::CompactKey>; 256],
    trans_table: &[[f64; 256]; 256],
    key_lookup: &KeyLookupTable,
) -> BeamArrays {
    let n_keys = keys.len();
    let work = arrays.len().saturating_mul(n_keys);
    let beam = options.beam;

    let scores = score_all_expansions_soa(
        arrays,
        pckeys,
        settings,
        options,
        ck_table,
        trans_table,
        work,
        key_lookup,
    );
    let survivor_indices = if scores.len() > beam {
        histogram_select_indices(&scores, beam, n_keys)
    } else {
        (0..scores.len()).collect()
    };
    build_survivors_soa(
        &survivor_indices,
        arrays,
        pckeys,
        settings,
        &scores,
        n_keys,
        key_lookup,
    )
}

#[allow(clippy::too_many_arguments)]
fn score_all_expansions_soa(
    arrays: &BeamArrays,
    pckeys: &[PrecomputedKey],
    settings: &Settings,
    options: &GenerateOptions,
    ck_table: &[Option<super::key::CompactKey>; 256],
    trans_table: &[[f64; 256]; 256],
    work: usize,
    key_lookup: &KeyLookupTable,
) -> Vec<f64> {
    let n_keys = pckeys.len();
    if options.parallel && work >= options.parallel_threshold {
        (0..work)
            .into_par_iter()
            .map(|i| {
                let si = i / n_keys;
                let ki = i % n_keys;
                let (score, _, _, _) = score_expansion_soa(
                    &arrays.scoring[si],
                    &arrays.positions[si],
                    &arrays.postures[si],
                    &arrays.finger_ready[si],
                    pckeys[ki].ckey.typed,
                    settings,
                    &pckeys[ki],
                    ck_table,
                    trans_table,
                    key_lookup,
                );
                score
            })
            .collect()
    } else {
        let mut scores = Vec::with_capacity(work);
        for si in 0..arrays.len() {
            let sd = &arrays.scoring[si];
            let pos = &arrays.positions[si];
            let post = &arrays.postures[si];
            let fr = &arrays.finger_ready[si];
            for pckey in pckeys.iter().take(n_keys) {
                let (score, _, _, _) = score_expansion_soa(
                    sd,
                    pos,
                    post,
                    fr,
                    pckey.ckey.typed,
                    settings,
                    pckey,
                    ck_table,
                    trans_table,
                    key_lookup,
                );
                scores.push(score);
            }
        }
        scores
    }
}

fn build_survivors_soa(
    indices: &[usize],
    arrays: &BeamArrays,
    pckeys: &[PrecomputedKey],
    settings: &Settings,
    scores: &[f64],
    n_keys: usize,
    key_lookup: &KeyLookupTable,
) -> BeamArrays {
    // Chunk-based parallel build: each chunk processes a batch of indices
    // sequentially within one rayon task, producing a small BeamArrays.
    // Then merge all chunk results. Reduces rayon scheduling overhead
    // from N individual tasks to N/CHUNK and improves cache locality.
    const BUILD_CHUNK: usize = 512;
    let chunks: Vec<BeamArrays> = indices
        .par_chunks(BUILD_CHUNK)
        .map(|chunk| {
            let mut ba = BeamArrays::with_capacity(chunk.len());
            for &i in chunk {
                let si = i / n_keys;
                let ki = i % n_keys;
                let sd = &arrays.scoring[si];
                let pos = &arrays.positions[si];
                let post = &arrays.postures[si];
                let fr = &arrays.finger_ready[si];
                let pckey = &pckeys[ki];
                let score = scores[i];
                let (press_time, finger_ready_value) =
                    compute_build_timing(sd, pos, post, fr, pckey, settings, key_lookup);
                let (sd, pos, post, fr, pidx, kb) = build_expanded_soa(
                    sd,
                    pos,
                    post,
                    fr,
                    si as u32,
                    ki,
                    score,
                    press_time,
                    finger_ready_value,
                    pckeys,
                );
                ba.scoring.push(sd);
                ba.positions.push(pos);
                ba.postures.push(post);
                ba.finger_ready.push(fr);
                ba.parent_idx.push(pidx);
                ba.key_byte.push(kb);
            }
            ba
        })
        .collect();

    let total: usize = chunks.iter().map(|c| c.len()).sum();
    let mut next = BeamArrays::with_capacity(total);
    for chunk in &chunks {
        next.scoring.extend_from_slice(&chunk.scoring);
        next.positions.extend_from_slice(&chunk.positions);
        next.postures.extend_from_slice(&chunk.postures);
        next.finger_ready.extend_from_slice(&chunk.finger_ready);
        next.parent_idx.extend_from_slice(&chunk.parent_idx);
        next.key_byte.extend_from_slice(&chunk.key_byte);
    }
    next
}

fn retain_best_soa(arrays: &mut BeamArrays, limit: usize) {
    if arrays.len() > limit {
        // Find the top `limit` indices via partial sort, then copy survivors
        // into a new set of arrays (sequential copy is more cache-friendly
        // than random in-place swaps across 6 parallel arrays).
        let mut indices: Vec<usize> = (0..arrays.len()).collect();
        indices.select_nth_unstable_by(limit, |&a: &usize, &b: &usize| {
            compare_states_soa(arrays, a, b)
        });
        // Sequential copy of survivors into front positions
        for (target, &idx) in indices.iter().enumerate().take(limit) {
            if target != idx {
                arrays.scoring[target] = arrays.scoring[idx];
                arrays.positions[target] = arrays.positions[idx];
                arrays.postures[target] = arrays.postures[idx];
                arrays.finger_ready[target] = arrays.finger_ready[idx];
                arrays.parent_idx[target] = arrays.parent_idx[idx];
                arrays.key_byte[target] = arrays.key_byte[idx];
            }
        }
        arrays.scoring.truncate(limit);
        arrays.positions.truncate(limit);
        arrays.postures.truncate(limit);
        arrays.finger_ready.truncate(limit);
        arrays.parent_idx.truncate(limit);
        arrays.key_byte.truncate(limit);
    }
}

fn retain_best_pending(pending: &mut Vec<PendingResult>, limit: usize) {
    if pending.len() > limit {
        pending.select_nth_unstable_by(limit, compare_pending);
        pending.truncate(limit);
    }
    pending.sort_by(compare_pending);
}

fn compare_states_soa(arrays: &BeamArrays, a: usize, b: usize) -> Ordering {
    compare_f64(arrays.average(a), arrays.average(b))
        .then_with(|| compare_f64(arrays.scoring[a].score, arrays.scoring[b].score))
        .then_with(|| arrays.scoring[a].tail.cmp(&arrays.scoring[b].tail))
        .then_with(|| arrays.scoring[a].text_len.cmp(&arrays.scoring[b].text_len))
}
