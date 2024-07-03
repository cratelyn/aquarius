//! summaries are aggregated views of many reports.

use {
    self::snapshots::Snapshots,
    crate::worker::Report,
    std::{
        collections::BTreeSet,
        time::{Duration, Instant},
    },
};

mod in_flight;
mod progress;
mod snapshots;

/// an aggregated summary of many reports.
#[derive(Default)]
pub struct Summary {
    success_count: u32,
    total: u32,
    durations: BTreeSet<Duration>,
    timestamps: Vec<(Instant, Instant)>,
}

// === impl Summary ===

impl Summary {
    /// returns the success rate.
    pub fn success_rate(&self) -> f32 {
        let Self {
            success_count,
            total,
            ..
        } = self;

        // TODO: handle conversions here more robustly.
        let s: f32 = *success_count as f32;
        let t: f32 = *total as f32;

        s / t * 100.0
    }

    /// returns the median worker duration.
    pub fn median_duration(&self) -> Duration {
        let Self { durations, .. } = self;
        let mid = durations.len() / 2;

        durations
            .iter()
            .take(mid)
            .next()
            .expect("median value should exist")
            .to_owned()
    }

    /// returns the minimum and maximum timestamps.
    ///
    /// this reports when the first worker started, and when the last worker finished?
    pub fn time_range(&self) -> (Instant, Instant) {
        let iter = || self.timestamps.iter().flat_map(|(x, y)| [x, y].into_iter());
        let min = iter().min().expect("timestamps should exist");
        let max = iter().max().expect("timestamps should exist");
        (*min, *max)
    }
}

impl Extend<Report> for Summary {
    fn extend<I: IntoIterator<Item = Report>>(&mut self, iter: I) {
        for report in iter {
            self.record(report);
        }
    }
}

impl Summary {
    fn record(
        &mut self,
        Report {
            success,
            duration,
            start,
            end,
        }: Report,
    ) {
        let Self {
            success_count,
            total,
            durations,
            timestamps,
        } = self;

        *total += 1;
        durations.insert(duration);
        timestamps.push((start, end));

        if success {
            *success_count += 1;
        }
    }
}
