use std::fs;

use crate::Weights;

pub fn load_weights_config(path: &str, mut weights: Weights) -> Result<Weights, String> {
    let content =
        fs::read_to_string(path).map_err(|err| format!("failed to read config {path}: {err}"))?;
    let mut section = String::from("weights");

    for (lineno, raw) in content.lines().enumerate() {
        let line = clean_config_line(raw);
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1]
                .trim()
                .to_ascii_lowercase()
                .replace('-', "_");
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!("{path}:{}: expected key = value", lineno + 1));
        };
        let key = raw_key.trim().replace('-', "_").to_ascii_lowercase();
        let value = raw_value
            .trim()
            .replace('_', "")
            .parse::<f64>()
            .map_err(|_| format!("{path}:{}: invalid float: {}", lineno + 1, raw_value.trim()))?;
        let target = config_target_key(&section, &key).ok_or_else(|| {
            format!(
                "{path}:{}: unknown config key [{}].{}",
                lineno + 1,
                section,
                key
            )
        })?;
        set_weight_by_name(&mut weights, &target, value)
            .map_err(|err| format!("{path}:{}: {err}", lineno + 1))?;
    }

    Ok(weights)
}

fn clean_config_line(raw: &str) -> String {
    let mut line = raw;
    for sep in ['#', ';'] {
        if let Some((head, _)) = line.split_once(sep) {
            line = head;
        }
    }
    line.trim().to_string()
}

fn config_target_key(section: &str, key: &str) -> Option<String> {
    if section == "weights" {
        return Some(key.to_string());
    }
    let finger = section.strip_prefix("finger.")?;
    let prefix = match finger {
        "left_pinky" | "left_ring" | "left_middle" | "left_index" | "right_index"
        | "right_middle" | "right_ring" | "right_pinky" => finger,
        _ => return None,
    };
    match key {
        "axis_dx" => Some(format!("{prefix}_axis_dx")),
        "axis_dy" => Some(format!("{prefix}_axis_dy")),
        "lateral_factor" => Some(format!("{prefix}_lateral_factor")),
        "repeat_factor" => Some(format!("{prefix}_repeat_factor")),
        _ => None,
    }
}

fn set_weight_by_name(weights: &mut Weights, key: &str, value: f64) -> Result<(), String> {
    match key {
        "base_key" => weights.base_key = value,
        "rest_distance" => weights.rest_distance = value,
        "dynamic_distance" => weights.dynamic_distance = value,
        "palm_distance" => weights.palm_distance = value,
        "palm_tension" => weights.palm_tension = value,
        "row_top" => weights.row_top = value,
        "row_home" => weights.row_home = value,
        "row_bottom" => weights.row_bottom = value,
        "row_number" => weights.row_number = value,
        "pinky_start" => weights.pinky_start = value,
        "shift" => weights.shift = value,
        "left_shift_right_reach" => weights.left_shift_right_reach = value,
        "beat_interval" => weights.beat_interval = value,
        "finger_recovery" => weights.finger_recovery = value,
        "movement_recovery" => weights.movement_recovery = value,
        "movement_time_factor" => weights.movement_time_factor = value,
        "row_change_time" => weights.row_change_time = value,
        "stretch_time" => weights.stretch_time = value,
        "timing_wait" => weights.timing_wait = value,
        "same_key_motion" => weights.same_key_motion = value,
        "same_finger_motion" => weights.same_finger_motion = value,
        "recent_same_finger" => weights.recent_same_finger = value,
        "two_finger_bounce" => weights.two_finger_bounce = value,
        "abba_bounce" => weights.abba_bounce = value,
        "same_hand" => weights.same_hand = value,
        "hand_switch_onehand" => weights.hand_switch_onehand = value,
        "hand_switch_neutral" => weights.hand_switch_neutral = value,
        "hand_switch_alternation" => weights.hand_switch_alternation = value,
        "prefer_hand_penalty" => weights.prefer_hand_penalty = value,
        "adjacent_roll_reward" => weights.adjacent_roll_reward = value,
        "forward_roll_reward" => weights.forward_roll_reward = value,
        "reverse_roll_penalty" => weights.reverse_roll_penalty = value,
        "compact_cluster_reward" => weights.compact_cluster_reward = value,
        "smooth_trigram_reward" => weights.smooth_trigram_reward = value,
        "reverse_trigram_penalty" => weights.reverse_trigram_penalty = value,
        "full_sweep_reward" => weights.full_sweep_reward = value,
        "reverse_full_sweep_penalty" => weights.reverse_full_sweep_penalty = value,
        "upper_reverse_coupled_roll_reward" => weights.upper_reverse_coupled_roll_reward = value,
        "upper_reverse_axis_relief" => weights.upper_reverse_axis_relief = value,
        "upper_reverse_split_sweep_reward" => weights.upper_reverse_split_sweep_reward = value,
        "home_return_wait_relief" => weights.home_return_wait_relief = value,
        "pre_sweep_direction_change_penalty" => weights.pre_sweep_direction_change_penalty = value,
        "pre_sweep_direction_match_reward" => weights.pre_sweep_direction_match_reward = value,
        "home_sweep_reward" => weights.home_sweep_reward = value,
        "non_home_sweep_reward_factor" => weights.non_home_sweep_reward_factor = value,
        "mixed_row_sweep_penalty" => weights.mixed_row_sweep_penalty = value,
        "asymmetric_row_actuation" => weights.asymmetric_row_actuation = value,
        "motor_program_repeat_reward" => weights.motor_program_repeat_reward = value,
        "mixed_motor_program_penalty" => weights.mixed_motor_program_penalty = value,
        "row_run_reward" => weights.row_run_reward = value,
        "bottom_lock_reward" => weights.bottom_lock_reward = value,
        "row_jump_penalty" => weights.row_jump_penalty = value,
        "redirect" => weights.redirect = value,
        "lateral_stretch" => weights.lateral_stretch = value,
        "adjacent_roll_long_gap_penalty" => weights.adjacent_roll_long_gap_penalty = value,
        "index_stretch" => weights.index_stretch = value,
        "half_v_stretch" => weights.half_v_stretch = value,
        "digit_5_stretch" => weights.digit_5_stretch = value,
        "digit_6_stretch" => weights.digit_6_stretch = value,
        "cognitive_sweep_reward" => weights.cognitive_sweep_reward = value,
        "cognitive_known_walk_reward" => weights.cognitive_known_walk_reward = value,
        "cognitive_compact_reward" => weights.cognitive_compact_reward = value,
        "cognitive_vertical_reward" => weights.cognitive_vertical_reward = value,
        "cognitive_cap_per_step" => weights.cognitive_cap_per_step = value,
        "finger_axis_lateral" => weights.finger_axis_lateral = value,
        "finger_axis_opposite" => weights.finger_axis_opposite = value,
        "finger_axis_time" => weights.finger_axis_time = value,
        "left_pinky_axis_dx" => weights.left_pinky_axis_dx = value,
        "left_pinky_axis_dy" => weights.left_pinky_axis_dy = value,
        "left_ring_axis_dx" => weights.left_ring_axis_dx = value,
        "left_ring_axis_dy" => weights.left_ring_axis_dy = value,
        "left_middle_axis_dx" => weights.left_middle_axis_dx = value,
        "left_middle_axis_dy" => weights.left_middle_axis_dy = value,
        "left_index_axis_dx" => weights.left_index_axis_dx = value,
        "left_index_axis_dy" => weights.left_index_axis_dy = value,
        "right_index_axis_dx" => weights.right_index_axis_dx = value,
        "right_index_axis_dy" => weights.right_index_axis_dy = value,
        "right_middle_axis_dx" => weights.right_middle_axis_dx = value,
        "right_middle_axis_dy" => weights.right_middle_axis_dy = value,
        "right_ring_axis_dx" => weights.right_ring_axis_dx = value,
        "right_ring_axis_dy" => weights.right_ring_axis_dy = value,
        "right_pinky_axis_dx" => weights.right_pinky_axis_dx = value,
        "right_pinky_axis_dy" => weights.right_pinky_axis_dy = value,
        "left_index_lateral_factor" => weights.left_index_lateral_factor = value,
        "right_index_lateral_factor" => weights.right_index_lateral_factor = value,
        "left_middle_lateral_factor" => weights.left_middle_lateral_factor = value,
        "right_middle_lateral_factor" => weights.right_middle_lateral_factor = value,
        "left_ring_lateral_factor" => weights.left_ring_lateral_factor = value,
        "right_ring_lateral_factor" => weights.right_ring_lateral_factor = value,
        "left_pinky_lateral_factor" => weights.left_pinky_lateral_factor = value,
        "right_pinky_lateral_factor" => weights.right_pinky_lateral_factor = value,
        "left_index_repeat_factor" => weights.left_index_repeat_factor = value,
        "right_index_repeat_factor" => weights.right_index_repeat_factor = value,
        "left_middle_repeat_factor" => weights.left_middle_repeat_factor = value,
        "right_middle_repeat_factor" => weights.right_middle_repeat_factor = value,
        "left_ring_repeat_factor" => weights.left_ring_repeat_factor = value,
        "right_ring_repeat_factor" => weights.right_ring_repeat_factor = value,
        "left_pinky_repeat_factor" => weights.left_pinky_repeat_factor = value,
        "right_pinky_repeat_factor" => weights.right_pinky_repeat_factor = value,
        _ => return Err(format!("unknown weight/config key: {key}")),
    }
    Ok(())
}
