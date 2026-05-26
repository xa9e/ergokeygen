use std::collections::HashMap;

use crate::{Finger, Hand, PreferHand, RhythmMode, Weights};

#[derive(Debug, Clone)]
pub struct Settings {
    pub weights: Weights,
    pub mode: RhythmMode,
    pub prefer_hand: PreferHand,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            weights: Weights::default(),
            mode: RhythmMode::OneHand,
            prefer_hand: PreferHand::Any,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Pos {
    pub x: f64,
    pub y: f64,
}

impl Pos {
    pub(crate) fn dist_to(self, key: Key) -> f64 {
        let dx = self.x - key.x;
        let dy = self.y - key.y;
        (dx * dx + dy * dy).sqrt()
    }

    #[allow(dead_code)]
    pub(crate) fn dist_to_xy(self, x: f64, y: f64) -> f64 {
        let dx = self.x - x;
        let dy = self.y - y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PalmPosture {
    pub offset_x: f64,
    pub offset_y: f64,
    pub rotation: f64,
    pub tension: f64,
}

impl PalmPosture {
    pub fn adjusted(self, pos: Pos, hand: Hand) -> Pos {
        let anchor_x = match hand {
            Hand::Left => 3.0,
            Hand::Right => 7.5,
        };
        let anchor_y = 1.0;
        let dx = pos.x - anchor_x;
        let dy = pos.y - anchor_y;
        let rot = match hand {
            Hand::Left => self.rotation,
            Hand::Right => -self.rotation,
        };
        Pos {
            x: pos.x + self.offset_x - rot * dy,
            y: pos.y + self.offset_y + rot * dx,
        }
    }
}

// ── f32 compact types for FastState ──────────────────────────────────
// Keyboard coordinates are small values (0-12 for x, -1 to 2 for y).
// f32 has plenty of precision for this range and halves the memory
// footprint per state, improving cache behavior in beam expansion.

#[derive(Clone, Copy, Default)]
pub(crate) struct F32Pos {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

impl F32Pos {
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn dist_to_key(self, key: Key) -> f64 {
        let dx = self.x as f64 - key.x;
        let dy = self.y as f64 - key.y;
        (dx * dx + dy * dy).sqrt()
    }

    #[inline]
    pub(crate) fn dist_to_xy_f32(self, x: f32, y: f32) -> f64 {
        let dx = self.x as f64 - x as f64;
        let dy = self.y as f64 - y as f64;
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct F32PalmPosture {
    pub(crate) offset_x: f32,
    pub(crate) offset_y: f32,
    pub(crate) rotation: f32,
    pub(crate) tension: f32,
}

#[inline]
pub(crate) fn f64_to_f32(v: f64) -> f32 {
    v as f32
}

#[inline]
#[allow(dead_code)]
pub(crate) fn f32_adjusted_rest(posture: F32PalmPosture, pos: Pos, hand: Hand) -> Pos {
    let anchor_x: f64 = match hand {
        Hand::Left => 3.0,
        Hand::Right => 7.5,
    };
    let anchor_y: f64 = 1.0;
    let dx = pos.x - anchor_x;
    let dy = pos.y - anchor_y;
    let rot: f64 = match hand {
        Hand::Left => posture.rotation as f64,
        Hand::Right => -(posture.rotation as f64),
    };
    Pos {
        x: pos.x + posture.offset_x as f64 - rot * dy,
        y: pos.y + posture.offset_y as f64 + rot * dx,
    }
}

/// f32 variant of f32_adjusted_rest that takes F32Pos rest and hand_idx.
/// Avoids f64↔f32 conversions in the hot path.
#[inline]
pub(crate) fn f32_adjusted_rest_f32(
    posture: F32PalmPosture,
    rest: F32Pos,
    hand_idx: usize,
) -> F32Pos {
    let anchor_x: f32 = if hand_idx == 0 { 3.0 } else { 7.5 };
    let anchor_y: f32 = 1.0;
    let dx = rest.x - anchor_x;
    let dy = rest.y - anchor_y;
    let rot: f32 = if hand_idx == 0 {
        posture.rotation
    } else {
        -posture.rotation
    };
    F32Pos {
        x: rest.x + posture.offset_x - rot * dy,
        y: rest.y + posture.offset_y + rot * dx,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Key {
    pub typed: char,
    pub physical: char,
    pub x: f64,
    pub y: f64,
    pub finger: Finger,
    pub hand: Hand,
    pub shifted: bool,
}

#[derive(Debug, Clone)]
pub struct Layout {
    keys: HashMap<char, Key>,
    home: [Pos; 8],
}

impl Layout {
    pub fn ansi_qwerty() -> Self {
        let mut layout = Self {
            keys: HashMap::new(),
            home: [Pos::default(); 8],
        };

        layout.home[Finger::LeftPinky.idx()] = Pos { x: 0.52, y: 0.78 };
        layout.home[Finger::LeftRing.idx()] = Pos { x: 1.75, y: 0.80 };
        layout.home[Finger::LeftMiddle.idx()] = Pos { x: 2.62, y: 0.52 };
        layout.home[Finger::LeftIndex.idx()] = Pos { x: 3.75, y: 0.90 };
        layout.home[Finger::RightIndex.idx()] = Pos { x: 6.75, y: 0.90 };
        layout.home[Finger::RightMiddle.idx()] = Pos { x: 7.88, y: 0.52 };
        layout.home[Finger::RightRing.idx()] = Pos { x: 8.75, y: 0.80 };
        layout.home[Finger::RightPinky.idx()] = Pos { x: 9.98, y: 0.78 };

        layout.insert_rows();
        layout.insert_shifted();
        layout
    }

    fn insert_key(
        &mut self,
        typed: char,
        physical: char,
        x: f64,
        y: f64,
        finger: Finger,
        shifted: bool,
    ) {
        self.keys.insert(
            typed,
            Key {
                typed,
                physical,
                x,
                y,
                finger,
                hand: finger.hand(),
                shifted,
            },
        );
    }

    fn insert_rows(&mut self) {
        let rows: Vec<(&str, f64, f64, Vec<Finger>)> = vec![
            (
                "`1234567890-=",
                0.0,
                -1.0,
                vec![
                    Finger::LeftPinky,
                    Finger::LeftPinky,
                    Finger::LeftRing,
                    Finger::LeftMiddle,
                    Finger::LeftIndex,
                    Finger::LeftIndex,
                    Finger::RightIndex,
                    Finger::RightIndex,
                    Finger::RightMiddle,
                    Finger::RightRing,
                    Finger::RightPinky,
                    Finger::RightPinky,
                    Finger::RightPinky,
                ],
            ),
            (
                "qwertyuiop[]\\",
                0.5,
                0.0,
                vec![
                    Finger::LeftPinky,
                    Finger::LeftRing,
                    Finger::LeftMiddle,
                    Finger::LeftIndex,
                    Finger::LeftIndex,
                    Finger::RightIndex,
                    Finger::RightIndex,
                    Finger::RightMiddle,
                    Finger::RightRing,
                    Finger::RightPinky,
                    Finger::RightPinky,
                    Finger::RightPinky,
                    Finger::RightPinky,
                ],
            ),
            (
                "asdfghjkl;'",
                0.75,
                1.0,
                vec![
                    Finger::LeftPinky,
                    Finger::LeftRing,
                    Finger::LeftMiddle,
                    Finger::LeftIndex,
                    Finger::LeftIndex,
                    Finger::RightIndex,
                    Finger::RightIndex,
                    Finger::RightMiddle,
                    Finger::RightRing,
                    Finger::RightPinky,
                    Finger::RightPinky,
                ],
            ),
            (
                "zxcvbnm,./",
                1.25,
                2.0,
                vec![
                    Finger::LeftPinky,
                    Finger::LeftRing,
                    Finger::LeftMiddle,
                    Finger::LeftIndex,
                    Finger::LeftIndex,
                    Finger::RightIndex,
                    Finger::RightIndex,
                    Finger::RightMiddle,
                    Finger::RightRing,
                    Finger::RightPinky,
                ],
            ),
        ];

        for (labels, x0, y, fingers) in rows {
            for (index, physical) in labels.chars().enumerate() {
                self.insert_key(
                    physical,
                    physical,
                    x0 + index as f64,
                    y,
                    fingers[index],
                    false,
                );
            }
        }
    }

    fn insert_shifted(&mut self) {
        let shifted_pairs = [
            ('~', '`'),
            ('!', '1'),
            ('@', '2'),
            ('#', '3'),
            ('$', '4'),
            ('%', '5'),
            ('^', '6'),
            ('&', '7'),
            ('*', '8'),
            ('(', '9'),
            (')', '0'),
            ('_', '-'),
            ('+', '='),
            ('{', '['),
            ('}', ']'),
            ('|', '\\'),
            (':', ';'),
            ('"', '\''),
            ('<', ','),
            ('>', '.'),
            ('?', '/'),
        ];

        for (typed, physical) in shifted_pairs {
            if let Some(base) = self.keys.get(&physical).copied() {
                self.insert_key(typed, physical, base.x, base.y, base.finger, true);
            }
        }

        for lower in 'a'..='z' {
            if let Some(base) = self.keys.get(&lower).copied() {
                self.insert_key(
                    lower.to_ascii_uppercase(),
                    lower,
                    base.x,
                    base.y,
                    base.finger,
                    true,
                );
            }
        }
    }

    pub fn key(&self, c: char) -> Option<Key> {
        self.keys.get(&c).copied()
    }

    pub fn contains(&self, c: char) -> bool {
        self.keys.contains_key(&c)
    }

    pub fn home_pos(&self, finger: Finger) -> Pos {
        self.home[finger.idx()]
    }
}
