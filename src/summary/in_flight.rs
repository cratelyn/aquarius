//! facilities for measuring the number of in-flight workers throughout a load-test.

use super::*;

/// a type alias for a collection of in-flight observations.
type Observations = std::collections::BTreeMap<Instant, u32>;

// === impl Summary ===

impl Summary {
    /// computes the average number of in-flight requests.
    pub fn average_in_flight(&self) -> f64 {
        let observations = self.compute_in_flight_observations();

        // now find the average across all of our observations.
        let sum: f64 = observations.values().sum::<u32>().into();
        let cnt: f64 = observations.len().try_into().map(u32::into).unwrap();

        sum / cnt
    }

    /// computes the number of in-flight requests at regular intervals in time.
    pub fn compute_in_flight_observations(&self) -> Observations {
        let Self { timestamps, .. } = self;

        // find how many workers were in flight at 5ms intervals during the load-test.
        let (min, max) = self.time_range();
        Snapshots::new(min, max)
            .map(|when| {
                let num = Self::count_in_flight(timestamps, when);
                (when, num)
            })
            .collect()
    }

    /// counts the number of timestampts in flight at a given [`Instant`].
    fn count_in_flight(timestamps: &[(Instant, Instant)], when: Instant) -> u32 {
        timestamps
            .iter()
            .filter(|ts| Self::was_in_flight(**ts, when))
            .count()
            .try_into()
            .expect("usize fits into u32")
    }

    /// returns true if a pair of timestamps encompass a given [`Instant`].
    fn was_in_flight((start, finish): (Instant, Instant), when: Instant) -> bool {
        start <= when && when <= finish
    }
}
