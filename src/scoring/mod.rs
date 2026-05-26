mod cost;
mod state;
mod types;

pub use types::{ScoreReport, StepCost, TypingState};

use crate::*;

pub fn score_sequence(
    layout: &Layout,
    settings: &Settings,
    sequence: &str,
) -> Result<ScoreReport, String> {
    let mut state = TypingState::new(layout);
    for ch in sequence.chars() {
        let key = layout
            .key(ch)
            .ok_or_else(|| format!("unsupported char: {ch:?}"))?;
        state = state.push(layout, key, settings);
    }
    Ok(ScoreReport {
        sequence: sequence.to_string(),
        total: state.score,
        average: state.average(),
        steps: state.steps,
    })
}
