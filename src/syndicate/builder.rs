use super::*;

/// a [`Syndicate`] builder.
#[allow(unused)]
pub struct Builder {
    pub(super) host: Host,
    pub(super) port: u16,
    pub(super) total: Option<u32>,
    pub(super) rps: Option<u32>,
}

// === impl Syndicate ===

impl Syndicate {
    /// returns a new [`Builder`].
    pub fn builder(host: Host, port: u16) -> Builder {
        Builder {
            host,
            port,
            total: None,
            rps: None,
        }
    }

    /// returns a new [`Builder`], oriented at `localhost`.
    pub fn local(port: u16) -> Builder {
        Self::builder(crate::LOCALHOST, port)
    }
}

// === impl Builder ===

impl Builder {
    /// sets the total number of requests to send.
    pub fn total(self, total: Option<u32>) -> Self {
        Self { total, ..self }
    }

    /// sets the rate of requests to send per-second.
    pub fn rps(self, rps: Option<u32>) -> Self {
        Self { rps, ..self }
    }
}
