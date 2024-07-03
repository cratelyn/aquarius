use super::*;

/// an iterator of instants between a start and end time.
pub(super) struct Snapshots {
    curr: Instant,
    max: Instant,
}

// === impl Snapshots ===

impl Snapshots {
    /// the step size between in-flight observations.
    const STEP: Duration = Duration::from_millis(5);

    /// returns a new snapshot iterator.
    pub fn new(min: Instant, max: Instant) -> Self {
        Self { curr: min, max }
    }
}

impl Iterator for Snapshots {
    type Item = Instant;
    fn next(&mut self) -> Option<Self::Item> {
        let Self { curr, max } = self;

        if curr >= max {
            return None; // we have reached our end point.
        }

        let out = *curr; // step forward and yield a new instant.
        *curr += Self::STEP;
        Some(out)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let Self { curr, max } = self;

        let lower = 0;
        let upper = max.duration_since(*curr).as_millis() / Self::STEP.as_millis();

        (lower, upper.try_into().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_proper_number_of_timestamps() {
        let curr = Instant::now();
        let max = curr + Duration::from_millis(100);
        let snapshots = Snapshots { curr, max };
        assert_eq!(snapshots.size_hint(), (0, Some(20)));
        assert_eq!(snapshots.into_iter().count(), 20);
    }
}
