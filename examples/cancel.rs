use std::time::Duration;

use tokio_util::{future::FutureExt, sync::CancellationToken};

#[tracing::instrument(skip(ct))]
async fn app_main(ct: CancellationToken) {
    let mut i = 0;
    while !ct.is_cancelled() {
        // Do some work
        tracing::info!(i = i, "Still running");
        i += 1;

        tokio::time::sleep(Duration::from_secs(1))
            // cancel cancellables :)
            .with_cancellation_token(&ct)
            .await;
    }
}

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG")
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        unsafe {
            std::env::set_var("RUST_LOG", "info");
        }
    }

    tracing_subscriber::fmt().init();

    let cancellation_token = CancellationToken::new();

    libsignal::cancel_after_signal(cancellation_token.clone());

    tracing::info!("Starting");

    app_main(cancellation_token).await;

    tracing::info!("Exiting");
}
