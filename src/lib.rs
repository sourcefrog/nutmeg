// Copyright 2022 Martin Pool.

//! Manage a console/terminal UI that can alternate between showing a progress
//! bar and lines of text output.
//!
//! **NOTE:** Nothing is implemented yet: this is only a sketch of an API.
//!
//! ## Concept
//!
//! By contrast to other Rust progress-bar libraries, Nutmeg has no
//! built-in concept of what the progress bar or indicator should look like:
//! this is entirely under the control of the application. Nutmeg handles
//! drawing the application's progress bar to the screen and removing it as needed.
//!
//! The application (or dependent library) is responsible for:
//!
//! * Defining a type that implements [State], which holds whatever information
//!   is relevant to drawing progress.
//! * Defining how to render that information into some text lines, by
//!   implementing [State::render].
//! * Constructing a [View] that will draw progress to the terminal.
//! * Notifying the [View] when there are state updates, by calling
//!   [View::update].
//! * While a [View] is in use, all text written to stdout/stderr should be sent
//!   via that view, to avoid the display getting scrambled.
//!
//! The Nutmeg library is responsible for:
//!
//! * Periodically drawing the progress bar in response to updates, including
//!   * Horizontally truncating output to fit on the screen.
//!   * Handling changes in the number of lines of progress display.
//! * Removing the progress bar when the view is finished or dropped.
//! * Coordinating to hide the bar to print text output, and restore it
//!   afterwards.
//! * Limiting the rate at which updates are drawn to the screen.
//!
//! Errors in writing to the terminal are discarded.
//!
//! ## Potential future features
//!
//! * Draw updates from a background thread, so that it will keep ticking even
//!   if not actively updated, and to better handle applications that send a
//!   burst of updates followed by a long pause. The background thread will
//!   eventually paint the last drawn update.

#![warn(missing_docs)]

use std::io::Write;
use std::sync::Mutex;
use std::time::Duration;

use crossterm::{cursor, terminal, QueueableCommand};

/// An application-defined type that holds whatever state is relevant to the
/// progress bar, and that can render it into one or more lines of text.
pub trait State {
    /// Render this state into a sequence of one or more lines.
    ///
    /// Each line should be no more than `width` columns as displayed.
    /// If they are longer, they will be truncated.
    ///
    /// The rendered version may contain ANSI escape sequences for coloring, etc.
    ///
    /// Lines are separarated by `\n` and there may optionally be a final
    /// newline.
    fn render<W: Write>(&self, width: usize, write_to: &mut W);
}

/// A view that draws and coordinates a progress bar on the terminal.
///
/// There should be only one `View` active on a terminal at any time, and
/// while it's in use it should be the only channel by which output is
/// printed.
///
/// The View may be shared freely across threads: it internally
/// synchronizes updates.
///
/// The View implements [std::io::Write] and so can be used by e.g.
/// [std::writeln] to print non-progress output lines.
pub struct View<S: State, Out: Write> {
    inner: Mutex<InnerView<S, Out>>,
}

impl<S, Out> View<S, Out>
where
    S: State,
    Out: Write,
{
    /// Construct a new progress view.
    ///
    /// `out` is typically `std::io::stdout.lock()`.
    ///
    /// `state` is the application-defined initial state.
    pub fn new(out: Out, state: S, options: ViewOptions) -> View<S, Out> {
        let inner_view = InnerView {
            out,
            state,
            progress_drawn: false,
            cursor_y: 0,
            incomplete_line: false,
            options,
        };
        // Should we paint now, or wait for the first update? Maybe we'll just wait...
        // inner_view.paint();
        View {
            inner: Mutex::new(inner_view),
        }
    }

    /// Erase the progress bar from the screen and conclude.
    pub fn finish(self) {
        self.hide();
    }

    /// Stop updating, without necessarily removing any currently visible
    /// progress.
    pub fn abandon(self) {
        // Mark it as not drawn (even if it is) so that Drop will not try to
        // hide it.
        self.inner.lock().unwrap().progress_drawn = false;
        // Nothing to do; consuming it is enough?
    }

    /// Update the state, and queue a redraw of the screen for later.
    pub fn update(&self, update_fn: fn(&mut S) -> ()) {
        self.inner.lock().unwrap().update(update_fn)
    }

    /// Hide the progress bar if it's currently drawn.
    pub fn hide(&self) {
        self.inner.lock().unwrap().hide()
    }
}

impl<S: State, Out: Write> std::io::Write for View<S, Out> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let mut inner = self.inner.lock().unwrap();
        inner.hide();
        if !buf.ends_with(b"\n") {
            inner.incomplete_line = true;
        }
        inner.out.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<S: State, Out: Write> Drop for View<S, Out> {
    fn drop(&mut self) {
        // Only try lock here: don't hang if it's locked or panic
        // if it's poisoned
        if let Some(mut inner) = self.inner.try_lock().ok() {
            inner.hide()
        }
    }
}

/// The real contents of a View, inside a mutex.
struct InnerView<S: State, Out: Write> {
    /// Current application state.
    state: S,

    /// Stream to write to the terminal.
    out: Out,

    /// True if the progress output is currently drawn to the screen.
    progress_drawn: bool,

    /// Number of lines the cursor is below the line where the progress bar
    /// should next be drawn.
    cursor_y: usize,

    /// True if there's an incomplete line of output printed, and the
    /// progress bar can't be drawn until it's completed.
    incomplete_line: bool,

    options: ViewOptions,
}

impl<S: State, Out: Write> InnerView<S, Out> {
    fn paint_progress(&mut self) {
        if !self.options.progress_enabled {
            return;
        }
        // TODO: Move up over any existing progress bar.
        // TODO: Throttle, and keep track of the last update.
        self.hide();
        let mut rendered = Vec::new();
        let width = 80; // TODO: Get the right width.
        self.state.render(width, &mut rendered);
        self.out.write(&rendered).expect("write progress to output");
        self.progress_drawn = true;
        // TODO: Count lines.
        // TODO: Turn off line wrap; write one line at a time and erase to EOL; finally erase downwards.
    }

    /// Clear the progress bars off the screen, leaving it ready to
    /// print other output.
    fn hide(&mut self) {
        if self.progress_drawn {
            // todo!("move up the right number of lines then clear downwards, then update state");
            self.out
                .queue(terminal::Clear(terminal::ClearType::CurrentLine))
                .expect("clear line")
                .queue(cursor::MoveToColumn(0))
                .expect("move to start of line");
            self.progress_drawn = false;
        }
    }

    fn update(&mut self, update_fn: fn(&mut S) -> ()) {
        update_fn(&mut self.state);
        self.paint_progress()
    }
}

/// Options controlling a View.
///
/// These are supplied to [View::new] and cannot be changed after the view is created.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewOptions {
    /// Target interval to repaint the progress bar.
    pub update_interval: Duration,

    /// How long to wait after printing output before drawing the progress bar again.
    pub print_holdoff: Duration,

    /// Is the progress bar drawn at all?
    pub progress_enabled: bool,
}

impl Default for ViewOptions {
    fn default() -> ViewOptions {
        ViewOptions {
            update_interval: Duration::from_millis(250),
            print_holdoff: Duration::from_millis(250),
            progress_enabled: true,
        }
    }
}
