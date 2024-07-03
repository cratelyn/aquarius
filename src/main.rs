//! a small tool for load-testing http/2 servers.
//!
//! see [`main()`] for more information.

use aquarius::{
    cli::{self, Cli},
    run_load_test,
};

/// the entrypoint of `aquarius`.
///
/// this function runs a load-test against a specified endpoint. see [`Cli`] for more information
/// about accepted command-line arguments. see [`Syndicate`] for more information about how
/// worker threads are orchestrated. see [`Summary`] for book-keeping related to load-test results.
#[tokio::main(flavor = "multi_thread", worker_threads = 64)]
async fn main() -> aquarius::Result<()> {
    // parse the command-line arguments.
    let cli @ Cli {
        show_charts, trace, ..
    } = cli::parse();

    // write logs to stderr if `--trace` was provided.
    if trace {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .init();
    }

    // run the load test.
    let summary = run_load_test(cli).await?;

    // render some charts.
    if show_charts {
        aquarius::charts::render_progress(&summary);
        aquarius::charts::render_in_flight(&summary);
        aquarius::charts::render_table(&summary);
    }

    Ok(())
}
