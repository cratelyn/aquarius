//! `aquarius` is a tool for load-testing http/2 servers.
//!
//! see [`run_load_test()`] and the project `README.md` file for more information.

#![deny(
    // rustc lints:
    deprecated,
    future_incompatible,
    keyword_idents,
    missing_docs,
    nonstandard_style,
    unused,
    // clippy lints:
    clippy::complexity,
    clippy::correctness,
    clippy::perf,
    clippy::suspicious,
    // rustdoc lints:
    rustdoc::bare_urls,
    rustdoc::broken_intra_doc_links,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_html_tags,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::private_doc_tests,
    rustdoc::private_intra_doc_links,
    rustdoc::redundant_explicit_links,
    rustdoc::unescaped_backticks,
)]

pub use self::{
    error::{Error, Result},
    summary::Summary,
    syndicate::Syndicate,
    worker::Worker,
};

pub mod charts;
pub mod cli;
pub mod summary;
pub mod syndicate;
pub mod worker;

/// runs a load-test against an http/2 server.
///
/// using the provided [`Cli`][cli::Cli] command-line options, this will spawn a group of workers
/// that each connect to the server, send it a request, and read the response. this returns a
/// [`Summary`][summary::Summary] containing information about the observed success rate, latency,
/// and average number of in-flight requests.
///
/// a [`Cli`][cli::Cli] may be obtained by [`cli::parse()`], parsing the arguments given to the
/// current process via [`std::env::args_os()`]. or, use [`cli::try_parse_from`] to parse
/// arguments from an [`Iterator`].
///
/// see [`charts`] for facilities related to printing graphs of the generated data.
pub async fn run_load_test(
    cli::Cli {
        server: cli::Server { host, port },
        requests_total,
        requests_per_second,
        show_charts: _,
        trace: _,
    }: cli::Cli,
) -> Result<Summary> {
    use {futures::TryStreamExt, tap::Tap, tracing::info};

    // prepare a stream of workers.
    let workers = Syndicate::builder(host, port)
        .total(requests_total)
        .rps(requests_per_second);

    // start the load test, and poll the tasks to completion.
    let summary: Summary = workers
        .tap(|_| info!("starting load-test"))
        .start()?
        .try_collect()
        .tap(|_| info!("collecting worker results"))
        .await
        .tap(|_| info!("load-test completed"))?;

    // log some information about the results of the load test.
    tracing::warn!(
        success.rate = %summary.success_rate(),
        duration.median_us = %summary.median_duration().as_micros(),
        in_flight.avg = %summary.average_in_flight(),
        "finished running load test"
    );

    Ok(summary)
}

/// the loopback address.
///
/// use this to run a worker against a server running on the same machine.
pub const LOCALHOST: url::Host = url::Host::Ipv6(std::net::Ipv6Addr::LOCALHOST);

/// error types.
pub mod error {
    /// a boxed error.
    pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

    /// a result type, with a boxed error.
    pub type Result<T> = std::result::Result<T, Error>;
}
