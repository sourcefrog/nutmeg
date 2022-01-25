// Copyright 2022 Martin Pool.

//! Manage a console/terminal UI that can alternate between showing a progress bar
//! and lines of text output.
//!
//! By contrast to other Rust progress-bar libraries, `steady_progress`
//! defers drawing the progress bar to the calling application, which can
//! draw whatever information it wants, however it wants.
//!
//! The application (or dependent library) is responsible for:
//! * Defining a type that implements [State], which holds whatever
//!   information is relevant to drawing progress.
//! * Defining how to render that information into some text lines,
//!   by implementing [State::render].
//! * Constructing a [View].
//! * Notifying the [View] when there are state updates, by calling
//!   [View::update].
//!
//! This library is responsible for:
//! * Periodically drawing the progress bar.
//! * Removing the progress bar when the view is finished or dropped.
//! * Coordinating to hide the bar to print text output, and restore
//!   it afterwards.
//! * Limiting the rate at which updates are drawn to the screen.
//!
//! Errors in writing to the terminal are discarded.

// TODO: Maybe, later, draw from a background thread, so that it will
// keep ticking even if not actively updated...

use std::borrow::Cow;
use std::io::Write;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

pub trait State {
    /// Render this state into a sequence of lines.
    ///
    /// Each line should be no more than `width` columns as displayed.
    ///
    /// The returned lines should not include `\n` characters.
    fn render(&self, width: usize) -> Vec<Cow<'_, str>>;
}

/// A view that draws and coordinates a progress bar on the terminal.
///
/// There should be only one `View` active on a terminal at any time, and
/// while it's in use it should be the only channel by which output is
/// printed.
///
/// The View may be shared freely across threads: it internally
/// synchronizes updates.
pub struct View<S: State, Out: Write> {
    inner: Mutex<Inner<S, Out>>,
}

struct Inner<S: State, Out: Write> {
    /// Current application state.
    state: S,

    /// Stream to write to the terminal.
    out: Out,

    /// True if the progress output is currently drawn to the screen.
    progress_drawn: bool,

    /// Number of lines the cursor is below the line where the progress bar
    /// should next be drawn.
    cursor_y: usize,

    /// Target interval to repaint the progress bar.
    update_interval: Duration,
    // TODO: Remember if we've printed an incomplete line, and in that
    // case don't draw progress until it's finished.
    // TODO: Make this implement Write and forward to `print`?
}

impl<S, Out> View<S, Out>
where
    S: State,
    Out: Write,
{
    /// Construct a new progress view.
    ///
    /// `out` is typically either [std::io::stdout] or [std::io::stderr].
    ///
    /// `state` is the application-defined initial state.
    pub fn new(out: Out, state: S) -> View<S, Out> {
        let mut inner = Inner {
            out,
            state,
            progress_drawn: false,
            cursor_y: 0,
            update_interval: Duration::from_millis(250),
        };
        inner.paint();
        View {
            inner: Mutex::new(inner),
        }
    }

    fn lock_inner<'s, 'a: 's>(&'s self) -> MutexGuard<'a, Inner<S, Out>> {
        self.inner.lock().unwrap()
    }

    /// Erase the progress bar from the screen and conclude.
    pub fn finish(self) {
        // TODO: Also do this from Drop.
        self.hide();
        todo!()
    }

    /// Stop updating, without necessarily removing any currently visible
    /// progress.
    pub fn abandon(self) {
        // Mark it as not drawn (even if it is) so that Drop will not try to
        // hide it.
        self.inner.lock().unwrap().progress_drawn = false;
        // Nothing to do; consuming it is enough?
        // TODO: Something to stop Drop trying to erase it?
    }

    /// Update the state, and queue a redraw of the screen for later.
    pub fn update(&self, update_fn: fn(&mut S) -> ()) {
        let mut inner = self.inner.lock().unwrap();
        update_fn(&mut inner.state);
        inner.paint();
        todo!()
    }

    /// Temporarily remove the progress bar, if necessary, and then print
    /// text to the console.
    ///
    /// `text` should contain a trailing newline.
    pub fn print(&mut self, text: &str) {
        self.hide();
        todo!("print");
    }

    /// Set the target interval at which to repaint the progress bar.
    pub fn set_update_interval(&mut self, update_interval: Duration) {
        self.inner.lock().unwrap().update_interval = update_interval;
        // TODO: Perhaps, wake up the thread.
    }

    /// Set how long to wait after printing before drawing the progress
    /// bar again.
    pub fn set_print_holdoff(&mut self, holdoff: Duration) {
        todo!()
    }

    /// Hide the progress bar if it's currently drawn.
    pub fn hide(&self) {
        todo!();
    }
}

impl<S: State, Out: Write> Inner<S, Out> {
    fn paint(&mut self) {
        todo!()
    }
}
