use crate::model::*;
use crate::*;

use super::types::TypingState;

impl TypingState {
    pub fn new(layout: &Layout) -> Self {
        let mut positions = [Pos::default(); 8];
        for finger in Finger::ALL {
            positions[finger.idx()] = layout.home_pos(finger);
        }

        Self {
            text: String::new(),
            text_len: 0,
            score: 0.0,
            positions,
            postures: [PalmPosture::default(); 2],
            finger_ready: [0.0; 8],
            time: 0.0,
            last: None,
            prev: None,
            prev2: None,
            row_run_y: None,
            row_run_len: 0,
            steps: Vec::new(),
        }
    }

    pub fn average(&self) -> f64 {
        if self.text_len == 0 {
            0.0
        } else {
            self.score / self.text_len as f64
        }
    }

    pub fn push(&self, layout: &Layout, key: Key, settings: &Settings) -> Self {
        self.push_internal(layout, key, settings, true)
    }

    /// Fast generation path: keep the same ergonomic state, but do not retain the
    /// per-step explanation vector. Full `StepCost` history is valuable for
    /// `score --explain`, but cloning it inside beam search is pure overhead.
    pub fn push_compact(&self, layout: &Layout, key: Key, settings: &Settings) -> Self {
        self.push_internal(layout, key, settings, false)
    }

    fn push_internal(
        &self,
        layout: &Layout,
        key: Key,
        settings: &Settings,
        record_steps: bool,
    ) -> Self {
        let mut text = String::with_capacity(self.text.len() + key.typed.len_utf8());
        text.push_str(&self.text);
        text.push(key.typed);
        let text_len = self.text_len + 1;

        let step = self.step_cost(layout, key, settings, &text, text_len, record_steps);
        let step_total = step.total;
        let step_dynamic_cost = step.dynamic_cost;
        let step_movement_time = step.movement_time;
        let step_press_time = step.press_time;

        let steps = if record_steps {
            let mut out = Vec::with_capacity(self.steps.len() + 1);
            out.extend_from_slice(&self.steps);
            out.push(step);
            out
        } else {
            Vec::new()
        };
        // Generation states intentionally have no explanations, while scoring
        // states keep all step details.
        let mut next = Self {
            text,
            text_len,
            score: self.score + step_total,
            positions: self.positions,
            postures: self.postures,
            finger_ready: self.finger_ready,
            time: self.time,
            last: Some(key),
            prev: self.last,
            prev2: self.prev,
            row_run_y: self.row_run_y,
            row_run_len: self.row_run_len,
            steps,
        };

        for finger in Finger::ALL {
            let idx = finger.idx();
            let old = next.positions[idx];
            if finger == key.finger {
                next.positions[idx] = Pos { x: key.x, y: key.y };
            } else if finger.hand() == key.hand {
                let seq_gap = (finger.seq() - key.finger.seq()).abs() as f64;
                let alpha = (0.26 - 0.045 * seq_gap).max(0.08);
                let coupled = coupled_neighbor_position(key, finger);
                next.positions[idx] = Pos {
                    x: old.x + alpha * (coupled.x - old.x),
                    y: old.y + alpha * (coupled.y - old.y),
                };
            }
        }

        let old_posture = next.postures[key.hand.idx()];
        let rest = layout.home_pos(key.finger);
        let target_offset_x = clamp((key.x - rest.x) * 0.55, -1.15, 1.15);
        let target_offset_y = clamp((key.y - rest.y) * 0.62, -1.05, 1.05);
        let target_rotation = clamp(target_offset_x * 0.08 + target_offset_y * 0.04, -0.18, 0.18);
        let travel = self.positions[key.finger.idx()].dist_to(key);
        let target_tension = clamp(
            (travel - 0.65).max(0.0) * 0.35 + (target_offset_x.abs() - 0.45).max(0.0) * 0.20,
            0.0,
            2.5,
        );
        next.postures[key.hand.idx()] = PalmPosture {
            offset_x: old_posture.offset_x * 0.72 + target_offset_x * 0.28,
            offset_y: old_posture.offset_y * 0.68 + target_offset_y * 0.32,
            rotation: old_posture.rotation * 0.78 + target_rotation * 0.22,
            tension: old_posture.tension * 0.72 + target_tension * 0.28,
        };

        let other_hand = key.hand.other();
        let other = next.postures[other_hand.idx()];
        next.postures[other_hand.idx()] = PalmPosture {
            offset_x: other.offset_x * 0.78,
            offset_y: other.offset_y * 0.78,
            rotation: other.rotation * 0.82,
            tension: other.tension * 0.70,
        };

        next.time = step_press_time;
        next.finger_ready[key.finger.idx()] = next.time
            + finger_recovery_time(
                key.finger,
                step_dynamic_cost,
                step_movement_time,
                &settings.weights,
            );

        if let Some(last) = self.last {
            if same_row(last, key) {
                next.row_run_y = Some(key.y);
                next.row_run_len = if self.row_run_y == Some(key.y) {
                    self.row_run_len + 1
                } else {
                    2
                };
            } else {
                next.row_run_y = Some(key.y);
                next.row_run_len = 1;
            }
        } else {
            next.row_run_y = Some(key.y);
            next.row_run_len = 1;
        }

        next
    }
}
