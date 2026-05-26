use super::key::*;
use super::scoring::{ck_cognitive_pattern_val, FastStep, PrecomputedKey};
use crate::layout::{f32_adjusted_rest_f32, f64_to_f32, F32PalmPosture, F32Pos};
use crate::*;

/// Tail buffer size for scoring.
const TAIL_SIZE: usize = 8;

/// Compact parent link for text reconstruction.
#[derive(Clone, Copy)]
pub(super) struct ParentLink {
    pub(super) parent_idx: u32,
    pub(super) key_byte: u8,
}

pub(super) const NO_KEY_FINGER: u8 = 0xFF;

/// Sentinel index meaning "no key present" in ScoringData key index fields.
pub(super) const NO_KEY_IDX: u16 = u16::MAX;

#[inline]
pub(super) fn no_key() -> CompactKey {
    CompactKey {
        typed: 0,
        physical: 0,
        x: 0.0,
        y: 0.0,
        finger: NO_KEY_FINGER,
        flags: 0,
    }
}

#[inline]
pub(super) fn has_key(ck: CompactKey) -> bool {
    ck.finger != NO_KEY_FINGER
}

/// Lookup table from key index (0..n_keys-1) to CompactKey.
/// Index NO_KEY_IDX maps to no_key(). Used by scoring functions
/// to resolve key indices in ScoringData back to CompactKey values.
pub(super) struct KeyLookupTable {
    keys: Vec<CompactKey>,
}

impl KeyLookupTable {
    pub(super) fn build(pckeys: &[PrecomputedKey]) -> Self {
        debug_assert!(pckeys.len() < NO_KEY_IDX as usize);
        let keys: Vec<CompactKey> = pckeys.iter().map(|pk| pk.ckey).collect();
        Self { keys }
    }

    #[inline]
    pub(super) fn get(&self, idx: u16) -> CompactKey {
        if idx == NO_KEY_IDX {
            no_key()
        } else {
            self.keys[idx as usize]
        }
    }

    #[inline]
    pub(super) fn has(&self, idx: u16) -> bool {
        idx != NO_KEY_IDX
    }
}

/// Compute the tail bytes for the new state after pushing `byte`.
#[inline]
pub(super) fn update_tail(tail: &[u8; TAIL_SIZE], text_len: u8, byte: u8) -> [u8; TAIL_SIZE] {
    let old_len = text_len as usize;
    let mut new_tail = [0u8; TAIL_SIZE];
    if old_len < TAIL_SIZE {
        new_tail[..old_len].copy_from_slice(&tail[..old_len]);
        new_tail[old_len] = byte;
    } else {
        new_tail[..TAIL_SIZE - 1].copy_from_slice(&tail[1..TAIL_SIZE]);
        new_tail[TAIL_SIZE - 1] = byte;
    }
    new_tail
}

#[cfg(test)]
mod tests {
    use super::{update_tail, TAIL_SIZE};

    #[test]
    fn update_tail_shifts_after_capacity() {
        let mut tail = [0u8; TAIL_SIZE];
        for (idx, slot) in tail.iter_mut().enumerate() {
            *slot = b'a' + idx as u8;
        }

        let shifted = update_tail(&tail, TAIL_SIZE as u8, b'i');

        assert_eq!(&shifted, b"bcdefghi");
    }
}

// ── SoA (Structure of Arrays) beam state ─────────────────────────────
// Split FastState into scoring-hot data (ScoringData ~40B) and cold data
// (positions, postures, finger_ready) stored in parallel arrays.
// During scoring, iterating ScoringData reads ~40B per state vs 200B,
// reducing memory bandwidth by ~80%.

/// Hot scoring data: fields accessed during every score_expansion call.
/// Field order optimized for packing: f64(8)+f64(8)+f32(4)+u16*4+u8*9 = 40 bytes.
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct ScoringData {
    pub(super) score: f64,
    pub(super) time: f64,
    pub(super) row_run_y: f32,
    pub(super) row_run_len: u16,
    pub(super) last_key: u16,
    pub(super) prev_key: u16,
    pub(super) prev2_key: u16,
    pub(super) text_len: u8,
    pub(super) tail: [u8; TAIL_SIZE],
}

/// Full beam state stored as parallel arrays (SoA layout).
pub(super) struct BeamArrays {
    pub(super) scoring: Vec<ScoringData>,
    pub(super) positions: Vec<[F32Pos; 8]>,
    pub(super) postures: Vec<[F32PalmPosture; 2]>,
    pub(super) finger_ready: Vec<[f32; 8]>,
    pub(super) parent_idx: Vec<u32>,
    pub(super) key_byte: Vec<u8>,
}

impl BeamArrays {
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            scoring: Vec::with_capacity(capacity),
            positions: Vec::with_capacity(capacity),
            postures: Vec::with_capacity(capacity),
            finger_ready: Vec::with_capacity(capacity),
            parent_idx: Vec::with_capacity(capacity),
            key_byte: Vec::with_capacity(capacity),
        }
    }

    pub(super) fn len(&self) -> usize {
        self.scoring.len()
    }

    pub(super) fn clear(&mut self) {
        self.scoring.clear();
        self.positions.clear();
        self.postures.clear();
        self.finger_ready.clear();
        self.parent_idx.clear();
        self.key_byte.clear();
    }

    pub(super) fn push_initial(&mut self, layout: &Layout) {
        let mut positions = [F32Pos::default(); 8];
        for finger in Finger::ALL {
            let p = layout.home_pos(finger);
            positions[finger.idx()] = F32Pos {
                x: p.x as f32,
                y: p.y as f32,
            };
        }
        self.scoring.push(ScoringData {
            score: 0.0,
            time: 0.0,
            row_run_y: 0.0,
            row_run_len: 0,
            text_len: 0,
            last_key: NO_KEY_IDX,
            prev_key: NO_KEY_IDX,
            prev2_key: NO_KEY_IDX,
            tail: [0u8; TAIL_SIZE],
        });
        self.positions.push(positions);
        self.postures.push([F32PalmPosture::default(); 2]);
        self.finger_ready.push([0.0f32; 8]);
        self.parent_idx.push(u32::MAX);
        self.key_byte.push(0);
    }

    #[inline]
    pub(super) fn to_parent_link(&self, idx: usize) -> ParentLink {
        ParentLink {
            parent_idx: self.parent_idx[idx],
            key_byte: self.key_byte[idx],
        }
    }

    #[inline]
    pub(super) fn average(&self, idx: usize) -> f64 {
        let sd = &self.scoring[idx];
        if sd.text_len == 0 {
            0.0
        } else {
            sd.score / sd.text_len as f64
        }
    }
}

/// Reconstruct text from parent links.
pub(super) fn reconstruct_text_soa(
    key_byte: u8,
    parent_idx: u32,
    text_len: u8,
    parent_links: &[Vec<ParentLink>],
    depth: usize,
) -> String {
    if text_len == 0 {
        return String::new();
    }
    let mut buf = vec![0u8; text_len as usize];
    let mut d = depth;
    let mut idx = parent_idx;
    let mut pos = text_len as usize;
    pos -= 1;
    buf[pos] = key_byte;
    while pos > 0 && d > 0 {
        pos -= 1;
        let links = &parent_links[d - 1];
        let link = &links[idx as usize];
        buf[pos] = link.key_byte;
        idx = link.parent_idx;
        d -= 1;
    }
    unsafe { String::from_utf8_unchecked(buf) }
}

// ── SoA scoring functions ─────────────────────────────────────────────

/// SoA scoring: compute score for expanding state with a key.
#[inline]
#[allow(clippy::too_many_arguments)]
pub(super) fn score_expansion_soa(
    sd: &ScoringData,
    positions: &[F32Pos; 8],
    postures: &[F32PalmPosture; 2],
    finger_ready: &[f32; 8],
    key_byte: u8,
    settings: &Settings,
    pckey: &PrecomputedKey,
    ck_table: &[Option<CompactKey>; 256],
    trans_table: &[[f64; 256]; 256],
    key_lookup: &KeyLookupTable,
) -> (f64, [u8; TAIL_SIZE], FastStep, f32) {
    let new_text_len = sd.text_len + 1;
    let new_tail = update_tail(&sd.tail, sd.text_len, key_byte);
    let ckey = pckey.ckey;
    let step = step_cost_soa(
        sd,
        positions,
        postures,
        finger_ready,
        &new_tail,
        new_text_len as usize,
        ckey,
        pckey,
        ck_table,
        trans_table,
        settings,
        key_lookup,
    );
    let score = sd.score + step.total;
    let finger_ready_value = f64_to_f32(
        step.press_time
            + pckey.finger_base_recovery
            + (step.dynamic_cost - 0.80).max(0.0) * settings.weights.movement_recovery
            + (step.movement_time - settings.weights.beat_interval).max(0.0) * 0.35,
    );
    (score, new_tail, step, finger_ready_value)
}

/// Lightweight timing-only computation for build phase.
/// Avoids full step_cost; only computes press_time and finger_ready_value
/// needed by build_expanded_soa. ~4-5x cheaper than score_expansion_soa.
#[inline]
pub(super) fn compute_build_timing(
    sd: &ScoringData,
    positions: &[F32Pos; 8],
    postures: &[F32PalmPosture; 2],
    finger_ready: &[f32; 8],
    pckey: &PrecomputedKey,
    settings: &Settings,
    key_lookup: &KeyLookupTable,
) -> (f64, f32) {
    let w = &settings.weights;
    let ckey = pckey.ckey;
    let posture = postures[pckey.hand_idx];
    let adjusted_rest = f32_adjusted_rest_f32(posture, pckey.rest, pckey.hand_idx);
    let dynamic_cost = w.dynamic_distance
        * positions[pckey.finger_idx].dist_to_xy_f32(ckey.x, ckey.y)
        + w.palm_distance * adjusted_rest.dist_to_xy_f32(ckey.x, ckey.y)
        + w.palm_tension * posture.tension as f64;
    let move_time = movement_duration_soa(sd, ckey, pckey, dynamic_cost, settings, key_lookup);
    let planned = sd.time + move_time;
    let mut wait = (finger_ready[pckey.finger_idx] as f64 - planned).max(0.0);
    if wait > 0.0 && key_lookup.has(sd.prev_key) {
        let prev = key_lookup.get(sd.prev_key);
        if prev.finger == ckey.finger
            && ckey.hand() == prev.hand()
            && (0.55f32..=1.45f32).contains(&ckey.y)
            && (prev.y - ckey.y).abs() as f64 >= 0.9
        {
            wait = (wait - w.home_return_wait_relief).max(0.0);
        }
    }
    let press_time = planned + wait;
    let finger_ready_value = f64_to_f32(
        press_time
            + pckey.finger_base_recovery
            + (dynamic_cost - 0.80).max(0.0) * w.movement_recovery
            + (move_time - w.beat_interval).max(0.0) * 0.35,
    );
    (press_time, finger_ready_value)
}

/// SoA build: construct next state data from a surviving expansion.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_expanded_soa(
    sd: &ScoringData,
    positions: &[F32Pos; 8],
    postures: &[F32PalmPosture; 2],
    finger_ready: &[f32; 8],
    parent_idx: u32,
    key_idx: usize,
    score: f64,
    press_time: f64,         // pre-computed by compute_build_timing
    finger_ready_value: f32, // pre-computed by compute_build_timing
    pckeys: &[PrecomputedKey],
) -> (
    ScoringData,
    [F32Pos; 8],
    [F32PalmPosture; 2],
    [f32; 8],
    u32,
    u8,
) {
    let pckey = &pckeys[key_idx];
    let key_byte = pckey.ckey.typed;
    let new_text_len = sd.text_len + 1;
    let new_tail = update_tail(&sd.tail, sd.text_len, key_byte);
    let ckey = pckey.ckey;
    debug_assert_eq!(key_idx as u16 as usize, key_idx);
    let key_idx_u16 = key_idx as u16;

    let new_sd = ScoringData {
        score,
        time: press_time,
        row_run_y: 0.0,
        row_run_len: 0,
        text_len: new_text_len,
        last_key: key_idx_u16,
        prev_key: sd.last_key,
        prev2_key: sd.prev_key,
        tail: new_tail,
    };

    // Position updates
    let mut new_positions = *positions;
    new_positions[pckey.finger_idx] = F32Pos {
        x: ckey.x,
        y: ckey.y,
    };
    let active_seq = finger_seq(ckey.finger);
    let hand_start = pckey.hand_idx * 4;
    for i in 0..4u8 {
        let idx = hand_start + i as usize;
        if idx == pckey.finger_idx {
            continue;
        }
        let old = new_positions[idx];
        let seq_i = finger_seq(idx as u8);
        let seq_gap = (seq_i - active_seq).abs() as f64;
        let alpha = (0.26 - 0.045 * seq_gap).max(0.08);
        let signed_gap = (seq_i - active_seq) as f32;
        let coupled_x = ckey.x + signed_gap * 0.95;
        let coupled_y = ckey.y + signed_gap.abs() * 0.15;
        new_positions[idx] = F32Pos {
            x: old.x + (alpha as f32 * (coupled_x - old.x)),
            y: old.y + (alpha as f32 * (coupled_y - old.y)),
        };
    }

    // Posture updates
    let mut new_postures = *postures;
    let old_posture = new_postures[pckey.hand_idx];
    let rest = pckey.rest;
    let target_offset_x = ((ckey.x - rest.x) * 0.55).clamp(-1.15, 1.15);
    let target_offset_y = ((ckey.y - rest.y) * 0.62).clamp(-1.05, 1.05);
    let target_rotation = (target_offset_x * 0.08 + target_offset_y * 0.04).clamp(-0.18, 0.18);
    let travel = positions[pckey.finger_idx].dist_to_xy_f32(ckey.x, ckey.y);
    let target_tension = ((travel as f32 - 0.65).max(0.0) * 0.35
        + (target_offset_x.abs() - 0.45).max(0.0) * 0.20)
        .clamp(0.0, 2.5);
    new_postures[pckey.hand_idx] = F32PalmPosture {
        offset_x: old_posture.offset_x * 0.72 + target_offset_x * 0.28,
        offset_y: old_posture.offset_y * 0.68 + target_offset_y * 0.32,
        rotation: old_posture.rotation * 0.78 + target_rotation * 0.22,
        tension: old_posture.tension * 0.72 + target_tension * 0.28,
    };
    let other_hand_idx = 1 - pckey.hand_idx;
    let other = new_postures[other_hand_idx];
    new_postures[other_hand_idx] = F32PalmPosture {
        offset_x: other.offset_x * 0.78,
        offset_y: other.offset_y * 0.78,
        rotation: other.rotation * 0.82,
        tension: other.tension * 0.70,
    };

    let mut new_finger_ready = *finger_ready;
    new_finger_ready[pckey.finger_idx] = finger_ready_value;

    let (new_row_run_y, new_row_run_len) = if sd.last_key != NO_KEY_IDX {
        let last = pckeys[sd.last_key as usize].ckey;
        if ck_same_row(last, ckey) {
            let len = if sd.row_run_len > 0 && (sd.row_run_y - ckey.y).abs() < 0.1 {
                sd.row_run_len + 1
            } else {
                2
            };
            (ckey.y, len)
        } else {
            (ckey.y, 1)
        }
    } else {
        (ckey.y, 1)
    };

    let final_sd = ScoringData {
        row_run_y: new_row_run_y,
        row_run_len: new_row_run_len,
        ..new_sd
    };
    (
        final_sd,
        new_positions,
        new_postures,
        new_finger_ready,
        parent_idx,
        key_byte,
    )
}

// ── SoA step_cost ─────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
#[inline]
pub(super) fn step_cost_soa(
    sd: &ScoringData,
    positions: &[F32Pos; 8],
    postures: &[F32PalmPosture; 2],
    finger_ready: &[f32; 8],
    tail: &[u8],
    candidate_len: usize,
    ckey: CompactKey,
    pckey: &PrecomputedKey,
    ck_table: &[Option<CompactKey>; 256],
    trans_table: &[[f64; 256]; 256],
    settings: &Settings,
    key_lookup: &KeyLookupTable,
) -> FastStep {
    let w = &settings.weights;
    let mut static_cost = pckey.base_static_cost;
    let axis_relief =
        fast_upper_reverse_axis_relief_soa(sd, ckey, pckey.axis_deviation, w, key_lookup);
    if axis_relief > 0.0 {
        static_cost -= axis_relief;
    }
    if sd.text_len == 0 && pckey.is_pinky_start {
        static_cost += w.pinky_start;
    }
    static_cost += pckey.shift_cost;
    let posture = postures[pckey.hand_idx];
    let adjusted_rest = f32_adjusted_rest_f32(posture, pckey.rest, pckey.hand_idx);
    let dynamic_cost = w.dynamic_distance
        * positions[pckey.finger_idx].dist_to_xy_f32(ckey.x, ckey.y)
        + w.palm_distance * adjusted_rest.dist_to_xy_f32(ckey.x, ckey.y)
        + w.palm_tension * posture.tension as f64;
    let mut rhythm_cost = pckey.rhythm_cost;
    let mut transition_cost = 0.0;
    if key_lookup.has(sd.last_key) {
        let last = key_lookup.get(sd.last_key);
        transition_cost += trans_table[last.physical as usize][ckey.physical as usize];
        if last.hand() != ckey.hand() {
            match settings.mode {
                RhythmMode::OneHand => rhythm_cost += w.hand_switch_onehand,
                RhythmMode::Neutral => rhythm_cost += w.hand_switch_neutral,
                RhythmMode::Alternation => rhythm_cost += w.hand_switch_alternation,
            }
        }
        if sd.row_run_len >= 2 && ck_same_row(last, ckey) {
            transition_cost += w.row_run_reward;
            if ckey.y > 1.5f32 {
                transition_cost += w.bottom_lock_reward;
            }
        } else if sd.row_run_len >= 2 && (ckey.y - last.y).abs() as f64 >= 2.0 {
            transition_cost += w.row_jump_penalty * sd.row_run_len as f64;
        }
    }
    if key_lookup.has(sd.prev_key) && key_lookup.has(sd.last_key) {
        let a = key_lookup.get(sd.prev_key);
        let b = key_lookup.get(sd.last_key);
        if ck_is_smooth_trigram(a, b, ckey) {
            let d1 = ck_roll_delta(a, b);
            let d2 = ck_roll_delta(b, ckey);
            if d1 > 0 && d2 > 0 && ckey.x > b.x && b.x > a.x {
                transition_cost += w.smooth_trigram_reward;
                if ck_same_row(a, b) && ck_same_row(b, ckey) && ckey.hand() == 0 {
                    if (ckey.y - 1.0f32).abs() < 0.1 {
                        transition_cost += w.home_sweep_reward;
                    } else {
                        transition_cost += w.home_sweep_reward * w.non_home_sweep_reward_factor;
                    }
                }
            } else {
                transition_cost += w.reverse_trigram_penalty;
            }
        }
        if ck_upper_reverse_coupled_trigram(a, b, ckey) {
            transition_cost += w.upper_reverse_coupled_roll_reward;
        }
        if ck_is_compact_cluster(a, b, ckey) {
            transition_cost += w.compact_cluster_reward;
        }
        let d1 = ck_roll_delta(a, b);
        let d2 = ck_roll_delta(b, ckey);
        if a.hand() == b.hand() && b.hand() == ckey.hand() && d1 * d2 < 0 {
            transition_cost += w.redirect;
        }
        if a.finger == ckey.finger && b.finger != a.finger {
            transition_cost += w.recent_same_finger * pckey.finger_recovery_factor;
        }
    }
    if key_lookup.has(sd.prev2_key) && key_lookup.has(sd.prev_key) && key_lookup.has(sd.last_key) {
        let a = key_lookup.get(sd.prev2_key);
        let b = key_lookup.get(sd.prev_key);
        let c = key_lookup.get(sd.last_key);
        if a.physical == ckey.physical && b.physical == c.physical {
            transition_cost += w.abba_bounce * pckey.finger_recovery_factor;
        }
        let cquad = [a, b, c, ckey];
        if ck_upper_reverse_split_sweep(&cquad) {
            transition_cost += w.upper_reverse_split_sweep_reward;
        } else {
            match ck_sweep_direction(&cquad) {
                1 => transition_cost += w.full_sweep_reward,
                -1 => transition_cost += w.reverse_full_sweep_penalty,
                _ if ck_four_finger_ordered_block(&cquad) => {
                    let mismatch = ck_mixed_motor_program_mismatch(&cquad);
                    if mismatch > 0.0 {
                        transition_cost +=
                            w.mixed_row_sweep_penalty * mismatch + w.asymmetric_row_actuation;
                    }
                }
                _ => {}
            }
        }
    }
    if candidate_len >= 8 && tail.len() >= 8 && tail[4..8] == tail[0..4] {
        let mut cblock: [CompactKey; 4] = [ckey; 4];
        let mut block_ok = true;
        for (i, &b) in tail[4..8].iter().enumerate() {
            if let Some(ck) = ck_table[b as usize] {
                cblock[i] = ck;
            } else {
                block_ok = false;
                break;
            }
        }
        if block_ok {
            if (ck_uniform_motor_program(&cblock) && ck_sweep_direction(&cblock) > 0)
                || ck_upper_reverse_split_sweep(&cblock)
            {
                transition_cost += w.motor_program_repeat_reward;
            } else if ck_four_finger_ordered_block(&cblock) {
                let mismatch = ck_mixed_motor_program_mismatch(&cblock);
                transition_cost +=
                    w.mixed_motor_program_penalty + mismatch * w.asymmetric_row_actuation;
            }
        }
    }
    if key_lookup.has(sd.prev2_key) && key_lookup.has(sd.prev_key) && key_lookup.has(sd.last_key) {
        transition_cost += ck_pair_direction_continuity(
            key_lookup.get(sd.prev2_key),
            key_lookup.get(sd.prev_key),
            key_lookup.get(sd.last_key),
            ckey,
            w,
        );
    }
    let direction_cost = ck_pre_sweep_direction_tail(tail, candidate_len, ck_table, w);
    transition_cost += direction_cost;
    let cognitive_cost = ck_cognitive_pattern_val(
        key_lookup.get(sd.prev2_key),
        key_lookup.get(sd.prev_key),
        key_lookup.get(sd.last_key),
        ckey,
        w,
    );
    let (timing_cost, _timing_wait, press_time, movement_time) = fast_timing_state_cost_soa(
        sd,
        ckey,
        pckey,
        dynamic_cost,
        finger_ready,
        settings,
        key_lookup,
    );
    let total =
        (static_cost + dynamic_cost + transition_cost + rhythm_cost + timing_cost + cognitive_cost)
            .max(0.05);
    FastStep {
        total,
        dynamic_cost,
        movement_time,
        press_time,
    }
}

#[inline]
fn fast_upper_reverse_axis_relief_soa(
    sd: &ScoringData,
    ckey: CompactKey,
    axis_cost: f64,
    w: &Weights,
    key_lookup: &KeyLookupTable,
) -> f64 {
    if axis_cost <= 0.0 {
        return 0.0;
    }
    if !key_lookup.has(sd.last_key) {
        return 0.0;
    }
    let last = key_lookup.get(sd.last_key);
    if ckey.hand() == 0
        && ckey.finger == Finger::LeftRing as u8
        && ckey.y < 0.5
        && last.finger == Finger::LeftMiddle as u8
        && last.y < 0.5
        && last.x > ckey.x
    {
        return axis_cost.min(w.upper_reverse_axis_relief);
    }
    0.0
}

fn fast_timing_state_cost_soa(
    sd: &ScoringData,
    ckey: CompactKey,
    pckey: &PrecomputedKey,
    dynamic_cost: f64,
    finger_ready: &[f32; 8],
    settings: &Settings,
    key_lookup: &KeyLookupTable,
) -> (f64, f64, f64, f64) {
    let weights = &settings.weights;
    let move_time = movement_duration_soa(sd, ckey, pckey, dynamic_cost, settings, key_lookup);
    let planned = sd.time + move_time;
    let mut wait = (finger_ready[pckey.finger_idx] as f64 - planned).max(0.0);
    if wait > 0.0 && key_lookup.has(sd.prev_key) {
        let prev = key_lookup.get(sd.prev_key);
        if prev.finger == ckey.finger
            && ckey.hand() == prev.hand()
            && (0.55f32..=1.45f32).contains(&ckey.y)
            && (prev.y - ckey.y).abs() as f64 >= 0.9
        {
            wait = (wait - weights.home_return_wait_relief).max(0.0);
        }
    }
    let press_time = planned + wait;
    (wait * weights.timing_wait, wait, press_time, move_time)
}

fn movement_duration_soa(
    sd: &ScoringData,
    ckey: CompactKey,
    pckey: &PrecomputedKey,
    dynamic_cost: f64,
    settings: &Settings,
    key_lookup: &KeyLookupTable,
) -> f64 {
    let weights = &settings.weights;
    let mut duration =
        weights.beat_interval + (dynamic_cost - 0.90).max(0.0) * weights.movement_time_factor;
    duration += pckey.axis_deviation * weights.finger_axis_time;
    if key_lookup.has(sd.last_key) {
        let last = key_lookup.get(sd.last_key);
        duration += (ckey.y - last.y).abs() as f64 * weights.row_change_time;
    }
    duration += pckey.stretch_time;
    if ckey.shifted() {
        duration += 0.10;
    }
    duration
}
