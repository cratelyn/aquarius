//! a small http/2 server for use in [`aquarius`] tests.
//!
//! see [`TestServer`].

use {
    futures::FutureExt,
    http::{Request, Response},
    hyper::{body::Incoming, server::conn::http2::Builder, service::Service},
    hyper_util::rt::{TokioExecutor, TokioIo},
    std::{
        convert::Infallible,
        future::Future,
        net::{IpAddr, Ipv6Addr, SocketAddr},
        pin::Pin,
        sync::{atomic::AtomicU32, Arc},
    },
    tap::{Pipe, Tap, TapFallible},
    tokio::{
        net::{TcpListener, TcpStream},
        sync::RwLock,
        task::{AbortHandle, JoinError, JoinHandle, JoinSet},
    },
    tracing::{debug, error, info, info_span, instrument, trace, Instrument, Span},
};

/// a boxed error.
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

/// A collection of tasks that have been spawned.
type Tasks = Arc<RwLock<JoinSet<Result<(), Error>>>>;

/// a small http/2 server for use in [`aquarius`] tests.
#[must_use = "a test server must be `.await`'ed"]
pub struct TestServer {
    /// the local port that this server has been bound to.
    pub port: u16,
    /// a handle to the listener task responsible for accepting connections.
    listener: JoinHandle<Result<(), Error>>,
    /// the tasks currently running.
    ///
    /// these are the tasks responsible for handling a connection.
    tasks: Tasks,
    /// the number of requests that have been received.
    reqs_received: Arc<AtomicU32>,
}

/// a simple [`Service`].
struct TestService {
    /// an optional callback.
    ///
    /// this will be invoked when this service is [called][Service::call].
    on_call: Option<Box<dyn Fn() + Send + 'static>>,
}

// === impl TestServer ===

impl Drop for TestServer {
    fn drop(&mut self) {
        self.listener.abort();
    }
}

impl TestServer {
    /// a local "ephemeral" address.
    ///
    /// by binding to port 0, we can ask the operating system to assign us a port.
    /// see [`TcpListener::bind()`] for more information.
    const EPHEMERAL: SocketAddr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 0);

    /// starts a new test server on the specified port.
    pub async fn start_on_port(port: u16) -> Result<Self, Error> {
        let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), port);
        Self::start_(addr).await
    }

    /// starts a new test server.
    ///
    /// this will bind the server to a port assigned by the operating system. check `port` to see
    /// what port the server is listening on.
    pub async fn start() -> Result<Self, Error> {
        Self::start_(Self::EPHEMERAL).await
    }

    async fn start_(addr: SocketAddr) -> Result<Self, Error> {
        let tasks = JoinSet::new().pipe(RwLock::new).pipe(Arc::new);
        let reqs_received = Arc::new(AtomicU32::new(0));

        // bind the server to a local "ephemeral" port.
        let listener = TcpListener::bind(addr).await?;
        let port = listener.local_addr()?.port();

        // spawn a task to continue listening for inbound connections.
        let listener = {
            let tasks = Arc::clone(&tasks);
            let reqs_received = Arc::clone(&reqs_received);
            let fut = Self::listen(tasks, listener, reqs_received);
            let span = info_span!("test server listener", %port);
            span.follows_from(Span::current());
            fut.instrument(span).pipe(tokio::spawn)
        };

        Ok(Self {
            port,
            listener,
            tasks,
            reqs_received,
        })
        .tap(|_| info!(%port, "test server is listening on local port"))
    }

    /// waits for all outstanding tasks to complete.
    ///
    /// returns the number of requests handled by this server in its lifetime.
    #[instrument(
        skip(self),
        fields(
            port = %self.port,
            tasks.cnt = tracing::field::Empty,
        )
    )]
    pub async fn finish(self) -> Result<u32, JoinError> {
        let Self {
            tasks,
            listener,
            reqs_received,
            port: _,
        } = &self;

        // first, abort the listener tasks to stop accepting any new connections.
        listener.abort();

        // aquire a write lock upon the tasks..
        let mut tasks = tasks
            .write()
            .tap(|_| debug!("waiting for write-lock on join set"))
            .await
            .tap(|t| {
                Span::current().record("tasks.cnt", t.len());
                debug!("acquired write-lock on join set");
            });

        // ..and then wait for all outstanding tasks to complete.
        #[allow(clippy::redundant_pattern_matching)]
        while let Some(_) = tasks.join_next().await.transpose()? {}
        drop(tasks);

        let reqs_received = reqs_received.load(std::sync::atomic::Ordering::Relaxed);
        Ok(reqs_received).tap(|_| info!("finished waiting for tasks to complete"))
    }

    /// returns the number of requests received by the test service.
    pub fn reqs_received(&self) -> u32 {
        self.reqs_received
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// listens for incoming connections, spawning tasks to process them.
    #[instrument(skip_all)]
    async fn listen(
        tasks: Tasks,
        listener: TcpListener,
        reqs_received: Arc<AtomicU32>,
    ) -> Result<(), Error> {
        let mut conns = 0_u64;

        loop {
            // accept a new client connection.
            debug!("waiting for new connections");
            let (conn, client_addr) = listener
                .accept()
                .await
                .tap_ok(|_| {
                    // emit an info-level event every 10th connection.
                    conns += 1;
                    if conns % 10 == 0 {
                        info!(%conns, "accepted a new connection")
                    } else {
                        debug!(%conns, "accepted a new connection")
                    }
                })
                .tap_err(|err| error!(?err, "error accepting connection"))?;

            // create the future for the connection handler.
            let fut = Self::handle_conn(conn, Arc::clone(&reqs_received))
                .instrument(info_span!("test server connection", ?client_addr));

            // spawn the connection handler into our pool of tasks.
            Self::spawn_task(&tasks, fut)
                .await
                .tap(|_| debug!("test server spawned a new session"));
        }
    }

    /// handles an http/2 connection.
    #[instrument]
    async fn handle_conn(conn: TcpStream, reqs_received: Arc<AtomicU32>) -> Result<(), Error> {
        let exec = TokioExecutor::new();
        let io = TokioIo::new(conn);
        let service = TestService::new().on_call(move || {
            // when the test service is called, increment our counter tracking how many requests
            // the test server has received.
            reqs_received.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });

        Builder::new(exec)
            .serve_connection(io, service)
            .await
            .map_err(Into::into)
    }

    /// spawns a task in the test server.
    async fn spawn_task<F>(tasks: &Tasks, fut: F) -> AbortHandle
    where
        F: Future<Output = Result<(), Error>>,
        F: Send + 'static,
    {
        tasks
            .write()
            .tap(|_| trace!("waiting for write-lock on join set"))
            .await
            .tap(|_| trace!("acquired write-lock on join set"))
            .spawn(fut)
            .tap(|_| debug!("spawned task onto join set"))
    }
}

// === impl TestService ===

impl TestService {
    /// returns a new test service.
    pub fn new() -> TestService {
        Self { on_call: None }
    }

    /// sets a callback to be invoked when this service is [called][Service::call].
    pub fn on_call<F>(self, f: F) -> Self
    where
        F: Fn() + Send + 'static,
    {
        Self {
            on_call: Some(Box::new(f)),
        }
    }
}

impl Service<Request<Incoming>> for TestService {
    type Response = Response<Incoming>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    /// processes the request, return a response asynchronously.
    ///
    /// the response body will echo the contents of the inbound request body.
    fn call(&self, req: Request<Incoming>) -> Self::Future {
        // if an `on_call(..)` callback was provided, invoke that now.
        if let Some(f) = self.on_call.as_ref() {
            f();
        }

        // return a response, echoing the request body back to the client.
        req.into_body().pipe(Self::response).boxed()
    }
}

impl TestService {
    /// returns a newly allocated response.
    ///
    //  NB: this doesn't need to be asynchronous, but it is helpful for the sake of implementing
    //  a [`Service`].
    async fn response<T>(body: T) -> Result<Response<T>, Infallible> {
        Ok(Response::builder()
            .status(200)
            .header("hello", "world")
            .body(body)
            .expect("response should be valid"))
    }
}
