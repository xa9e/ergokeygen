use crate::model::*;
use crate::*;

use super::types::{StepCost, TypingState};

impl TypingState {
    pub(crate) fn step_cost(
        &self,
        layout: &Layout,
        key: Key,
        settings: &Settings,
        candidate_text: &str,
        candidate_len: usize,
        record_flags: bool,
    ) -> StepCost {
        let w = &settings.weights;
        let mut flags = Vec::new();
        let rest = layout.home_pos(key.finger);

        let mut static_cost = w.base_key
            + w.rest_distance * rest.dist_to(key)
            + key.finger.strength_penalty()
            + row_penalty(key, w);

        let mut axis_cost = finger_axis_deviation(layout, key, w);
        let (axis_relief, axis_relief_flags) =
            upper_reverse_axis_relief(self, key, axis_cost, w, record_flags);
        axis_cost = (axis_cost - axis_relief).max(0.0);
        if axis_cost > 0.08 {
            static_cost += axis_cost;
            push_flag(&mut flags, record_flags, "finger-axis:lateral-deviation");
        }
        flags.extend(axis_relief_flags);

        if self.text.is_empty() && matches!(key.finger, Finger::LeftPinky | Finger::RightPinky) {
            static_cost += w.pinky_start;
            push_flag(&mut flags, record_flags, "pinky-start");
        }

        let stretch = index_stretch_penalty(key, w);
        if stretch > 0.0 {
            static_cost += stretch;
            push_flag(&mut flags, record_flags, "index-stretch");
        }

        if key.shifted {
            static_cost += w.shift;
            push_flag(&mut flags, record_flags, "shift");
            if key.hand == Hand::Left {
                static_cost += (key.x - 2.75).max(0.0) * w.left_shift_right_reach;
                push_flag(&mut flags, record_flags, "left-shift-right-reach");
            }
        }

        let posture = self.postures[key.hand.idx()];
        let adjusted_rest = posture.adjusted(rest, key.hand);
        let dynamic_cost = w.dynamic_distance * self.positions[key.finger.idx()].dist_to(key)
            + w.palm_distance * adjusted_rest.dist_to(key)
            + w.palm_tension * posture.tension;
        let mut rhythm_cost = settings.prefer_hand.penalty_for(key.hand, w);
        let mut transition_cost = 0.0;

        if let Some(last) = self.last {
            if last.physical == key.physical {
                transition_cost += w.same_key_motion * finger_repeat_factor(key.finger, w);
                push_flag(&mut flags, record_flags, "finger-inertia:repeat-tap");
            } else if last.finger == key.finger {
                transition_cost += (w.same_finger_motion + 0.22 * key_distance(last, key))
                    * finger_repeat_factor(key.finger, w);
                push_flag(
                    &mut flags,
                    record_flags,
                    "finger-inertia:same-finger-travel",
                );
            } else if last.hand == key.hand {
                transition_cost += w.same_hand;
                if is_adjacent_roll(last, key) {
                    transition_cost += w.adjacent_roll_reward;
                    push_flag(&mut flags, record_flags, "adjacent-roll");
                    if (key.x - last.x).abs() > 1.35 {
                        transition_cost +=
                            w.adjacent_roll_long_gap_penalty * ((key.x - last.x).abs() - 1.35);
                        push_flag(&mut flags, record_flags, "adjacent-roll-long-gap");
                    }
                    if is_forward_physical_roll(last, key) {
                        transition_cost += w.forward_roll_reward;
                        push_flag(&mut flags, record_flags, "forward-roll");
                    } else {
                        transition_cost += w.reverse_roll_penalty;
                        push_flag(&mut flags, record_flags, "reverse-roll");
                    }
                } else if (key.x - last.x).abs() > 1.7 {
                    transition_cost += w.lateral_stretch;
                    push_flag(&mut flags, record_flags, "lateral-stretch");
                }
            } else {
                match settings.mode {
                    RhythmMode::OneHand => {
                        rhythm_cost += w.hand_switch_onehand;
                        push_flag(&mut flags, record_flags, "hand-switch");
                    }
                    RhythmMode::Neutral => rhythm_cost += w.hand_switch_neutral,
                    RhythmMode::Alternation => {
                        rhythm_cost += w.hand_switch_alternation;
                        push_flag(&mut flags, record_flags, "hand-alternation");
                    }
                }
            }

            if self.row_run_len >= 2 && same_row(last, key) {
                transition_cost += w.row_run_reward;
                push_flag(&mut flags, record_flags, "row-run");
                if key.y > 1.5 {
                    transition_cost += w.bottom_lock_reward;
                    push_flag(&mut flags, record_flags, "bottom-lock");
                }
            } else if self.row_run_len >= 2 && (key.y - last.y).abs() >= 2.0 {
                transition_cost += w.row_jump_penalty * self.row_run_len as f64;
                push_flag(&mut flags, record_flags, "row-jump");
            }
        }

        if let (Some(a), Some(b)) = (self.prev, self.last) {
            if is_smooth_trigram(a, b, key) {
                let d1 = roll_delta(a, b);
                let d2 = roll_delta(b, key);
                if d1 > 0 && d2 > 0 && key.x > b.x && b.x > a.x {
                    transition_cost += w.smooth_trigram_reward;
                    push_flag(&mut flags, record_flags, "smooth-trigram");
                    if same_row(a, b) && same_row(b, key) && key.hand == Hand::Left {
                        if key.y == 1.0 {
                            transition_cost += w.home_sweep_reward;
                        } else {
                            transition_cost += w.home_sweep_reward * w.non_home_sweep_reward_factor;
                        }
                        push_flag(&mut flags, record_flags, "sweep");
                    }
                } else {
                    transition_cost += w.reverse_trigram_penalty;
                    push_flag(&mut flags, record_flags, "reverse-trigram");
                }
            }

            if upper_reverse_coupled_trigram(&[a, b, key]) {
                transition_cost += w.upper_reverse_coupled_roll_reward;
                push_flag(&mut flags, record_flags, "upper-reverse-coupled-roll");
            }

            if is_compact_cluster(&[a, b, key]) {
                transition_cost += w.compact_cluster_reward;
                push_flag(&mut flags, record_flags, "compact-cluster");
            }

            let d1 = roll_delta(a, b);
            let d2 = roll_delta(b, key);
            if a.hand == b.hand && b.hand == key.hand && d1 * d2 < 0 {
                transition_cost += w.redirect;
                push_flag(&mut flags, record_flags, "redirect");
            }
            if a.finger == key.finger && b.finger != a.finger {
                transition_cost += w.recent_same_finger * finger_repeat_factor(key.finger, w);
                push_flag(
                    &mut flags,
                    record_flags,
                    "finger-inertia:recent-same-finger",
                );
            }
        }

        if let (Some(a), Some(b), Some(c)) = (self.prev2, self.prev, self.last) {
            if a.physical == key.physical && b.physical == c.physical {
                transition_cost += w.abba_bounce * finger_repeat_factor(key.finger, w);
                push_flag(&mut flags, record_flags, "finger-inertia:abba");
            }

            let quad = [a, b, c, key];
            if upper_reverse_split_sweep(&quad) {
                transition_cost += w.upper_reverse_split_sweep_reward;
                push_flag(&mut flags, record_flags, "upper-reverse-split-sweep");
            } else {
                match sweep_direction(&quad) {
                    1 => {
                        transition_cost += w.full_sweep_reward;
                        push_flag(&mut flags, record_flags, "full-sweep");
                    }
                    -1 => {
                        transition_cost += w.reverse_full_sweep_penalty;
                        push_flag(&mut flags, record_flags, "reverse-full-sweep");
                    }
                    _ if four_finger_ordered_block(&quad) => {
                        let mismatch = mixed_motor_program_mismatch(&quad);
                        if mismatch > 0.0 {
                            transition_cost +=
                                w.mixed_row_sweep_penalty * mismatch + w.asymmetric_row_actuation;
                            push_flag(&mut flags, record_flags, "mixed-row-sweep");
                        }
                    }
                    _ => {}
                }
            }
        }

        if candidate_len >= 8 {
            let chars: Vec<char> = candidate_text.chars().collect();
            let n = chars.len();
            if chars[n - 4..n] == chars[n - 8..n - 4] {
                let mut block = Vec::with_capacity(4);
                for &ch in &chars[n - 4..n] {
                    if let Some(k) = layout.key(ch) {
                        block.push(k);
                    }
                }
                if block.len() == 4 {
                    let quad = [block[0], block[1], block[2], block[3]];
                    if (uniform_motor_program(&quad) && sweep_direction(&quad) > 0)
                        || upper_reverse_split_sweep(&quad)
                    {
                        transition_cost += w.motor_program_repeat_reward;
                        push_flag(
                            &mut flags,
                            record_flags,
                            "motor-program-repeat:uniform-or-coupled",
                        );
                    } else if four_finger_ordered_block(&quad) {
                        let mismatch = mixed_motor_program_mismatch(&quad);
                        transition_cost +=
                            w.mixed_motor_program_penalty + mismatch * w.asymmetric_row_actuation;
                        push_flag(&mut flags, record_flags, "motor-program-repeat:mixed");
                    }
                }
            }
        }

        if let (Some(a0), Some(a1), Some(b0)) = (self.prev2, self.prev, self.last) {
            let (pair_direction_cost, pair_direction_flags) =
                pair_direction_continuity_adjustment(&[a0, a1, b0, key], w, record_flags);
            transition_cost += pair_direction_cost;
            flags.extend(pair_direction_flags);
        }

        let (direction_cost, direction_flags) =
            pre_sweep_direction_adjustment(candidate_text, candidate_len, layout, w, record_flags);
        transition_cost += direction_cost;
        flags.extend(direction_flags);

        let (cognitive_cost, cognitive_flags) = cognitive_pattern_adjustment(
            self.prev2,
            self.prev,
            self.last,
            key,
            settings,
            record_flags,
        );
        flags.extend(cognitive_flags);

        let (timing_cost, timing_wait, press_time, movement_time) =
            timing_state_cost(self, layout, key, dynamic_cost, settings);
        if let Some(prev) = self.prev {
            if prev.finger == key.finger
                && (0.55..=1.45).contains(&key.y)
                && (prev.y - key.y).abs() >= 0.9
            {
                push_flag(
                    &mut flags,
                    record_flags,
                    "finger-inertia:home-return-relief",
                );
            }
        }
        if timing_wait > 0.0 {
            push_flag(&mut flags, record_flags, "finger-inertia:timing-wait");
        }

        let total = (static_cost
            + dynamic_cost
            + transition_cost
            + rhythm_cost
            + timing_cost
            + cognitive_cost)
            .max(0.05);
        StepCost {
            typed: key.typed,
            total,
            static_cost,
            dynamic_cost,
            transition_cost,
            rhythm_cost,
            timing_cost,
            cognitive_cost,
            timing_wait,
            press_time,
            movement_time,
            flags,
        }
    }
}
