use std::{
    collections::VecDeque,
    time::{
        Duration,
        Instant,
    },
};

use cs2::StateVariable;

/// Types which can be linearly interpolated (and extrapolated).
pub trait Lerp: Clone {
    /// Linear interpolation between `a` (t = 0) and `b` (t = 1).
    /// Values of t outside [0, 1] extrapolate along the same direction.
    fn lerp(a: &Self, b: &Self, t: f32) -> Self;

    /// Change detection used to only record samples when the
    /// underlying game state actually changed.
    fn approx_eq(a: &Self, b: &Self) -> bool;
}

impl Lerp for nalgebra::Vector3<f32> {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        a + (b - a) * t
    }

    fn approx_eq(a: &Self, b: &Self) -> bool {
        (a - b).norm_squared() < 1e-6
    }
}

impl Lerp for Vec<nalgebra::Vector3<f32>> {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        if a.len() != b.len() {
            /* bone layout changed, snap instead of interpolating garbage */
            return b.clone();
        }

        a.iter()
            .zip(b.iter())
            .map(|(a, b)| a + (b - a) * t)
            .collect()
    }

    fn approx_eq(a: &Self, b: &Self) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b.iter())
                .all(|(a, b)| (a - b).norm_squared() < 1e-6)
    }
}

struct Sample<T> {
    time: Instant,
    value: T,
}

/// Ring buffer of timestamped value snapshots.
///
/// Game state is read at a fixed cadence (CS2 updates entity positions at 64Hz)
/// while the overlay renders at a much higher, variable frame rate.
/// This buffer allows sampling the game state at any point in time,
/// either interpolated (slightly in the past, perfectly smooth) or
/// extrapolated (up-to-date, velocity projected).
pub struct InterpBuffer<T: Lerp> {
    samples: VecDeque<Sample<T>>,
    capacity: usize,
}

impl<T: Lerp> InterpBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Record a new sample if the value actually changed.
    /// Returns true whenever a new sample has been recorded.
    pub fn push_changed(&mut self, time: Instant, value: T) -> bool {
        if let Some(latest) = self.samples.back() {
            if T::approx_eq(&latest.value, &value) {
                return false;
            }
        }

        if self.samples.len() >= self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(Sample { time, value });
        true
    }

    /// The value of the newest recorded sample.
    pub fn latest_value(&self) -> Option<&T> {
        self.samples.back().map(|sample| &sample.value)
    }

    /// Find the two samples bracketing `render_time` along with the
    /// interpolation factor t (start = t 0, end = t 1).
    ///
    /// - `render_time` between two recorded samples: linear interpolation
    /// - `render_time` after the newest sample: extrapolation along the
    ///   last observed velocity (t > 1), clamped to `max_extrapolate`
    /// - `render_time` before the oldest sample: clamped to the oldest value
    fn bracket(&self, render_time: Instant, max_extrapolate: Duration) -> Option<(&T, &T, f32)> {
        let newest = self.samples.back()?;
        let newest_index = self.samples.len() - 1;

        if render_time >= newest.time {
            /* current or future: extrapolate from the newest segment */
            if newest_index == 0 {
                return Some((&newest.value, &newest.value, 0.0));
            }

            let previous = &self.samples[newest_index - 1];
            let segment = newest.time.duration_since(previous.time).as_secs_f32();
            if segment < 1e-6 {
                return Some((&newest.value, &newest.value, 0.0));
            }

            let overshoot = render_time
                .duration_since(newest.time)
                .min(max_extrapolate)
                .as_secs_f32();
            return Some((&previous.value, &newest.value, 1.0 + overshoot / segment));
        }

        /* find the segment containing render_time (scanning newest to oldest) */
        for index in (1..self.samples.len()).rev() {
            let end = &self.samples[index];
            if render_time > end.time {
                continue;
            }

            let start = &self.samples[index - 1];
            let segment = end.time.duration_since(start.time).as_secs_f32();
            if segment < 1e-6 {
                return Some((&end.value, &end.value, 0.0));
            }

            let t = render_time.duration_since(start.time).as_secs_f32() / segment;
            return Some((&start.value, &end.value, t.clamp(0.0, 1.0)));
        }

        /* render_time is older than everything we have */
        let oldest = &self.samples.front()?;
        Some((&oldest.value, &oldest.value, 0.0))
    }

    /// Sample the buffer at the given point in time.
    pub fn sample(&self, render_time: Instant, max_extrapolate: Duration) -> Option<T> {
        self.bracket(render_time, max_extrapolate)
            .map(|(start, end, t)| T::lerp(start, end, t))
    }
}

fn positions_approx_eq(a: &[nalgebra::Vector3<f32>], b: &[nalgebra::Vector3<f32>]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(a, b)| (a - b).norm_squared() < 1e-6)
}

impl InterpBuffer<Vec<nalgebra::Vector3<f32>>> {
    /// Record bone positions, reusing the allocation of the oldest
    /// sample instead of allocating a new buffer per update.
    pub fn push_bones_changed(
        &mut self,
        time: Instant,
        positions: &[nalgebra::Vector3<f32>],
    ) -> bool {
        if let Some(latest) = self.samples.back() {
            if positions_approx_eq(&latest.value, positions) {
                return false;
            }
        }

        let mut value = if self.samples.len() >= self.capacity {
            self.samples
                .pop_front()
                .map(|sample| sample.value)
                .unwrap_or_default()
        } else {
            Vec::with_capacity(positions.len())
        };
        value.clear();
        value.extend_from_slice(positions);

        self.samples.push_back(Sample { time, value });
        true
    }

    /// Sample the buffer into the provided output buffer (reusing its allocation).
    /// Returns false when no samples have been recorded yet.
    pub fn sample_into(
        &self,
        render_time: Instant,
        max_extrapolate: Duration,
        out: &mut Vec<nalgebra::Vector3<f32>>,
    ) -> bool {
        let Some((start, end, t)) = self.bracket(render_time, max_extrapolate) else {
            return false;
        };

        out.clear();
        if start.len() != end.len() || t <= 0.0 {
            /* bone layout changed or exact sample: snap instead of interpolating garbage */
            out.extend_from_slice(end);
            return true;
        }

        out.extend(
            start
                .iter()
                .zip(end.iter())
                .map(|(start, end)| start + (end - start) * t),
        );
        true
    }
}

/// Live smoothing statistics displayed in the settings UI.
#[derive(Debug, Default, Clone, Copy)]
pub struct EspSmoothingStats {
    /// Measured cadence (in seconds) of actual game state changes
    pub sample_cadence: f32,

    /// The effective render delay (in seconds) currently in use
    pub effective_delay: f32,

    /// Amount of entities currently tracked by the smoothing system
    pub tracked_entities: usize,

    /// Age (in seconds) of the latest snapshot published
    /// by the background memory reader thread
    pub snapshot_age: f32,
}

pub type StateEspSmoothingStats = StateVariable<EspSmoothingStats>;
