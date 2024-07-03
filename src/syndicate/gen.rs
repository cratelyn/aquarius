use {super::*, std::time::Duration, tracing::instrument};

/// a generator periodically yields values of type `T`.
pub(super) struct Generator<T> {
    /// the number of values that this generator will yield.
    total: Option<u32>,
    /// the amount of time to pause between yielding values.
    pause: Option<Duration>,
    /// the kinds of values that this generator yields.
    _yields: PhantomData<T>,
}

// === impl Generator ===

impl<T> Generator<T>
where
    T: Send + 'static,
{
    const CHANNEL_SIZE: usize = 256;

    /// returns a new generator.
    pub fn new() -> Self {
        Self {
            total: None,
            pause: None,
            _yields: PhantomData,
        }
    }

    /// sets the total number of values to be yielded.
    pub fn with_total(self, total: Option<u32>) -> Self {
        Self { total, ..self }
    }

    /// sets the duration to pause between values.
    pub fn with_pause(self, pause: Option<Duration>) -> Self {
        Self { pause, ..self }
    }

    /// generates values at `rate`-per-second.
    ///
    /// this is a convenience method abstracting over `with_pause()`.
    pub fn at_rate_per_second(self, rate: Option<u32>) -> Self {
        let duration_from_rate = |amt| Duration::from_secs(1) / amt;
        let pause = rate.map(duration_from_rate);

        self.with_pause(pause)
    }

    /// starts the generator, using the given [`Fn`] to yield values.
    ///
    /// returns a [`Receiver<T>`] to receive values, and a handle to the running task.
    ///
    /// # panics
    ///
    /// this will panic if called outside of a tokio runtime.
    #[instrument(skip_all)]
    pub fn start<F>(self, f: F) -> (Receiver<T>, JoinHandle<()>)
    where
        F: Fn() -> T,
        F: Send + 'static,
    {
        debug!("spawning generator worker");
        let (tx, rx) = mpsc::channel::<T>(Self::CHANNEL_SIZE);
        let worker = tokio::spawn(self.run(f, tx));

        (rx, worker)
    }

    /// the core event loop of a generator.
    #[instrument(skip_all)]
    async fn run<F>(self, f: F, tx: Sender<T>)
    where
        F: Fn() -> T,
    {
        let Self { total, pause, .. } = self;
        let mut remaining = total; // how many items are remaining?
        let mut yielded = 0; // how many items have we yielded?
        debug!("generator is running");

        loop {
            if let Some(rem) = remaining.as_mut() {
                if *rem > 0 {
                    // decrement how many items are remaining, if applicable.
                    debug!(%yielded, ?remaining, "generator is going to yield a value");
                    if let Some(rem) = remaining.as_mut() {
                        *rem -= 1;
                    }
                } else {
                    // stop yielding items when we reach the total.
                    debug!("generator has finished yielding items");
                    break;
                }
            }

            // yield a value and send it through the channel.
            //
            // TODO: backpressure / timeouts would be nice to have here.
            let t = f();
            match tx
                .try_send(t)
                .tap_ok(|_| debug!(%yielded, ?remaining, "generator yielded a value"))
            {
                Ok(()) => yielded += 1,
                Err(mpsc::error::TrySendError::Full(_)) => {
                    // TODO: gracefully handle this condition.
                    panic!("generator channel is full");
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    debug!("generator channel has been closed. stopping...");
                    break;
                }
            }

            // wait for the proscribed amount of time before continuing.
            if let Some(pause) = pause {
                debug!("generator is pausing");
                tokio::time::sleep(pause).await;
            }
        }

        debug!("generator has finished yielding values");
    }
}

#[cfg(test)]
mod generator_total_unit_tests {
    use std::ops::RangeInclusive;

    use super::*;

    /// for tests, we yield nothing.
    fn job() {}

    #[tokio::test]
    async fn generator_can_yield_a_value() {
        let (mut rx, _gen) = Generator::new().with_total(Some(1)).start(job);
        assert!(rx.recv().await.is_some());
        assert!(rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn generator_can_yield_four_values() {
        let (mut rx, _gen) = Generator::new().with_total(Some(4)).start(job);
        for _ in 0..4 {
            assert!(rx.recv().await.is_some());
        }
        assert!(rx.recv().await.is_none());
    }

    /// show that a generator yields work at roughly the specified rate.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn generator_can_yield_values_at_approximate_rps() {
        /// the "requests"-per-second rate at which the generator should yield items.
        const RPS: u32 = 32;
        /// the number of seconds we will let the generator run.
        const SECS: u64 = 2;
        const WAIT: Duration = Duration::from_secs(SECS);
        /// the number of items we expect to see, exactly.
        const EXPECTED: u64 = RPS as u64 * SECS;
        /// the margin of error we will accept.
        const MARGIN_OF_ERROR: u64 = EXPECTED / 10;
        /// the range of observed values we'll accept.
        const APPROX: RangeInclusive<u64> =
            (EXPECTED - MARGIN_OF_ERROR)..=(EXPECTED + MARGIN_OF_ERROR);

        // make sure our constants won't cause the generator's channel to fill up.
        debug_assert!(
            EXPECTED < Generator::<()>::CHANNEL_SIZE as u64,
            "this test has bad constants"
        );

        // start the generator, and wait for it to feed items into the channel.
        let (rx, gen) = Generator::new().at_rate_per_second(Some(RPS)).start(job);
        tokio::time::sleep(WAIT).await;

        // now stop the generator and observe how many items were yield.
        gen.abort();
        let cnt = rx.len() as u64;

        assert!(
            APPROX.contains(&cnt),
            "generator yielded an unexpected amount of work. \n\
            expected: ~{EXPECTED} \n\
            would accept: {APPROX:?} \n\
            found: {cnt}\n"
        );
    }
}
