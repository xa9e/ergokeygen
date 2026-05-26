use std::char;

use crate::model::{monotonic_f64, monotonic_i32};
use crate::{Finger, Hand, Key, Weights};

// ── Compact generation state (FastV1) ──────────────────────────────────
// TypingState is shared by scoring and generation. During generation,
// `steps: Vec<StepCost>` is always empty and Option<Key> costs 24 bytes
// each. FastState stores only what beam expansion and ordering need.

#[derive(Clone, Copy, Debug)]
pub(super) struct CompactKey {
    pub(super) typed: u8, // needed for emission, not scoring; char is ASCII for keyboard
    pub(super) physical: u8, // used in scoring
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) finger: u8, // Finger as u8 (0-7)
    /// bit 0: hand (0=Left, 1=Right), bit 1: shifted
    pub(super) flags: u8,
}
// 12 bytes (align 4) vs 16 bytes with separate hand+shifted. Saves 4B per CompactKey, 12B per FastState.

impl CompactKey {
    #[inline]
    pub(super) fn from_key(k: Key) -> Self {
        Self {
            typed: k.typed as u8,
            physical: k.physical as u8,
            x: k.x as f32,
            y: k.y as f32,
            finger: k.finger as u8,
            flags: (k.hand.idx() as u8) | ((k.shifted as u8) << 1),
        }
    }

    #[inline]
    pub(super) fn hand(self) -> u8 {
        self.flags & 1
    }

    #[inline]
    pub(super) fn shifted(self) -> bool {
        self.flags & 2 != 0
    }

    #[inline]
    #[allow(dead_code)]
    pub(super) fn to_key(self) -> Key {
        Key {
            typed: self.typed as char,
            physical: self.physical as char,
            x: self.x as f64,
            y: self.y as f64,
            finger: Finger::from_u8(self.finger),
            hand: if self.hand() == 0 {
                Hand::Left
            } else {
                Hand::Right
            },
            shifted: self.shifted(),
        }
    }
}

// ── CompactKey-native helper functions ────────────────────────────────
// Avoid .to_key() which copies 48 bytes. These check the same conditions
// using CompactKey fields directly.

/// Finger sequence number (0=pinky, 1=ring, 2=middle, 3=index) within hand.
/// Direct lookup instead of Finger::from_u8(finger).seq() which involves
/// a match chain through enum conversion.
#[inline]
pub(super) fn finger_seq(finger_u8: u8) -> i32 {
    // LeftPinky=0->0, LeftRing=1->1, LeftMiddle=2->2, LeftIndex=3->3,
    // RightIndex=4->0, RightMiddle=5->1, RightRing=6->2, RightPinky=7->3
    const TABLE: [i32; 8] = [0, 1, 2, 3, 0, 1, 2, 3];
    TABLE[finger_u8 as usize & 7]
}

/// Hand index from finger u8: 0=Left, 1=Right.
#[inline]
#[allow(dead_code)]
pub(super) fn finger_hand(finger_u8: u8) -> u8 {
    // LeftPinky..LeftIndex = 0..3 -> hand 0
    // RightIndex..RightPinky = 4..7 -> hand 1
    if finger_u8 < 4 {
        0
    } else {
        1
    }
}

#[inline]
pub(super) fn ck_same_row(a: CompactKey, b: CompactKey) -> bool {
    (a.y - b.y).abs() < 0.1
}

#[inline]
pub(super) fn ck_roll_delta(a: CompactKey, b: CompactKey) -> i32 {
    finger_seq(b.finger) - finger_seq(a.finger)
}

#[inline]
pub(super) fn ck_is_smooth_trigram(a: CompactKey, b: CompactKey, c: CompactKey) -> bool {
    if !(a.hand() == b.hand() && b.hand() == c.hand()) {
        return false;
    }
    let d1 = ck_roll_delta(a, b);
    let d2 = ck_roll_delta(b, c);
    d1.abs() == 1
        && d2.abs() == 1
        && d1 == d2
        && ((b.x as f64 - a.x as f64) - (c.x as f64 - b.x as f64)).abs() < 0.7
}

#[inline]
pub(super) fn ck_upper_reverse_coupled_trigram(
    a: CompactKey,
    b: CompactKey,
    c: CompactKey,
) -> bool {
    if a.hand() != 0 || b.hand() != 0 || c.hand() != 0 {
        return false;
    }
    [
        finger_seq(a.finger),
        finger_seq(b.finger),
        finger_seq(c.finger),
    ] == [3, 2, 1]
        && (a.y - 1.0f32).abs() < 0.1
        && b.y < 0.5
        && c.y < 0.5
        && a.x > b.x
        && b.x > c.x
}

#[inline]
pub(super) fn ck_is_compact_cluster(a: CompactKey, b: CompactKey, c: CompactKey) -> bool {
    if a.hand() != b.hand() || b.hand() != c.hand() {
        return false;
    }
    // Check all different fingers
    if a.finger == b.finger || a.finger == c.finger || b.finger == c.finger {
        return false;
    }
    let min_x = a.x.min(b.x).min(c.x);
    let max_x = a.x.max(b.x).max(c.x);
    let min_y = a.y.min(b.y).min(c.y);
    let max_y = a.y.max(b.y).max(c.y);
    (max_x - min_x) <= 2.75 && (max_y - min_y) <= 1.15
}

#[inline]
pub(super) fn ck_is_adjacent_roll(a: CompactKey, b: CompactKey) -> bool {
    a.hand() == b.hand() && ck_roll_delta(a, b).abs() == 1
}

#[inline]
pub(super) fn ck_is_forward_physical_roll(a: CompactKey, b: CompactKey) -> bool {
    ck_roll_delta(a, b) > 0 && b.x > a.x
}

// ── CompactKey-native quad functions ────────────────────────────────
// These replace .to_key() calls in the fast path. They work on [CompactKey; 4]
// or [CompactKey; N] instead of [Key; N].

#[inline]
pub(super) fn ck_sweep_direction(q: &[CompactKey; 4]) -> i32 {
    if !(q[0].hand() == q[1].hand() && q[1].hand() == q[2].hand() && q[2].hand() == q[3].hand()) {
        return 0;
    }
    if !ck_same_row(q[0], q[1]) || !ck_same_row(q[1], q[2]) || !ck_same_row(q[2], q[3]) {
        return 0;
    }
    let seqs = [
        finger_seq(q[0].finger),
        finger_seq(q[1].finger),
        finger_seq(q[2].finger),
        finger_seq(q[3].finger),
    ];
    let xs = [q[0].x as f64, q[1].x as f64, q[2].x as f64, q[3].x as f64];
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

#[inline]
pub(super) fn ck_two_key_roll_direction(a: CompactKey, b: CompactKey) -> i32 {
    if a.hand() != b.hand() || !ck_same_row(a, b) {
        return 0;
    }
    let delta = ck_roll_delta(a, b);
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

#[inline]
pub(super) fn ck_four_finger_ordered_block(q: &[CompactKey; 4]) -> bool {
    if !(q[0].hand() == q[1].hand() && q[1].hand() == q[2].hand() && q[2].hand() == q[3].hand()) {
        return false;
    }
    if q[0].finger == q[1].finger
        || q[0].finger == q[2].finger
        || q[0].finger == q[3].finger
        || q[1].finger == q[2].finger
        || q[1].finger == q[3].finger
        || q[2].finger == q[3].finger
    {
        return false;
    }
    let seqs = [
        finger_seq(q[0].finger),
        finger_seq(q[1].finger),
        finger_seq(q[2].finger),
        finger_seq(q[3].finger),
    ];
    if monotonic_i32(&seqs) == 0 {
        return false;
    }
    seqs.windows(2).all(|w| (w[1] - w[0]).abs() == 1)
}

#[inline]
pub(super) fn ck_row_actuation_mismatch(q: &[CompactKey; 4]) -> f64 {
    let ys = [q[0].y as f64, q[1].y as f64, q[2].y as f64, q[3].y as f64];
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

#[inline]
pub(super) fn ck_uniform_motor_program(q: &[CompactKey; 4]) -> bool {
    if !ck_four_finger_ordered_block(q) {
        return false;
    }
    let ys = [q[0].y as f64, q[1].y as f64, q[2].y as f64, q[3].y as f64];
    let xs = [q[0].x as f64, q[1].x as f64, q[2].x as f64, q[3].x as f64];
    let y_spread = ys.iter().fold(f64::NEG_INFINITY, |acc, y| acc.max(*y))
        - ys.iter().fold(f64::INFINITY, |acc, y| acc.min(*y));
    y_spread <= 0.10 && xs.windows(2).all(|w| (w[1] - w[0]).abs() <= 1.35)
}

#[inline]
pub(super) fn ck_mixed_motor_program_mismatch(q: &[CompactKey; 4]) -> f64 {
    if !ck_four_finger_ordered_block(q) || ck_uniform_motor_program(q) {
        return 0.0;
    }
    ck_row_actuation_mismatch(q)
}

#[inline]
pub(super) fn ck_upper_reverse_split_sweep(q: &[CompactKey; 4]) -> bool {
    if !(q[0].hand() == 0 && q[1].hand() == 0 && q[2].hand() == 0 && q[3].hand() == 0) {
        return false;
    }
    if [
        finger_seq(q[0].finger),
        finger_seq(q[1].finger),
        finger_seq(q[2].finger),
        finger_seq(q[3].finger),
    ] != [3, 2, 1, 0]
    {
        return false;
    }
    if !(q[0].x > q[1].x && q[1].x > q[2].x && q[2].x > q[3].x) {
        return false;
    }
    let rows = [
        q[0].y.round(),
        q[1].y.round(),
        q[2].y.round(),
        q[3].y.round(),
    ];
    rows == [1.0, 0.0, 0.0, 0.0] || rows == [1.0, 0.0, 0.0, 1.0]
}

#[inline]
pub(super) fn ck_physical_matches<const N: usize>(
    keys: &[CompactKey; N],
    pattern: &[u8; N],
) -> bool {
    keys.iter()
        .zip(pattern)
        .all(|(key, expected)| key.physical == *expected)
}

#[inline]
pub(super) fn ck_pair_direction_continuity(
    a0: CompactKey,
    a1: CompactKey,
    b0: CompactKey,
    b1: CompactKey,
    w: &Weights,
) -> f64 {
    if !ck_same_row(a1, b0) {
        return 0.0;
    }
    let prev_dir = ck_two_key_roll_direction(a0, a1);
    let cur_dir = ck_two_key_roll_direction(b0, b1);
    if prev_dir == 0 || cur_dir == 0 {
        return 0.0;
    }
    if prev_dir != cur_dir {
        w.pre_sweep_direction_change_penalty * 0.72
    } else {
        w.pre_sweep_direction_match_reward
    }
}

/// Tail-buffer variant of ck_pre_sweep_direction.
/// `tail` holds the last 8 bytes; `total_len` is the full text length.
/// Only needs the last 6 bytes.
#[inline]
pub(super) fn ck_pre_sweep_direction_tail(
    tail: &[u8],
    total_len: usize,
    ck_table: &[Option<CompactKey>; 256],
    w: &Weights,
) -> f64 {
    if total_len < 6 {
        return 0.0;
    }
    let offset = if total_len >= 8 {
        2
    } else {
        8 - total_len.min(8)
    };
    let last6 = &tail[offset..];
    if last6.len() < 6 {
        return 0.0;
    }
    let Some(p0) = ck_table[last6[0] as usize] else {
        return 0.0;
    };
    let Some(p1) = ck_table[last6[1] as usize] else {
        return 0.0;
    };
    let Some(s0) = ck_table[last6[2] as usize] else {
        return 0.0;
    };
    let Some(s1) = ck_table[last6[3] as usize] else {
        return 0.0;
    };
    let Some(s2) = ck_table[last6[4] as usize] else {
        return 0.0;
    };
    let Some(s3) = ck_table[last6[5] as usize] else {
        return 0.0;
    };
    let prefix_dir = ck_two_key_roll_direction(p0, p1);
    let suffix_dir = ck_sweep_direction(&[s0, s1, s2, s3]);
    if prefix_dir == 0 || suffix_dir == 0 {
        return 0.0;
    }
    if prefix_dir != suffix_dir {
        w.pre_sweep_direction_change_penalty
    } else {
        w.pre_sweep_direction_match_reward
    }
}
