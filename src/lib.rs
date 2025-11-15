// Mostly stolen from https://stackoverflow.com/a/77591939

use std::future::pending;

use tokio_util::sync::CancellationToken;

/// Represents different types of shutdown signals that can be received.
///
/// This enum covers both Unix and Windows signals, allowing cross-platform
/// signal handling in a unified way.
///
/// # Examples
///
/// ```
/// use libsignal::Signal;
///
/// let signal = Signal::Terminate;
/// assert_eq!(signal.to_string(), "SIGTERM");
///
/// let custom = Signal::Other("CUSTOM".into());
/// assert_eq!(custom.to_string(), "CUSTOM");
/// ```
#[allow(unused)]
pub enum Signal {
    /// Unix SIGTERM signal - termination request
    Terminate,
    /// Unix SIGINT signal - interrupt (Ctrl+C)
    Interrupt,
    /// Windows CTRL_C event
    CtrlC,
    /// Windows CTRL_BREAK event
    Break,
    /// Windows CTRL_CLOSE event - console window closed
    Close,
    /// Windows CTRL_SHUTDOWN event - system shutdown
    Shutdown,
    /// Custom signal with user-defined name
    Other(String),
}

impl std::fmt::Display for Signal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Signal::Terminate => write!(f, "SIGTERM"),
            Signal::Interrupt => write!(f, "SIGINT"),
            Signal::CtrlC => write!(f, "CTRL_C"),
            Signal::Break => write!(f, "CTRL_BREAK"),
            Signal::Close => write!(f, "CTRL_CLOSE"),
            Signal::Shutdown => write!(f, "CTRL_SHUTDOWN"),
            Signal::Other(other) => write!(f, "{other}"),
        }
    }
}

/// Waits for a signal that requests a graceful shutdown, like SIGTERM or SIGINT.
#[cfg(unix)]
async fn wait_for_signal_impl<Or>(or: Or)
where
    Or: Future<Output = Option<Signal>>,
{
    use tokio::signal::unix::{SignalKind, signal};
    use tracing::Level;

    // Infos here:
    // https://www.gnu.org/software/libc/manual/html_node/Termination-Signals.html
    let mut signal_terminate = signal(SignalKind::terminate()).unwrap();
    let mut signal_interrupt = signal(SignalKind::interrupt()).unwrap();

    let signal = tokio::select! {
        _ = signal_terminate.recv() => Signal::Terminate,
        _ = signal_interrupt.recv() => Signal::Interrupt,
        Some(signal) = or => signal
    };

    let span = tracing::span!(Level::WARN, "signal");
    let guard = span.enter();
    tracing::warn!(signal =% signal, "Received cancellation signal");
    drop(guard);
}

/// Waits for a signal that requests a graceful shutdown, Ctrl-C (SIGINT).
#[cfg(windows)]
async fn wait_for_signal_impl<Or>(or: Or)
where
    Or: Future<Output = Option<Signal>>,
{
    use tokio::signal::windows;
    use tracing::Level;

    // https://learn.microsoft.com/en-us/windows/console/handlerroutine
    let mut signal_c = windows::ctrl_c().unwrap();
    let mut signal_break = windows::ctrl_break().unwrap();
    let mut signal_close = windows::ctrl_close().unwrap();
    let mut signal_shutdown = windows::ctrl_shutdown().unwrap();

    let signal = tokio::select! {
        _ = signal_c.recv() => Signal::CtrlC,
        _ = signal_break.recv() => Signal::Break,
        _ = signal_close.recv() => Signal::Close,
        _ = signal_shutdown.recv() => Signal::Shutdown,
        Some(signal) = or => signal
    };

    let span = tracing::span!(Level::WARN, "signal");
    let guard = span.enter();
    tracing::warn!(signal =% signal, "Received cancellation signal");
    drop(guard);
}

/// Waits indefinitely for an OS signal that requests a graceful shutdown.
///
/// On Unix platforms, this waits for SIGTERM or SIGINT.
/// On Windows, this waits for CTRL_C, CTRL_BREAK, CTRL_CLOSE, or CTRL_SHUTDOWN.
///
/// This function will block until a signal is received and logs the received signal
/// at the WARN level using tracing.
///
/// # Examples
///
/// ```no_run
/// use libsignal::wait_for_signal;
///
/// #[tokio::main]
/// async fn main() {
///     println!("Running... Press Ctrl+C to stop");
///
///     wait_for_signal().await;
///
///     println!("Received shutdown signal, cleaning up...");
/// }
/// ```
pub async fn wait_for_signal() {
    wait_for_signal_impl(pending()).await;
}

/// Spawns a task that cancels the given `CancellationToken` when an OS signal is received.
///
/// This is a convenience function that combines signal waiting with token cancellation.
/// The signal handler runs in a separate Tokio task.
///
/// On Unix platforms, this waits for SIGTERM or SIGINT.
/// On Windows, this waits for CTRL_C, CTRL_BREAK, CTRL_CLOSE, or CTRL_SHUTDOWN.
///
/// # Examples
///
/// ```
/// use libsignal::cancel_after_signal;
/// use tokio_util::sync::CancellationToken;
///
/// #[tokio::main]
/// async fn main() {
///     let token = CancellationToken::new();
///
///     // Spawn signal handler
///     cancel_after_signal(token.clone());
///
///     // Simulate some work, then manually trigger for testing
///     // In real usage, this would wait for actual OS signals
///     println!("Signal handler spawned");
///
///     // For testing purposes, we'll just demonstrate the setup
///     // In production, token.cancelled().await would wait for real signals
/// }
/// ```
pub fn cancel_after_signal(ct: CancellationToken) {
    tokio::spawn(async move {
        wait_for_signal().await;
        ct.cancel();
    });
}

/// Waits for either an OS signal or a custom future to complete.
///
/// This function allows you to combine OS signal handling with custom shutdown
/// triggers. Whichever completes first will cause the function to return.
///
/// If the custom future returns `None`, the function continues waiting for an OS signal.
/// If the custom future returns `Some(Signal)`, that signal is used and logged.
///
/// # Arguments
///
/// * `or` - A future that returns `Option<Signal>`. If it returns `Some`, that signal
///   is used. If it returns `None`, only OS signals are waited for.
///
/// # Examples
///
/// ```
/// use libsignal::{wait_for_signal_or, Signal};
///
/// #[tokio::main]
/// async fn main() {
///     // Wait for signal or immediate custom trigger
///     let immediate_trigger = async {
///         Some(Signal::Other("CUSTOM_TRIGGER".into()))
///     };
///
///     wait_for_signal_or(immediate_trigger).await;
///     println!("Custom trigger fired");
/// }
/// ```
///
/// ```
/// use libsignal::wait_for_signal_or;
///
/// # #[tokio::main]
/// # async fn main() {
///     // Returns None, so only waits for OS signals (would wait forever)
///     // In practice, use with a timeout or actual signal
///     let no_custom_signal = async { None as Option<()> };
///
///     // Note: This would wait indefinitely, so we don't actually call it
///     // wait_for_signal_or(no_custom_signal).await;
/// # }
/// ```
pub async fn wait_for_signal_or<Or>(or: Or)
where
    Or: Future<Output = Option<Signal>>,
{
    wait_for_signal_impl(or).await;
}

/// Spawns a task that cancels the given token when either an OS signal or custom future completes.
///
/// This combines [`wait_for_signal_or`] with automatic token cancellation in a spawned task.
/// This is useful for implementing graceful shutdown with multiple trigger conditions.
///
/// If the custom future returns `None`, only OS signals will trigger cancellation.
/// If the custom future returns `Some(Signal)`, cancellation happens immediately.
///
/// # Arguments
///
/// * `ct` - The `CancellationToken` to cancel when a signal is received
/// * `or` - A future that returns `Option<Signal>` to provide custom shutdown triggers
///
/// # Examples
///
/// ```
/// use libsignal::{cancel_after_signal_or, Signal};
/// use tokio_util::sync::CancellationToken;
///
/// #[tokio::main]
/// async fn main() {
///     let token = CancellationToken::new();
///
///     // Immediate custom shutdown trigger
///     let custom_shutdown = async {
///         Some(Signal::Other("IMMEDIATE".into()))
///     };
///
///     cancel_after_signal_or(token.clone(), custom_shutdown);
///
///     // Wait for cancellation
///     token.cancelled().await;
///     println!("Shutdown triggered");
/// }
/// ```
///
/// ```
/// use libsignal::{cancel_after_signal_or, Signal};
/// use tokio_util::sync::CancellationToken;
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicBool, Ordering};
///
/// #[tokio::main]
/// async fn main() {
///     let token = CancellationToken::new();
///     let shutdown_flag = Arc::new(AtomicBool::new(false));
///     let flag_clone = shutdown_flag.clone();
///
///     // Custom shutdown trigger - check a flag
///     let custom_shutdown = async move {
///         loop {
///             if flag_clone.load(Ordering::Relaxed) {
///                 return Some(Signal::Other("FLAG_SET".into()));
///             }
///             tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
///         }
///     };
///
///     cancel_after_signal_or(token.clone(), custom_shutdown);
///
///     // Set the flag immediately
///     shutdown_flag.store(true, Ordering::Relaxed);
///
///     token.cancelled().await;
///     println!("Shutdown complete");
/// }
/// ```
pub fn cancel_after_signal_or<Or>(ct: CancellationToken, or: Or)
where
    Or: Future<Output = Option<Signal>> + Send + 'static,
{
    tokio::spawn(async move {
        wait_for_signal_or(or).await;
        ct.cancel();
    });
}

#[cfg(test)]
mod tests {
    use crate::{Signal, cancel_after_signal_or, wait_for_signal_or};
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn test_cancels_on_or() {
        let ct = CancellationToken::new();
        let or = std::future::ready(Some(Signal::Other("TEST".into())));

        cancel_after_signal_or(ct.clone(), or);

        let cancelled = ct.cancelled();

        // Give it some time to poll
        let result = tokio::time::timeout(Duration::from_secs(5), cancelled).await;

        // Not elapsed
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_or_future_returns_none() {
        let ct = CancellationToken::new();
        let or = std::future::ready(None);

        cancel_after_signal_or(ct.clone(), or);

        // Should NOT cancel when or returns None
        let cancelled = ct.cancelled();
        let result = tokio::time::timeout(Duration::from_millis(100), cancelled).await;

        // Should timeout (not cancelled)
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wait_for_signal_or_with_immediate_signal() {
        let or = std::future::ready(Some(Signal::Terminate));

        let result = tokio::time::timeout(Duration::from_secs(1), wait_for_signal_or(or)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_signal_display_formatting() {
        assert_eq!(Signal::Terminate.to_string(), "SIGTERM");
        assert_eq!(Signal::Interrupt.to_string(), "SIGINT");
        assert_eq!(Signal::CtrlC.to_string(), "CTRL_C");
        assert_eq!(Signal::Break.to_string(), "CTRL_BREAK");
        assert_eq!(Signal::Close.to_string(), "CTRL_CLOSE");
        assert_eq!(Signal::Shutdown.to_string(), "CTRL_SHUTDOWN");
        assert_eq!(Signal::Other("CUSTOM".into()).to_string(), "CUSTOM");
    }

    #[tokio::test]
    async fn test_cancellation_token_already_cancelled() {
        let ct = CancellationToken::new();
        ct.cancel(); // Pre-cancel

        let or = async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Some(Signal::Other("LATE".into()))
        };

        cancel_after_signal_or(ct.clone(), or);

        // Should already be cancelled
        assert!(ct.is_cancelled());
    }

    #[tokio::test]
    async fn test_multiple_cancellation_tokens() {
        let ct1 = CancellationToken::new();
        let ct2 = CancellationToken::new();

        let or1 = std::future::ready(Some(Signal::Other("FIRST".into())));
        let or2 = async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Some(Signal::Other("SECOND".into()))
        };

        cancel_after_signal_or(ct1.clone(), or1);
        cancel_after_signal_or(ct2.clone(), or2);

        // First should cancel immediately
        tokio::time::timeout(Duration::from_millis(50), ct1.cancelled())
            .await
            .expect("ct1 should cancel quickly");

        // Second should cancel after delay
        tokio::time::timeout(Duration::from_millis(200), ct2.cancelled())
            .await
            .expect("ct2 should cancel after delay");
    }

    #[tokio::test]
    async fn test_or_future_with_delay() {
        let ct = CancellationToken::new();
        let or = async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Some(Signal::Other("DELAYED".into()))
        };

        cancel_after_signal_or(ct.clone(), or);

        // Should not be cancelled immediately
        let result = tokio::time::timeout(Duration::from_millis(10), ct.cancelled()).await;
        assert!(result.is_err(), "Should timeout before cancellation");

        // Should be cancelled after delay
        let result = tokio::time::timeout(Duration::from_millis(100), ct.cancelled()).await;
        assert!(result.is_ok(), "Should be cancelled after delay");
    }
}
