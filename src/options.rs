// Copyright 2022-2023 Martin Pool.

use std::time::Duration;

use crate::Destination;

/// Options controlling a View.
///
/// These are supplied to a constructor like [View::new], and cannot be changed after the view is created.
///
/// The default options created by [Options::default] should be reasonable
/// for most applications.
///
/// # Example
///
/// ```
/// let options = nutmeg::Options::default()
///     .progress_enabled(false); // Don't draw bars, only print.
/// ```
///
/// Options can be constructed as a static or constant value, using [Options::new].
///
/// ```
/// use std::time::Duration;
/// use nutmeg::Options;
///
/// static NUTMEG_OPTIONS: Options = Options::new()
///     .update_interval(Duration::from_millis(100))
///     .progress_enabled(true)
///     .destination(nutmeg::Destination::Stderr);
/// ```
#[derive(Debug, Clone)]
pub struct Options {
    /// Target interval to repaint the progress bar.
    pub(crate) update_interval: Duration,

    /// How long to wait after printing output before drawing the progress bar again.
    pub(crate) print_holdoff: Duration,

    /// Is the progress bar drawn at all?
    pub(crate) progress_enabled: bool,

    /// Use a fake clock for testing.
    pub(crate) fake_clock: bool,

    /// Write progress and messages to stdout, stderr, or a capture buffer for tests?
    pub(crate) destination: Destination,
}

impl Options {
    /// Return some reasonable default options.
    ///
    /// The update interval and print holdoff are 100ms, the progress bar is enabled,
    /// and output is sent to stdout.
    pub const fn new() -> Options {
        Options {
            update_interval: Duration::from_millis(100),
            print_holdoff: Duration::from_millis(100),
            progress_enabled: true,
            fake_clock: false,
            destination: Destination::Stdout,
        }
    }

    /// Set whether the progress bar will be drawn.
    ///
    /// By default it is drawn, except that this value will be ignored by [View::new] if stdout is not a terminal.
    pub const fn progress_enabled(self, progress_enabled: bool) -> Options {
        Options {
            progress_enabled,
            ..self
        }
    }

    /// Set the minimal interval to repaint the progress bar.
    ///
    /// `Duration::ZERO` can be used to cause the bar to repaint on every update.
    pub const fn update_interval(self, update_interval: Duration) -> Options {
        Options {
            update_interval,
            ..self
        }
    }

    /// Set the minimal interval between printing a message and painting
    /// the progress bar.
    ///
    /// This is used to avoid the bar flickering if the application is
    /// repeatedly printing messages at short intervals.
    ///
    /// `Duration::ZERO` can be used to disable this behavior.
    pub const fn print_holdoff(self, print_holdoff: Duration) -> Options {
        Options {
            print_holdoff,
            ..self
        }
    }

    /// Enable use of a fake clock, for testing.
    ///
    /// When true, all calculations of when to repaint use the fake
    /// clock rather than the real system clock.
    ///
    /// The fake clock begins at [Instant::now()] when the [View] is
    /// constructed.
    ///
    /// If this is enabled the fake clock can be updated with
    /// [View::set_fake_clock].
    pub const fn fake_clock(self, fake_clock: bool) -> Options {
        Options { fake_clock, ..self }
    }

    /// Set whether progress bars are drawn to stdout, stderr, or an internal capture buffer.
    ///
    /// [Destination::Stdout] is the default.
    ///
    /// [Destination::Stderr] may be useful for programs that expect stdout to be redirected
    /// to a file and that want to draw progress output that is not captured by the
    /// redirection.
    pub const fn destination(self, destination: Destination) -> Options {
        Options {
            destination,
            ..self
        }
    }
}

impl Default for Options {
    /// Create default reasonable view options.
    ///
    /// This is the same as [Options::new].
    fn default() -> Options {
        Options::new()
    }
}
