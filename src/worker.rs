//! defines a single worker used for load-testing.

use {
    self::connect::ConnectionHandle,
    crate::error::Error,
    http::{response::Parts, Request, Response},
    http_body_util::Full,
    hyper::{
        body::{Body, Bytes, Incoming},
        client::conn::http2::{self, Connection, SendRequest},
    },
    hyper_util::rt::{TokioExecutor, TokioIo},
    std::time::{Duration, Instant},
    tap::{Pipe, Tap},
    tokio::{net::TcpStream, task::JoinHandle},
    tracing::{debug, info_span, instrument, trace, Instrument},
    url::Host,
};

/// workers can [`connect()`][Worker::connect] to a server.
mod connect;

/// workers can [`tx()`][Worker::tx] a request, awaiting a response.
mod tx;

/// a load-test worker.
///
/// a worker represents a single "job", responsible for connecting to an http/2 server, sending
/// a request, and receiving a response.
///
/// most callers should use [`Worker::run()`]. use [`Worker::run_local()`] to run the worker
/// against a server that is running on a local port.
pub struct Worker<B = Full<Bytes>> {
    /// the host to send requests to.
    pub host: Host,
    /// the port to send requests to.
    pub port: u16,
    /// the sender-side of the connection.
    tx: SendRequest<B>,
    /// the background task responsible for http state.
    conn: ConnectionHandle,
}

/// a report, containing information about the outcome of a [`Worker`].
pub struct Report {
    /// how long the worker took to run.
    pub duration: Duration,
    /// true if the response was a success.
    pub success: bool,
    /// the timestamp marking when the worker started running.
    pub start: Instant,
    /// the timestamp marking when the worker finished running.
    pub end: Instant,
}

/// the result of [`Worker::run()`].
pub type WorkerResult = Result<Report, Error>;

/// a handle to a [`Worker`] running in the background.
pub type WorkerHandle = JoinHandle<WorkerResult>;

// === impl Worker ===

impl Worker {
    /// spawns a worker.
    ///
    /// # panics
    ///
    /// this will panic if called outside of a tokio runtime.
    #[instrument]
    pub fn spawn(host: Host, port: u16) -> WorkerHandle {
        let fut = Self::run(host, port);
        tokio::spawn(fut)
    }

    /// runs a worker.
    #[instrument]
    pub async fn run(host: Host, port: u16) -> WorkerResult {
        use http_body_util::BodyExt;

        let start = std::time::Instant::now();
        let resp: Parts = {
            // === /!\ critical section /!\ ===
            // this is where the worker will connect, send a request, and read the response.
            // NB: even though it is unused, we should be sure to read the contents of the body.
            let mut worker = Self::connect(host, port).await?;
            let (resp, body) = worker.tx().await?.into_parts();
            let _body = body.collect().await?.to_bytes();
            resp
        };
        let end = std::time::Instant::now();

        // build a report about what the worker observed.
        let report = Report {
            duration: end.duration_since(start),
            success: resp.status.is_success(),
            start,
            end,
        };

        Ok(report)
    }

    /// runs a worker against a `localhost` port.
    pub async fn run_local(port: u16) -> WorkerResult {
        Self::run(crate::LOCALHOST, port).await
    }
}

impl<B> Drop for Worker<B> {
    /// the background task driving http state should be aborted when the worker is dropped.
    fn drop(&mut self) {
        self.conn.abort();
    }
}
