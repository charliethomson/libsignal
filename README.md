# libsignal

A simple, cross-platform Rust library for handling OS signals and graceful shutdown.
## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
libsignal = { git = "https://github.com/charliethomson/libsignal" }
```

## Usage

### Basic Signal Waiting

Wait for a shutdown signal:

```rust
use libsignal::wait_for_signal;

#[tokio::main]
async fn main() {
    println!("Running... Press Ctrl+C to stop");

    wait_for_signal().await;

    println!("Received shutdown signal, cleaning up...");
}
```

### Cancellation Token Integration

Automatically cancel a `CancellationToken` when a signal is received:

```rust
use libsignal::cancel_after_signal;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    let ct = CancellationToken::new();

    // Spawn signal handler
    cancel_after_signal(ct.clone());

    // Run your application
    tokio::select! {
        _ = ct.cancelled() => {
            println!("Shutting down gracefully...");
        }
        _ = app_main() => {
            println!("Application finished");
        }
    }
}

async fn app_main() {
    // Your application logic
}
```

### Custom Signal Sources

Combine OS signals with custom shutdown triggers:

```rust
use libsignal::{cancel_after_signal_or, Signal};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    let token = CancellationToken::new();

    // Custom shutdown trigger
    let custom_shutdown = async {
        // Wait for some custom condition
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        Some(Signal::Other("TIMEOUT".into()))
    };

    cancel_after_signal_or(token.clone(), custom_shutdown);

    // Your application loop
    loop {
        tokio::select! {
            _ = token.cancelled() => {
                println!("Shutdown triggered");
                break;
            }
            _ = do_work() => {}
        }
    }
}

async fn do_work() {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
}
```

## API

### Functions

- `wait_for_signal()` - Waits for an OS signal
- `wait_for_signal_or(or)` - Waits for an OS signal or custom future
- `cancel_after_signal(token)` - Cancels token when signal received
- `cancel_after_signal_or(token, or)` - Cancels token when signal or custom future completes

### Signal Enum

```rust
pub enum Signal {
    Terminate,       // SIGTERM (Unix)
    Interrupt,       // SIGINT (Unix)
    CtrlC,          // CTRL_C (Windows)
    Break,          // CTRL_BREAK (Windows)
    Close,          // CTRL_CLOSE (Windows)
    Shutdown,       // CTRL_SHUTDOWN (Windows)
    Other(String),  // Custom signals
}
```

## Examples

See the [examples](examples/) directory for more usage patterns.

## License
[MIT](LICENSE.md)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

Signal handling implementation inspired by [this Stack Overflow answer](https://stackoverflow.com/a/77591939).
