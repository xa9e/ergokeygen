use crate::Weights;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hand {
    Left,
    Right,
}

impl Hand {
    pub fn idx(self) -> usize {
        match self {
            Hand::Left => 0,
            Hand::Right => 1,
        }
    }

    pub fn other(self) -> Self {
        match self {
            Hand::Left => Hand::Right,
            Hand::Right => Hand::Left,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Finger {
    LeftPinky = 0,
    LeftRing = 1,
    LeftMiddle = 2,
    LeftIndex = 3,
    RightIndex = 4,
    RightMiddle = 5,
    RightRing = 6,
    RightPinky = 7,
}

impl Finger {
    #[inline]
    pub(crate) fn from_u8(v: u8) -> Self {
        match v {
            0 => Finger::LeftPinky,
            1 => Finger::LeftRing,
            2 => Finger::LeftMiddle,
            3 => Finger::LeftIndex,
            4 => Finger::RightIndex,
            5 => Finger::RightMiddle,
            6 => Finger::RightRing,
            7 => Finger::RightPinky,
            _ => Finger::LeftPinky,
        }
    }

    pub const ALL: [Finger; 8] = [
        Finger::LeftPinky,
        Finger::LeftRing,
        Finger::LeftMiddle,
        Finger::LeftIndex,
        Finger::RightIndex,
        Finger::RightMiddle,
        Finger::RightRing,
        Finger::RightPinky,
    ];

    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn hand(self) -> Hand {
        match self {
            Finger::LeftPinky | Finger::LeftRing | Finger::LeftMiddle | Finger::LeftIndex => {
                Hand::Left
            }
            Finger::RightIndex | Finger::RightMiddle | Finger::RightRing | Finger::RightPinky => {
                Hand::Right
            }
        }
    }

    pub fn seq(self) -> i32 {
        match self {
            Finger::LeftPinky => 0,
            Finger::LeftRing => 1,
            Finger::LeftMiddle => 2,
            Finger::LeftIndex => 3,
            Finger::RightIndex => 0,
            Finger::RightMiddle => 1,
            Finger::RightRing => 2,
            Finger::RightPinky => 3,
        }
    }

    pub fn strength_penalty(self) -> f64 {
        match self {
            Finger::LeftIndex | Finger::RightIndex => 0.00,
            Finger::LeftMiddle | Finger::RightMiddle => 0.06,
            Finger::LeftRing | Finger::RightRing => 0.22,
            Finger::LeftPinky | Finger::RightPinky => 0.68,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RhythmMode {
    OneHand,
    Neutral,
    Alternation,
}

impl RhythmMode {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "onehand" | "one-hand" | "samehand" | "same-hand" => Ok(Self::OneHand),
            "neutral" | "mixed" => Ok(Self::Neutral),
            "alternation" | "alternate" | "alt" => Ok(Self::Alternation),
            _ => Err(format!("unknown rhythm mode: {raw}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferHand {
    Any,
    Left,
    Right,
}

impl PreferHand {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "any" | "none" => Ok(Self::Any),
            "left" | "l" => Ok(Self::Left),
            "right" | "r" => Ok(Self::Right),
            _ => Err(format!("unknown hand preference: {raw}")),
        }
    }

    pub(crate) fn penalty_for(self, hand: Hand, weights: &Weights) -> f64 {
        match (self, hand) {
            (PreferHand::Any, _) => 0.0,
            (PreferHand::Left, Hand::Left) | (PreferHand::Right, Hand::Right) => 0.0,
            _ => weights.prefer_hand_penalty,
        }
    }
}
