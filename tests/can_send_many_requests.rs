//! spawn a test server, and send a fixed amount of requests to it.

use {
    aquarius::{syndicate::Syndicate, worker::Report},
    aquarius_test_server::TestServer,
    futures::TryStreamExt,
    tap::Tap,
    tracing::info,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 64)]
async fn can_send_many_requests() -> Result<(), aquarius::error::Error> {
    let guard = aquarius_test_subscriber::set_default();
    // aquarius_test_timeout::spawn();

    const COUNT: u32 = 10;

    let server = TestServer::start().await?;
    info!("test server is running");

    let syndicate = Syndicate::local(server.port)
        .total(Some(COUNT))
        .rps(Some(8))
        .start()?;
    info!("workers are running");

    let reports = syndicate.try_collect::<Vec<Report>>().await?;
    info!("collected reports");

    assert_eq!(reports.len(), COUNT as usize);
    assert_eq!(server.reqs_received(), COUNT);

    server.finish().await?;
    Ok(()).tap(|_| drop(guard))
}
