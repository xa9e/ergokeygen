use std::cmp::Ordering;

pub(crate) fn compare_f64(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

pub(crate) fn push_flag(flags: &mut Vec<&'static str>, enabled: bool, flag: &'static str) {
    if enabled {
        flags.push(flag);
    }
}

pub(crate) fn flags_if(enabled: bool, flag: &'static str) -> Vec<&'static str> {
    if enabled {
        vec![flag]
    } else {
        Vec::new()
    }
}

pub(crate) fn monotonic_i32(values: &[i32]) -> i32 {
    if values.windows(2).all(|w| w[0] < w[1]) {
        return 1;
    }
    if values.windows(2).all(|w| w[0] > w[1]) {
        return -1;
    }
    0
}

pub(crate) fn monotonic_f64(values: &[f64]) -> i32 {
    if values.windows(2).all(|w| w[0] < w[1]) {
        return 1;
    }
    if values.windows(2).all(|w| w[0] > w[1]) {
        return -1;
    }
    0
}

pub(crate) fn clamp(value: f64, low: f64, high: f64) -> f64 {
    value.max(low).min(high)
}
