//! Reduce detected timing evidence to a bounded-error piecewise musical map.

pub const DEFAULT_BEAT_EPSILON_MS: f64 = 30.0;

pub(super) fn default_beat_epsilon_ms() -> f64 {
    DEFAULT_BEAT_EPSILON_MS
}

// Points are strictly ordered source frames at equal notated pulse intervals.
// Keep measure endpoints exact, then split at the largest timing residual until
// every remaining pulse is within epsilon. Check newly created segments too:
// simply comparing all pulses to the original straight line is insufficient.
pub(super) fn fitted_anchor_indices(points: &[u64], epsilon_frames: f64) -> Vec<usize> {
    fit(points, epsilon_frames, |start, end, i| {
        let intervals = (end - start) as u128;
        let numerator = u128::from(points[end] - points[start]) * (i - start) as u128;
        points[start] + ((numerator + intervals / 2) / intervals) as u64
    })
}

// Unequal notated intervals support pickups and fractional final pulses.
pub(super) fn fitted_anchor_indices_at(
    points: &[u64],
    offsets: &[f64],
    epsilon_frames: f64,
) -> Vec<usize> {
    fit(points, epsilon_frames, |start, end, i| {
        let ratio = (offsets[i] - offsets[start]) / (offsets[end] - offsets[start]);
        points[start] + ((points[end] - points[start]) as f64 * ratio).round() as u64
    })
}

fn fit(
    points: &[u64],
    epsilon_frames: f64,
    interpolate: impl Fn(usize, usize, usize) -> u64,
) -> Vec<usize> {
    let mut segments = vec![(0, points.len() - 1)];
    let mut anchors = vec![];
    while let Some((start, end)) = segments.pop() {
        let mut worst = None;
        let mut error = epsilon_frames;
        for (i, point) in points.iter().enumerate().take(end).skip(start + 1) {
            let interpolated = interpolate(start, end, i);
            let residual = point.abs_diff(interpolated) as f64;
            if residual > error {
                error = residual;
                worst = Some(i);
            }
        }
        if let Some(index) = worst {
            anchors.push(index);
            segments.push((start, index));
            segments.push((index, end));
        }
    }
    anchors.sort_unstable();
    anchors
}
