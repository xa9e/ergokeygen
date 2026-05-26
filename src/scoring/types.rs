use crate::*;

#[derive(Debug, Clone)]
pub struct StepCost {
    pub typed: char,
    pub total: f64,
    pub static_cost: f64,
    pub dynamic_cost: f64,
    pub transition_cost: f64,
    pub rhythm_cost: f64,
    pub timing_cost: f64,
    pub cognitive_cost: f64,
    pub timing_wait: f64,
    pub press_time: f64,
    pub movement_time: f64,
    pub flags: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct ScoreReport {
    pub sequence: String,
    pub total: f64,
    pub average: f64,
    pub steps: Vec<StepCost>,
}

#[derive(Debug, Clone)]
pub struct TypingState {
    pub text: String,
    pub text_len: usize,
    pub score: f64,
    pub positions: [Pos; 8],
    pub postures: [PalmPosture; 2],
    pub finger_ready: [f64; 8],
    pub time: f64,
    pub last: Option<Key>,
    pub prev: Option<Key>,
    pub prev2: Option<Key>,
    pub row_run_y: Option<f64>,
    pub row_run_len: usize,
    pub steps: Vec<StepCost>,
}
