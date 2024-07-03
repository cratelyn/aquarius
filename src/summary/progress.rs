//! facilities for measuring the rate of progress throughout a load-test.

use super::*;

/// a type alias for a collection of progress observations.
type Observations = std::collections::BTreeMap<Instant, f32>;

// === impl Summary ===

impl Summary {
    /// computes the percentage of workers that have finished at regular intervals in time.
    pub fn compute_progress_observations(&self) -> Observations {
        let Self { timestamps, .. } = self;

        // find how many workers were finished at 5ms intervals during the load-test.
        let (min, max) = self.time_range();
        Snapshots::new(min, max)
            .map(|when| {
                let num = Self::percent_finished(timestamps, when);
                (when, num)
            })
            .collect()
    }

    /// computes the percentage of workers that have finished at a given [`Instant`].
    fn percent_finished(timestamps: &[(Instant, Instant)], when: Instant) -> f32 {
        let total = timestamps.len() as f32;
        let finished = timestamps
            .iter()
            .filter(|ts| Self::was_done(**ts, when))
            .count() as f32;

        finished / total * 100.0
    }

    /// returns true if the timestamps represent a window before the given [`Instant`].
    fn was_done((_start, finish): (Instant, Instant), when: Instant) -> bool {
        when >= finish
    }
}
