//! a small timeout fuse for exiting deadlocked tests.

use {
    futures::FutureExt,
    std::time::Duration,
    tokio::{task::JoinHandle, time::sleep},
    tracing::{error, info},
};

/// spawns a future that will exit the process after 10 seconds.
///
/// this is useful for enforcing a timeout in tokio tests.
pub fn spawn() -> JoinHandle<()> {
    info!("spawning timeout fuse");
    let fut = self::timeout();
    tokio::spawn(fut)
}

/// exits the process after 10 seconds.
async fn timeout() {
    const DELAY: Duration = Duration::from_secs(10);

    sleep(DELAY)
        .then(|_| async {
            error!("time limit reached, exiting process");
            std::process::exit(1)
        })
        .await;
}
