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

    tracing::info!("It's running, ctrl+c to exit");

    libsignal::wait_for_signal().await;

    tracing::info!("Good job u killed it!");
}
