//! a small test server binary.

const PORT: u16 = 8080;

#[tokio::main(flavor = "multi_thread", worker_threads = 64)]
async fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let _server = aquarius_test_server::TestServer::start_on_port(PORT)
        .await
        .unwrap();

    futures::future::pending::<()>().await;
}
