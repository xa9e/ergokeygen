use crate::*;

use super::key_cost::{finger_axis_deviation, key_stretch_time};
use super::util::flags_if;

pub(crate) fn movement_duration(
    state: &TypingState,
    layout: &Layout,
    key: Key,
    dynamic_cost: f64,
    settings: &Settings,
) -> f64 {
    let weights = &settings.weights;
    let mut duration =
        weights.beat_interval + (dynamic_cost - 0.90).max(0.0) * weights.movement_time_factor;
    duration += finger_axis_deviation(layout, key, weights) * weights.finger_axis_time;
    if let Some(last) = state.last {
        duration += (key.y - last.y).abs() * weights.row_change_time;
    }
    duration += key_stretch_time(key, weights);
    if key.shifted {
        duration += 0.10;
    }
    duration
}

pub(crate) fn timing_state_cost(
    state: &TypingState,
    layout: &Layout,
    key: Key,
    dynamic_cost: f64,
    settings: &Settings,
) -> (f64, f64, f64, f64) {
    let weights = &settings.weights;
    let move_time = movement_duration(state, layout, key, dynamic_cost, settings);
    let planned = state.time + move_time;
    let mut wait = (state.finger_ready[key.finger.idx()] - planned).max(0.0);
    wait = home_return_wait_relief(state, key, wait, weights).0;
    let press_time = planned + wait;
    (wait * weights.timing_wait, wait, press_time, move_time)
}

pub(crate) fn upper_reverse_axis_relief(
    state: &TypingState,
    key: Key,
    axis_cost: f64,
    weights: &Weights,
    record_flags: bool,
) -> (f64, Vec<&'static str>) {
    if axis_cost <= 0.0 {
        return (0.0, Vec::new());
    }
    let Some(last) = state.last else {
        return (0.0, Vec::new());
    };
    if key.hand == Hand::Left
        && key.finger == Finger::LeftRing
        && key.y < 0.5
        && last.finger == Finger::LeftMiddle
        && last.y < 0.5
        && last.x > key.x
    {
        return (
            axis_cost.min(weights.upper_reverse_axis_relief),
            flags_if(record_flags, "upper-reverse-axis-relief"),
        );
    }
    (0.0, Vec::new())
}

pub(crate) fn home_return_wait_relief(
    state: &TypingState,
    key: Key,
    wait: f64,
    weights: &Weights,
) -> (f64, Vec<&'static str>) {
    if wait <= 0.0 {
        return (wait, Vec::new());
    }
    let Some(prev) = state.prev else {
        return (wait, Vec::new());
    };
    if prev.finger == key.finger
        && key.hand == prev.hand
        && (0.55..=1.45).contains(&key.y)
        && (prev.y - key.y).abs() >= 0.9
    {
        return (
            (wait - weights.home_return_wait_relief).max(0.0),
            vec!["finger-inertia:home-return-relief"],
        );
    }
    (wait, Vec::new())
}
