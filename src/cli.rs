//! command-line options.
//!
//! defines a [`Cli`] structure for parsing command-line arguments.

pub use self::{
    parse::{parse, try_parse_from},
    server::Server,
};

use {
    crate::error::Error,
    clap::Parser,
    std::{ffi::OsString, str::FromStr},
    tap::Tap,
    tracing::{debug, trace},
    url::Host,
};

/// command-line options for `aquarius`.
///
/// these are acquired via [`parse()`].
#[derive(Debug, Eq, Parser, PartialEq)]
#[command(
    about = "aquarius: a lightweight http/2 load-tester",
    long_about = "aquarius: a lightweight http/2 load-tester\n\n  __ _ __ _ _  _ __ _ _ _(_)_  _ ___\n / _` / _` | || / _` | '_| | || (_-<\n \\__,_\\__, |\\_,_\\__,_|_| |_|\\_,_/__/\n         |_|                     "
)]
pub struct Cli {
    /// the number of requests to send to the server.
    #[clap(long = "total")]
    pub requests_total: Option<u32>,
    /// the rate at which to send requests to the server.
    #[clap(long = "rate")]
    pub requests_per_second: Option<u32>,
    /// if true, render ascii charts after finishing.
    #[clap(long)]
    pub show_charts: bool,
    /// if true, configure a [`tracing`] subscriber.
    ///
    /// these logs will be written to stdout.
    #[clap(long)]
    pub trace: bool,
    /// the address of the server to be load-tested.
    ///
    /// this should be provided in the form of a `hostname:port` pair.
    pub server: Server,
}

mod parse {
    use super::*;

    /// parses the command line arguments, returning a [`Cli`].
    ///
    /// # panics
    ///
    /// NB: this prints an error and exits the process if the given arguments are malformed.
    pub fn parse() -> Cli {
        Cli::parse()
    }

    /// parse arguments from the provided iterator.
    pub fn try_parse_from<I, T>(i: I) -> Result<Cli, clap::error::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        Cli::try_parse_from(i)
    }

    // === test parse() ===

    #[cfg(test)]
    const AQUARIUS: &str = "aquarius";

    #[cfg(test)]
    const ADDRESS: &str = "localhost:8080";

    #[cfg(test)]
    lazy_static::lazy_static! {
        static ref SERVER: Server = Server {
                host: Host::Domain("localhost").to_owned(),
                port: 8080,
            };
    }

    #[test]
    fn args_parser_handles_server_only() -> Result<(), Error> {
        const ARGS: &[&str] = &[AQUARIUS, ADDRESS];
        assert_eq!(
            try_parse_from(ARGS)?,
            Cli {
                requests_total: None,
                requests_per_second: None,
                show_charts: false,
                trace: false,
                server: SERVER.clone(),
            },
            "can parse simple command-line args"
        );
        Ok(())
    }

    #[test]
    fn args_parser_handles_rate() -> Result<(), Error> {
        const ARGS: &[&str] = &[AQUARIUS, "--rate", "42", ADDRESS];
        assert_eq!(
            try_parse_from(ARGS)?,
            Cli {
                requests_total: None,
                requests_per_second: Some(42),
                show_charts: false,
                trace: false,
                server: SERVER.clone(),
            },
            "can parse simple command-line args"
        );
        Ok(())
    }

    #[test]
    fn args_parser_handles_total() -> Result<(), Error> {
        const ARGS: &[&str] = &[AQUARIUS, "--total", "666", ADDRESS];
        assert_eq!(
            try_parse_from(ARGS)?,
            Cli {
                requests_total: Some(666),
                requests_per_second: None,
                show_charts: false,
                trace: false,
                server: SERVER.clone(),
            },
            "can parse command-line args with `--total`"
        );
        Ok(())
    }

    #[test]
    fn args_parser_accepts_rate_and_total() -> Result<(), Error> {
        const ARGS: &[&str] = &[AQUARIUS, "--total", "666", "--rate", "42", ADDRESS];
        assert_eq!(
            try_parse_from(ARGS)?,
            Cli {
                requests_total: Some(666),
                requests_per_second: Some(42),
                show_charts: false,
                trace: false,
                server: SERVER.clone(),
            },
            "can parse command-line args with `--total`"
        );
        Ok(())
    }
}

mod server {
    use super::*;

    /// the address of the server to be load-tested.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Server {
        /// the host name.
        pub host: Host,
        /// the port number.
        pub port: u16,
    }

    // === impl Server ===

    impl FromStr for Server {
        type Err = Error;
        fn from_str(server: &str) -> Result<Self, Self::Err> {
            let (host, port) = server
                .rsplit_once(':')
                .ok_or("server address must be given as `hostname:port`")?;

            let host = Host::parse(host)?.tap(|host| trace!(?host, "parsed server hostname"));
            let port = port
                .parse::<u16>()?
                .tap(|host| trace!(?host, "parsed server port"));

            Ok(Self { host, port }.tap(|server| debug!(?server, "parsed server address")))
        }
    }

    // === test Server ===

    #[test]
    fn ipv4_loopback_can_be_parsed() -> Result<(), Error> {
        let _guard = aquarius_test_subscriber::set_default();
        "127.0.0.1:8080".parse::<Server>().map(|_| ())
    }

    #[test]
    fn ipv6_loopback_can_be_parsed() -> Result<(), Error> {
        let _guard = aquarius_test_subscriber::set_default();
        "[::1]:8080".parse::<Server>().map(|_| ())
    }

    #[test]
    fn localhost_port_can_be_parsed() -> Result<(), Error> {
        let _guard = aquarius_test_subscriber::set_default();
        "localhost:8080".parse::<Server>().map(|_| ())
    }

    #[test]
    fn port_must_be_a_number() -> Result<(), Error> {
        let _guard = aquarius_test_subscriber::set_default();
        "localhost:abc".parse::<Server>().unwrap_err();
        Ok(())
    }

    #[test]
    fn port_must_be_a_u16() -> Result<(), Error> {
        let _guard = aquarius_test_subscriber::set_default();
        const TOO_BIG: u32 = u16::MAX as u32 + 1;
        format!("localhost:{TOO_BIG}")
            .parse::<Server>()
            .unwrap_err();
        Ok(())
    }
}
