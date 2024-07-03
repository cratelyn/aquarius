//! a [`tracing-subscriber`] for use in [`aquarius`] tests.

/// installs a test subscriber, returning a guard that will remove it when dropped.
pub fn set_default() -> tracing::subscriber::DefaultGuard {
    use {tracing::subscriber::set_default, tracing_subscriber::fmt};

    let subscriber = fmt()
        .with_env_filter("debug,aquarius=trace")
        .pretty()
        .with_test_writer()
        .finish();

    set_default(subscriber)
}
