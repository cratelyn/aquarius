use super::*;

// === impl Worker ===

impl Worker {
    /// sends a request to the server.
    #[instrument(skip(self), fields(host = %self.host, port = %self.port))]
    pub(super) async fn tx(&mut self) -> Result<Response<Incoming>, hyper::Error> {
        let Self { tx, host, .. } = self;

        let req = Self::req(host);
        tx.send_request(req)
            .tap(|_| trace!("sending request"))
            .await
            .tap(|_| debug!("received response"))
    }

    /// returns a request to send to the server.
    fn req(host: &Host) -> Request<Full<Bytes>> {
        let body = Full::new(Self::BODY);
        Request::builder()
            .header("host", host.to_string())
            .body(body)
            .unwrap()
    }

    /// a request body.
    const BODY: Bytes = Bytes::from_static(b"request body");
}
