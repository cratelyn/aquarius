use super::*;

/// an http/2 connection driven by tokio i/o and a tokio executor.
pub type TokioConnection<B> = Connection<TokioIo<TcpStream>, B, TokioExecutor>;

/// a handle to a [`TokioConnection`] task running in the background.
pub type ConnectionHandle = JoinHandle<Result<(), hyper::Error>>;

// === impl Worker ===

impl<B> Worker<B>
where
    B: Body + Send + 'static + Unpin,
    B::Data: Send,
    B::Error: Into<Error>,
{
    /// creates a new worker, connecting to the server.
    #[instrument]
    pub(super) async fn connect(host: Host, port: u16) -> Result<Self, Error> {
        // establish a connection to the server.
        let conn = format!("{host}:{port}")
            .pipe(TcpStream::connect)
            .tap(|_| trace!("establishing tcp connection"))
            .await
            .map(TokioIo::new) // use the tokio/hyper compatibility wrapper
            .tap(|_| debug!("established tcp connection"))?;

        // then perform the handshake with the server.
        let (tx, conn) = TokioExecutor::new()
            .pipe(http2::Builder::new)
            .handshake::<_, B>(conn)
            .tap(|_| trace!("beginning http/2 handshake"))
            .await
            .tap(|_| debug!("finished http/2 handshake"))?;
        let conn = Self::spawn_conn(conn);

        Ok(Self {
            tx,
            conn,
            host,
            port,
        })
    }

    /// spawns a worker task to process http state.
    ///
    /// see [`Connection`] for more information.
    fn spawn_conn(conn: TokioConnection<B>) -> ConnectionHandle {
        let span = {
            // create a span that follows from the current context.
            let curr = tracing::Span::current();
            let span = info_span!("connection worker");
            span.follows_from(curr);
            span
        };

        // instrument the connection, and spawn a background task
        conn.instrument(span).pipe(tokio::spawn)
    }
}
