use super::key::*;
use super::state::has_key;
use crate::layout::F32Pos;
use crate::model::*;
use crate::*;

pub(super) struct FastStep {
    pub(super) total: f64,
    pub(super) dynamic_cost: f64,
    pub(super) movement_time: f64,
    pub(super) press_time: f64,
}

/// Precomputed per-key data that doesn't change across beam states.
/// Computed once per depth and passed to scoring, avoiding repeated
/// layout.home_pos(), strength_penalty(), row_penalty(), etc.
pub(super) struct PrecomputedKey {
    pub(super) rest: F32Pos, // f32 rest position for f32_adjusted_rest_f32 and push_with_pckeys
    pub(super) axis_deviation: f64,
    pub(super) stretch_time: f64,
    /// Precomputed: base_key + rest_distance * rest_dist + strength + row + axis(if>0.08) + stretch
    pub(super) base_static_cost: f64,
    /// Precomputed: shift + left_shift_right_reach if applicable
    pub(super) shift_cost: f64,
    // CompactKey data precomputed per key per depth:
    pub(super) ckey: CompactKey,
    pub(super) finger_idx: usize,
    pub(super) hand_idx: usize,
    pub(super) is_pinky_start: bool,
    pub(super) finger_recovery_factor: f64, // precomputed finger_repeat_factor
    pub(super) finger_base_recovery: f64,   // precomputed weights.finger_recovery * factor
    pub(super) rhythm_cost: f64,            // precomputed prefer_hand penalty
}

pub(super) fn precompute_keys(
    keys: &[Key],
    layout: &Layout,
    w: &Weights,
    prefer_hand: &crate::input::PreferHand,
) -> Vec<PrecomputedKey> {
    keys.iter()
        .map(|key| {
            let rest = layout.home_pos(key.finger);
            let rest_f32 = F32Pos {
                x: rest.x as f32,
                y: rest.y as f32,
            };
            let ckey = CompactKey::from_key(*key);
            let finger_idx = key.finger.idx();
            let hand_idx = key.hand.idx();
            let hand = key.hand;
            let axis_deviation = finger_axis_deviation(layout, *key, w);
            let stretch_penalty = index_stretch_penalty(*key, w);
            let base_static_cost = w.base_key
                + w.rest_distance * rest.dist_to(*key)
                + key.finger.strength_penalty()
                + row_penalty(*key, w)
                + if axis_deviation > 0.08 {
                    axis_deviation
                } else {
                    0.0
                }
                + if stretch_penalty > 0.0 {
                    stretch_penalty
                } else {
                    0.0
                };
            let shift_cost = if key.shifted {
                let mut sc = w.shift;
                if key.hand.idx() == 0 {
                    sc += (key.x - 2.75).max(0.0) * w.left_shift_right_reach;
                }
                sc
            } else {
                0.0
            };
            PrecomputedKey {
                rest: rest_f32,
                axis_deviation,
                stretch_time: key_stretch_time(*key, w),
                base_static_cost,
                shift_cost,
                ckey,
                finger_idx,
                hand_idx,
                is_pinky_start: matches!(key.finger, Finger::LeftPinky | Finger::RightPinky),
                finger_recovery_factor: finger_repeat_factor(key.finger, w),
                finger_base_recovery: w.finger_recovery * finger_repeat_factor(key.finger, w),
                rhythm_cost: prefer_hand.penalty_for(hand, w),
                // hand field removed: hand_idx used instead
            }
        })
        .collect()
}

/// Build a 256×256 transition base cost table.
/// Indexed by (last_physical, current_physical) as u8 bytes.
/// Contains the state-independent transition cost: same_key, same_finger + dist,
/// same_hand + roll + stretch. Hand-switch rhythm cost is NOT included.
pub(super) fn build_transition_base_table(
    keys: &[Key],
    pckeys: &[PrecomputedKey],
    w: &Weights,
) -> [[f64; 256]; 256] {
    let mut table = [[0.0f64; 256]; 256];
    for (ki, _key) in keys.iter().enumerate() {
        let pckey = &pckeys[ki];
        let ckey = pckey.ckey;
        for (lj, _last_key) in keys.iter().enumerate() {
            let last = pckeys[lj].ckey;
            let mut cost = 0.0;
            if last.physical == ckey.physical {
                cost += w.same_key_motion * pckey.finger_recovery_factor;
            } else if last.finger == ckey.finger {
                let dist = {
                    let dx = (last.x - ckey.x) as f64;
                    let dy = (last.y - ckey.y) as f64;
                    (dx * dx + dy * dy).sqrt()
                };
                cost += (w.same_finger_motion + 0.22 * dist) * pckey.finger_recovery_factor;
            } else if last.hand() == ckey.hand() {
                cost += w.same_hand;
                let x_gap = (ckey.x - last.x).abs() as f64;
                if ck_is_adjacent_roll(last, ckey) {
                    cost += w.adjacent_roll_reward;
                    if x_gap > 1.35 {
                        cost += w.adjacent_roll_long_gap_penalty * (x_gap - 1.35);
                    }
                    if ck_is_forward_physical_roll(last, ckey) {
                        cost += w.forward_roll_reward;
                    } else {
                        cost += w.reverse_roll_penalty;
                    }
                } else if x_gap > 1.7 {
                    cost += w.lateral_stretch;
                }
            }
            table[last.physical as usize][ckey.physical as usize] = cost;
        }
    }
    table
}

/// Build a byte→CompactKey lookup table from the key set.
/// Used by the motor-program-repeat check to avoid layout.key() + from_key().
pub(super) fn build_compact_key_table(keys: &[Key]) -> [Option<CompactKey>; 256] {
    let mut table = [None; 256];
    for key in keys {
        let ckey = CompactKey::from_key(*key);
        table[key.typed as usize] = Some(ckey);
        // Also index by physical key for characters that differ
        if (key.physical as usize) != key.typed as usize {
            // Don't overwrite if already set (e.g., shifted variants)
            if table[key.physical as usize].is_none() {
                table[key.physical as usize] = Some(ckey);
            }
        }
    }
    table
}

/// Variant of ck_cognitive_pattern that takes CompactKey values instead of Option<CompactKey>.
/// Uses has_key() to check for presence.
#[inline]
pub(super) fn ck_cognitive_pattern_val(
    prev2: CompactKey,
    prev: CompactKey,
    last: CompactKey,
    key: CompactKey,
    w: &Weights,
) -> f64 {
    let mut bonus: f64 = 0.0;
    if has_key(prev2) && has_key(prev) && has_key(last) {
        let quad = [prev2, prev, last, key];
        if ck_sweep_direction(&quad) > 0 {
            bonus += -w.cognitive_sweep_reward;
        }
        if ck_physical_matches(&quad, b"asdf")
            || ck_physical_matches(&quad, b"qwer")
            || ck_physical_matches(&quad, b"zxcv")
            || ck_physical_matches(&quad, b"1234")
            || ck_physical_matches(&quad, b"fewa")
            || ck_physical_matches(&quad, b"fewq")
        {
            bonus += -w.cognitive_known_walk_reward;
        }
    }
    if has_key(prev) && has_key(last) {
        let trigram = [prev, last, key];
        if ck_physical_matches(&trigram, b"wef")
            || ck_physical_matches(&trigram, b"few")
            || ck_physical_matches(&trigram, b"wer")
            || ck_physical_matches(&trigram, b"qaz")
            || ck_physical_matches(&trigram, b"wsx")
            || ck_physical_matches(&trigram, b"edc")
            || ck_physical_matches(&trigram, b"zxc")
            || ck_physical_matches(&trigram, b"xcv")
        {
            bonus += -w.cognitive_compact_reward;
        }
        if ck_physical_matches(&trigram, b"qaz")
            || ck_physical_matches(&trigram, b"wsx")
            || ck_physical_matches(&trigram, b"edc")
        {
            bonus += -w.cognitive_vertical_reward;
        }
    }
    bonus = bonus.min(w.cognitive_cap_per_step);
    -bonus
}
