use crate::*;

use super::key_cost::{roll_delta, same_row};
use super::util::{flags_if, monotonic_f64, monotonic_i32, push_flag};

pub(crate) fn sweep_direction(keys: &[Key; 4]) -> i32 {
    if !keys.iter().all(|key| key.hand == keys[0].hand) {
        return 0;
    }
    if !same_row(keys[0], keys[1]) || !same_row(keys[1], keys[2]) || !same_row(keys[2], keys[3]) {
        return 0;
    }
    let seqs = [
        keys[0].finger.seq(),
        keys[1].finger.seq(),
        keys[2].finger.seq(),
        keys[3].finger.seq(),
    ];
    let xs = [keys[0].x, keys[1].x, keys[2].x, keys[3].x];
    if seqs.windows(2).any(|w| (w[1] - w[0]).abs() != 1) {
        return 0;
    }
    if xs.windows(2).any(|w| (w[1] - w[0]).abs() > 1.35) {
        return 0;
    }
    let seq_dir = monotonic_i32(&seqs);
    let x_dir = monotonic_f64(&xs);
    if seq_dir > 0 && x_dir > 0 {
        1
    } else if seq_dir < 0 && x_dir < 0 {
        -1
    } else {
        0
    }
}

pub(crate) fn two_key_roll_direction(keys: &[Key; 2]) -> i32 {
    // Physical direction of a short same-row roll. This supports the
    // FD+ASDF vs DF+ASDF distinction documented in IDEA.md: a short prefix
    // should not turn against the sweep motor program it is about to enter.
    let a = keys[0];
    let b = keys[1];
    if a.hand != b.hand || !same_row(a, b) {
        return 0;
    }
    let delta = roll_delta(a, b);
    if delta.abs() != 1 {
        return 0;
    }
    let x_dir = if b.x > a.x {
        1
    } else if b.x < a.x {
        -1
    } else {
        0
    };
    if delta > 0 && x_dir > 0 {
        1
    } else if delta < 0 && x_dir < 0 {
        -1
    } else {
        0
    }
}

pub(crate) fn pre_sweep_direction_adjustment(
    candidate_text: &str,
    candidate_len: usize,
    layout: &Layout,
    weights: &Weights,
    record_flags: bool,
) -> (f64, Vec<&'static str>) {
    if candidate_len < 6 {
        return (0.0, Vec::new());
    }
    let chars: Vec<char> = candidate_text.chars().collect();
    let n = chars.len();
    let Some(p0) = layout.key(chars[n - 6]) else {
        return (0.0, Vec::new());
    };
    let Some(p1) = layout.key(chars[n - 5]) else {
        return (0.0, Vec::new());
    };
    let Some(s0) = layout.key(chars[n - 4]) else {
        return (0.0, Vec::new());
    };
    let Some(s1) = layout.key(chars[n - 3]) else {
        return (0.0, Vec::new());
    };
    let Some(s2) = layout.key(chars[n - 2]) else {
        return (0.0, Vec::new());
    };
    let Some(s3) = layout.key(chars[n - 1]) else {
        return (0.0, Vec::new());
    };

    let prefix_dir = two_key_roll_direction(&[p0, p1]);
    let suffix_dir = sweep_direction(&[s0, s1, s2, s3]);
    if prefix_dir == 0 || suffix_dir == 0 {
        return (0.0, Vec::new());
    }
    if prefix_dir != suffix_dir {
        (
            weights.pre_sweep_direction_change_penalty,
            flags_if(record_flags, "pre-sweep-direction-change"),
        )
    } else {
        (
            weights.pre_sweep_direction_match_reward,
            flags_if(record_flags, "pre-sweep-direction-match"),
        )
    }
}

pub(crate) fn pair_direction_continuity_adjustment(
    keys: &[Key; 4],
    weights: &Weights,
    record_flags: bool,
) -> (f64, Vec<&'static str>) {
    // Do not compare direction pairs across a row-transition bridge. FEWAS has
    // EW and AS, but W->A is a real split-row transition rather than FDAS-like
    // same-row reversal.
    if !same_row(keys[1], keys[2]) {
        return (0.0, Vec::new());
    }
    let prev_dir = two_key_roll_direction(&[keys[0], keys[1]]);
    let cur_dir = two_key_roll_direction(&[keys[2], keys[3]]);
    if prev_dir == 0 || cur_dir == 0 {
        return (0.0, Vec::new());
    }
    if prev_dir != cur_dir {
        (
            weights.pre_sweep_direction_change_penalty * 0.72,
            flags_if(record_flags, "pair-direction-change"),
        )
    } else {
        (
            weights.pre_sweep_direction_match_reward,
            flags_if(record_flags, "pair-direction-match"),
        )
    }
}

pub(crate) fn upper_reverse_coupled_trigram(keys: &[Key; 3]) -> bool {
    // FEW-like upper reverse coupling: after F->E, the ring finger is already
    // dragged toward W. This models the user's observation without treating
    // every mixed-row block as easy.
    keys.iter().all(|key| key.hand == Hand::Left)
        && [
            keys[0].finger.seq(),
            keys[1].finger.seq(),
            keys[2].finger.seq(),
        ] == [3, 2, 1]
        && (keys[0].y - 1.0).abs() < 0.1
        && keys[1].y < 0.5
        && keys[2].y < 0.5
        && keys[0].x > keys[1].x
        && keys[1].x > keys[2].x
}

pub(crate) fn upper_reverse_split_sweep(keys: &[Key; 4]) -> bool {
    // FEWQ/FEWA: an FDSA-like split-row reverse sweep. The first three keys
    // are a coupled upper/home roll, and the pinky target is Q or a slight drop
    // to A. This should be mildly harder than FDSA, not a generic mixed-row
    // failure like AWDF.
    if !keys.iter().all(|key| key.hand == Hand::Left) {
        return false;
    }
    if [
        keys[0].finger.seq(),
        keys[1].finger.seq(),
        keys[2].finger.seq(),
        keys[3].finger.seq(),
    ] != [3, 2, 1, 0]
    {
        return false;
    }
    if !(keys[0].x > keys[1].x && keys[1].x > keys[2].x && keys[2].x > keys[3].x) {
        return false;
    }
    let rows = [
        keys[0].y.round(),
        keys[1].y.round(),
        keys[2].y.round(),
        keys[3].y.round(),
    ];
    rows == [1.0, 0.0, 0.0, 0.0] || rows == [1.0, 0.0, 0.0, 1.0]
}

pub(crate) fn row_actuation_mismatch(keys: &[Key; 4]) -> f64 {
    let ys = [keys[0].y, keys[1].y, keys[2].y, keys[3].y];
    let spread = ys.iter().fold(f64::NEG_INFINITY, |acc, y| acc.max(*y))
        - ys.iter().fold(f64::INFINITY, |acc, y| acc.min(*y));
    let mut best_modal = 0usize;
    for y in ys {
        let count = ys.iter().filter(|other| (**other - y).abs() < 0.1).count();
        best_modal = best_modal.max(count);
    }
    let odd_fraction = (4 - best_modal) as f64 / 4.0;
    spread + odd_fraction
}

pub(crate) fn four_finger_ordered_block(keys: &[Key; 4]) -> bool {
    if !keys.iter().all(|key| key.hand == keys[0].hand) {
        return false;
    }
    let fingers = [
        keys[0].finger,
        keys[1].finger,
        keys[2].finger,
        keys[3].finger,
    ];
    for i in 0..fingers.len() {
        for right in &fingers[i + 1..] {
            if fingers[i] == *right {
                return false;
            }
        }
    }
    let seqs = [
        keys[0].finger.seq(),
        keys[1].finger.seq(),
        keys[2].finger.seq(),
        keys[3].finger.seq(),
    ];
    if monotonic_i32(&seqs) == 0 {
        return false;
    }
    seqs.windows(2).all(|w| (w[1] - w[0]).abs() == 1)
}

pub(crate) fn uniform_motor_program(keys: &[Key; 4]) -> bool {
    if !four_finger_ordered_block(keys) {
        return false;
    }
    let ys = [keys[0].y, keys[1].y, keys[2].y, keys[3].y];
    let xs = [keys[0].x, keys[1].x, keys[2].x, keys[3].x];
    let y_spread = ys.iter().fold(f64::NEG_INFINITY, |acc, y| acc.max(*y))
        - ys.iter().fold(f64::INFINITY, |acc, y| acc.min(*y));
    y_spread <= 0.10 && xs.windows(2).all(|w| (w[1] - w[0]).abs() <= 1.35)
}

pub(crate) fn mixed_motor_program_mismatch(keys: &[Key; 4]) -> f64 {
    if !four_finger_ordered_block(keys) || uniform_motor_program(keys) {
        return 0.0;
    }
    row_actuation_mismatch(keys)
}

pub(crate) fn cognitive_pattern_adjustment(
    prev2: Option<Key>,
    prev: Option<Key>,
    last: Option<Key>,
    key: Key,
    settings: &Settings,
    record_flags: bool,
) -> (f64, Vec<&'static str>) {
    let weights = &settings.weights;
    let mut flags = Vec::new();
    let mut bonus: f64 = 0.0;

    if let (Some(a), Some(b), Some(c)) = (prev2, prev, last) {
        let quad = [a, b, c, key];
        if sweep_direction(&quad) > 0 {
            bonus += -weights.cognitive_sweep_reward;
            push_flag(&mut flags, record_flags, "cognitive:forward-sweep");
        }
        if physical_matches(&quad, b"asdf")
            || physical_matches(&quad, b"qwer")
            || physical_matches(&quad, b"zxcv")
            || physical_matches(&quad, b"1234")
            || physical_matches(&quad, b"fewa")
            || physical_matches(&quad, b"fewq")
        {
            bonus += -weights.cognitive_known_walk_reward;
            push_flag(&mut flags, record_flags, "cognitive:known-walk");
        }
    }

    if let (Some(a), Some(b)) = (prev, last) {
        let trigram = [a, b, key];
        if physical_matches(&trigram, b"wef")
            || physical_matches(&trigram, b"few")
            || physical_matches(&trigram, b"wer")
            || physical_matches(&trigram, b"qaz")
            || physical_matches(&trigram, b"wsx")
            || physical_matches(&trigram, b"edc")
            || physical_matches(&trigram, b"zxc")
            || physical_matches(&trigram, b"xcv")
        {
            bonus += -weights.cognitive_compact_reward;
            push_flag(&mut flags, record_flags, "cognitive:compact-or-column");
        }
        if physical_matches(&trigram, b"qaz")
            || physical_matches(&trigram, b"wsx")
            || physical_matches(&trigram, b"edc")
        {
            bonus += -weights.cognitive_vertical_reward;
            push_flag(&mut flags, record_flags, "cognitive:vertical-walk");
        }
    }

    bonus = bonus.min(weights.cognitive_cap_per_step);
    (-bonus, flags)
}

pub(crate) fn physical_matches<const N: usize>(keys: &[Key; N], pattern: &[u8; N]) -> bool {
    keys.iter()
        .zip(pattern)
        .all(|(key, expected)| key.physical as u8 == *expected)
}
