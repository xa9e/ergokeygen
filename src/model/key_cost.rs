use crate::*;

pub(crate) fn key_distance(a: Key, b: Key) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

pub(crate) fn same_row(a: Key, b: Key) -> bool {
    (a.y - b.y).abs() < 0.1
}

pub(crate) fn roll_delta(a: Key, b: Key) -> i32 {
    b.finger.seq() - a.finger.seq()
}

pub(crate) fn is_adjacent_roll(a: Key, b: Key) -> bool {
    a.hand == b.hand && roll_delta(a, b).abs() == 1
}

pub(crate) fn is_forward_physical_roll(a: Key, b: Key) -> bool {
    roll_delta(a, b) > 0 && b.x > a.x
}

pub(crate) fn is_smooth_trigram(a: Key, b: Key, c: Key) -> bool {
    if !(a.hand == b.hand && b.hand == c.hand) {
        return false;
    }
    let d1 = roll_delta(a, b);
    let d2 = roll_delta(b, c);
    d1.abs() == 1 && d2.abs() == 1 && d1 == d2 && ((b.x - a.x) - (c.x - b.x)).abs() < 0.7
}

pub(crate) fn is_compact_cluster(keys: &[Key]) -> bool {
    if keys.len() < 3 {
        return false;
    }
    let hand = keys[0].hand;
    if keys.iter().any(|key| key.hand != hand) {
        return false;
    }
    for (i, left) in keys.iter().enumerate() {
        if keys
            .iter()
            .skip(i + 1)
            .any(|right| right.finger == left.finger)
        {
            return false;
        }
    }
    let mut min_x = keys[0].x;
    let mut max_x = keys[0].x;
    let mut min_y = keys[0].y;
    let mut max_y = keys[0].y;
    for key in keys {
        min_x = min_x.min(key.x);
        max_x = max_x.max(key.x);
        min_y = min_y.min(key.y);
        max_y = max_y.max(key.y);
    }
    (max_x - min_x) <= 2.75 && (max_y - min_y) <= 1.15
}

pub(crate) fn finger_repeat_factor(finger: Finger, weights: &Weights) -> f64 {
    match finger {
        Finger::LeftIndex => weights.left_index_repeat_factor,
        Finger::RightIndex => weights.right_index_repeat_factor,
        Finger::LeftMiddle => weights.left_middle_repeat_factor,
        Finger::RightMiddle => weights.right_middle_repeat_factor,
        Finger::LeftRing => weights.left_ring_repeat_factor,
        Finger::RightRing => weights.right_ring_repeat_factor,
        Finger::LeftPinky => weights.left_pinky_repeat_factor,
        Finger::RightPinky => weights.right_pinky_repeat_factor,
    }
}

pub(crate) fn finger_recovery_time(
    finger: Finger,
    dynamic_cost: f64,
    movement_time: f64,
    weights: &Weights,
) -> f64 {
    weights.finger_recovery * finger_repeat_factor(finger, weights)
        + (dynamic_cost - 0.80).max(0.0) * weights.movement_recovery
        + (movement_time - weights.beat_interval).max(0.0) * 0.35
}

pub(crate) fn key_stretch_time(key: Key, weights: &Weights) -> f64 {
    if key.finger != Finger::LeftIndex {
        return 0.0;
    }
    match key.physical {
        'v' => weights.stretch_time * 0.50,
        't' | 'g' => weights.stretch_time,
        'b' => weights.stretch_time * 1.35,
        '5' => weights.stretch_time * 0.75,
        '6' => weights.stretch_time * 1.65,
        _ => 0.0,
    }
}

pub(crate) fn unit_vec(dx: f64, dy: f64) -> (f64, f64) {
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 1e-9 {
        (0.0, -1.0)
    } else {
        (dx / len, dy / len)
    }
}

pub(crate) fn natural_finger_axis(finger: Finger, weights: &Weights) -> (f64, f64) {
    let (dx, dy) = match finger {
        Finger::LeftPinky => (weights.left_pinky_axis_dx, weights.left_pinky_axis_dy),
        Finger::LeftRing => (weights.left_ring_axis_dx, weights.left_ring_axis_dy),
        Finger::LeftMiddle => (weights.left_middle_axis_dx, weights.left_middle_axis_dy),
        Finger::LeftIndex => (weights.left_index_axis_dx, weights.left_index_axis_dy),
        Finger::RightIndex => (weights.right_index_axis_dx, weights.right_index_axis_dy),
        Finger::RightMiddle => (weights.right_middle_axis_dx, weights.right_middle_axis_dy),
        Finger::RightRing => (weights.right_ring_axis_dx, weights.right_ring_axis_dy),
        Finger::RightPinky => (weights.right_pinky_axis_dx, weights.right_pinky_axis_dy),
    };
    unit_vec(dx, dy)
}

pub(crate) fn axis_lateral_factor(finger: Finger, weights: &Weights) -> f64 {
    match finger {
        Finger::LeftIndex => weights.left_index_lateral_factor,
        Finger::RightIndex => weights.right_index_lateral_factor,
        Finger::LeftMiddle => weights.left_middle_lateral_factor,
        Finger::RightMiddle => weights.right_middle_lateral_factor,
        Finger::LeftRing => weights.left_ring_lateral_factor,
        Finger::RightRing => weights.right_ring_lateral_factor,
        Finger::LeftPinky => weights.left_pinky_lateral_factor,
        Finger::RightPinky => weights.right_pinky_lateral_factor,
    }
}

pub(crate) fn finger_axis_deviation(layout: &Layout, key: Key, weights: &Weights) -> f64 {
    if (0.55..=1.45).contains(&key.y) {
        return 0.0;
    }
    let rest = layout.home_pos(key.finger);
    let dx = key.x - rest.x;
    let dy = key.y - rest.y;
    let distance = (dx * dx + dy * dy).sqrt();
    if distance < 0.28 {
        return 0.0;
    }
    let (ax, ay) = natural_finger_axis(key.finger, weights);
    let along = dx * ax + dy * ay;
    let lateral = (dx * (-ay) + dy * ax).abs();
    let mut cost = lateral * weights.finger_axis_lateral * axis_lateral_factor(key.finger, weights);
    if along < 0.0 {
        cost += along.abs() * weights.finger_axis_opposite;
    }
    cost
}

pub(crate) fn coupled_neighbor_position(active: Key, finger: Finger) -> Pos {
    let signed_gap = (finger.seq() - active.finger.seq()) as f64;
    Pos {
        x: active.x + signed_gap * 0.95,
        y: active.y + signed_gap.abs() * 0.15,
    }
}

pub(crate) fn row_penalty(key: Key, weights: &Weights) -> f64 {
    if key.y < -0.5 {
        weights.row_number
    } else if key.y < 0.5 {
        weights.row_top
    } else if key.y < 1.5 {
        weights.row_home
    } else {
        weights.row_bottom
    }
}

pub(crate) fn index_stretch_penalty(key: Key, weights: &Weights) -> f64 {
    if key.finger != Finger::LeftIndex {
        return 0.0;
    }
    match key.physical {
        'v' => weights.half_v_stretch,
        't' | 'g' => weights.index_stretch,
        'b' => weights.index_stretch * 1.25,
        '5' => weights.digit_5_stretch,
        '6' => weights.digit_6_stretch,
        _ => 0.0,
    }
}
