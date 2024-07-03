//! defines a collective [`Syndicate`] of workers.
//!
//! this is used for composing groups of workers to perform a load-test.

use {
    self::gen::Generator,
    crate::{
        error::Error,
        worker::{WorkerHandle, WorkerResult},
    },
    futures::{FutureExt, Stream},
    pin_project::pin_project,
    std::{collections::VecDeque, marker::PhantomData, pin::Pin, task::Poll},
    tap::TapFallible,
    tokio::{
        sync::mpsc::{self, Receiver, Sender},
        task::JoinHandle,
    },
    tracing::{debug, info},
    url::Host,
};

pub use self::builder::Builder;

/// provides a builder to start a syndicate.
mod builder;

/// provides a generator for spawning workers at regular intervals.
mod gen;

/// a syndicate is a group of [`Worker`][crate::worker::Worker]s.
///
/// create a new syndicate using [`Syndicate::builder()`].
#[pin_project]
#[must_use = "a syndicate of workers do nothing unless polled"]
pub struct Syndicate {
    /// a generator that is spawning work for the syndicate to manage.
    gen: JoinHandle<()>,
    /// a receiver used to listen for newly generated jobs.
    ///
    /// this will be dropped and set to `None` once the stream is closed.
    rx: Option<Receiver<WorkerHandle>>,
    /// the running workers currently in-flight.
    workers: VecDeque<WorkerHandle>,
}

// === impl Builder ===

impl Builder {
    /// spawns a new [`Syndicate`].
    ///
    /// this spawns the tasks on the current tokio runtime.
    ///
    /// # panics
    ///
    /// this function panics if called outside of a tokio runtime.
    pub fn start(self) -> Result<Syndicate, Error> {
        let Self {
            host,
            port,
            total,
            rps,
        } = self;

        let make_fn = move || crate::worker::Worker::spawn(host.clone(), port);
        let (rx, gen) = Generator::new()
            .with_total(total)
            .at_rate_per_second(rps)
            .start(make_fn);

        Ok(Syndicate {
            gen,
            rx: Some(rx),
            workers: Default::default(),
        })
    }
}

// === impl Syndicate ===

/// a syndicate may be treated as an asynchronous stream of worker output.
impl Stream for Syndicate {
    type Item = WorkerResult;
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        debug!("polling for next item from workers");
        let syndicate = self.project();

        // first, poll the channel to see if we have received any new jobs.
        if let Some(rx) = syndicate.rx {
            match rx.poll_recv(cx) {
                Poll::Pending => {}
                Poll::Ready(Some(worker)) => {
                    debug!("a new worker has joined");
                    syndicate.workers.push_back(worker);
                }
                Poll::Ready(None) => {
                    // the channel is closed. we can drop it now.
                    debug!("dropping work receiver");
                    let _ = syndicate.rx.take();
                }
            }
        }

        // next, poll our queue of workers.
        if let Some(worker) = syndicate.workers.front_mut() {
            match worker.poll_unpin(cx) {
                Poll::Ready(res) => {
                    // the worker is finished! be sure to remove it from the queue.
                    debug!("a worker has finished");
                    let _ = syndicate.workers.pop_front();
                    let res = res.map(Some).unwrap(/*TODO handle JoinError*/);
                    return Poll::Ready(res);
                }
                Poll::Pending => return Poll::Pending,
            }
        }

        if syndicate.rx.is_some() {
            // there is not any work to do, because we have not been given work to do yet.
            Poll::Pending
        } else {
            // we are all done, yield none to signal the end of the stream.
            info!("work stream has finished");
            Poll::Ready(None)
        }
    }
}
