use crate::model::Seg;
use crate::segment;

/// Returns whether every path accepted by `y` is accepted by `x`.
pub fn inside(y: &[Seg], x: &[Seg]) -> bool {
    let (x_fixed, x_has_rest) = fixed_prefix(x);
    let (y_fixed, y_has_rest) = fixed_prefix(y);

    match (x_has_rest, y_has_rest) {
        (false, false) => {
            y_fixed.len() == x_fixed.len()
                && y_fixed
                    .iter()
                    .zip(x_fixed)
                    .all(|(y_segment, x_segment)| segment::subset(y_segment, x_segment))
        }
        (false, true) => false,
        (true, false) => {
            y_fixed.len() > x_fixed.len()
                && y_fixed
                    .iter()
                    .take(x_fixed.len())
                    .zip(x_fixed)
                    .all(|(y_segment, x_segment)| segment::subset(y_segment, x_segment))
        }
        (true, true) => {
            y_fixed.len() >= x_fixed.len()
                && y_fixed
                    .iter()
                    .take(x_fixed.len())
                    .zip(x_fixed)
                    .all(|(y_segment, x_segment)| segment::subset(y_segment, x_segment))
        }
    }
}

/// Returns the shortest canonical path in the intersection of two patterns.
pub fn witness(x: &[Seg], y: &[Seg]) -> Option<Vec<String>> {
    let (x_fixed, x_has_rest) = fixed_prefix(x);
    let (y_fixed, y_has_rest) = fixed_prefix(y);

    match (x_has_rest, y_has_rest) {
        (false, false) => {
            if x_fixed.len() != y_fixed.len() {
                return None;
            }

            meet_prefix(x_fixed, y_fixed, x_fixed.len())
        }
        (true, false) => witness_rest_fixed(x_fixed, y_fixed),
        (false, true) => witness_rest_fixed(y_fixed, x_fixed),
        (true, true) => witness_rest_rest(x_fixed, y_fixed),
    }
}

fn fixed_prefix(segments: &[Seg]) -> (&[Seg], bool) {
    match segments.split_last() {
        Some((Seg::Rest { .. }, fixed)) => (fixed, true),
        _ => (segments, false),
    }
}

fn meet_prefix(x: &[Seg], y: &[Seg], length: usize) -> Option<Vec<String>> {
    let mut result = Vec::with_capacity(length);

    for (x_segment, y_segment) in x.iter().take(length).zip(y.iter().take(length)) {
        result.push(segment::meet(x_segment, y_segment)?);
    }

    if result.len() == length {
        Some(result)
    } else {
        None
    }
}

fn witness_rest_fixed(rest_fixed: &[Seg], fixed: &[Seg]) -> Option<Vec<String>> {
    if fixed.len() <= rest_fixed.len() {
        return None;
    }

    let mut result = meet_prefix(rest_fixed, fixed, rest_fixed.len())?;
    result.extend(fixed.iter().skip(rest_fixed.len()).map(segment::sample));
    Some(result)
}

fn witness_rest_rest(x_fixed: &[Seg], y_fixed: &[Seg]) -> Option<Vec<String>> {
    let shared_length = x_fixed.len().min(y_fixed.len());
    let longest_length = x_fixed.len().max(y_fixed.len());
    let mut result = meet_prefix(x_fixed, y_fixed, shared_length)?;

    let longer_fixed = if x_fixed.len() > y_fixed.len() {
        x_fixed
    } else {
        y_fixed
    };
    result.extend(
        longer_fixed
            .iter()
            .skip(shared_length)
            .take(longest_length - shared_length)
            .map(segment::sample),
    );
    result.push("x".to_owned());
    Some(result)
}
