//! spawn a test server, and run a single worker against it.

use {aquarius::worker::Worker, aquarius_test_server::TestServer, tap::Tap};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn worker_can_send_one_request_to_a_server() -> Result<(), aquarius::error::Error> {
    let guard = aquarius_test_subscriber::set_default();
    aquarius_test_timeout::spawn();

    let server = TestServer::start().await?;
    Worker::run_local(server.port).await?;
    server.finish().await?;

    Ok(()).tap(|_| drop(guard))
}
