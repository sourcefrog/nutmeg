// Copyright 2022 Martin Pool.

//! Manage a console/terminal UI that can alternate between showing a progress
//! bar and lines of text output.
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
//! * Defining a type that implements [Model], which holds whatever information
//!   is relevant to drawing progress.
//! * Defining how to render that information into some text lines, by
//!   implementing [Model::render]. This returns a `String` for the progress
//!   representation, optionally including ANSI styling.
//! * Constructing a [View] that will draw progress to the terminal.
//! * Notifying the [View] when there are model updates, by calling
//!   [View::update].
//! * While a [View] is in use, all text written to stdout/stderr should be sent
//!   via that view, to avoid the display getting scrambled. That is to say,
//!   use `writeln!(view, "hello")` rather than `println!("hello")`.
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
//! * Disabling progress if stdout is not a terminal.
//!
//! Errors in writing to the terminal cause a panic.
//!
//! ## Potential future features
//!
//! * Draw updates from a background thread, so that it will keep ticking even
//!   if not actively updated, and to better handle applications that send a
//!   burst of updates followed by a long pause. The background thread will
//!   eventually paint the last drawn update.
//!
//! * Write to an arbitrary `Write`, not just stdout?
//!
//! * Also set the window title from the progress model, perhaps by a different
//!   render function?

#![warn(missing_docs)]

use std::io::{self, Write};
use std::sync::Mutex;
use std::time::Duration;

use crossterm::terminal::ClearType;
use crossterm::{cursor, queue, style, terminal};
use crossterm::tty::IsTty;

/// An application-defined type that holds whatever state is relevant to the
/// progress bar, and that can render it into one or more lines of text.
pub trait Model {
    /// Render this model into a string to draw on the console.
    ///
    /// Each line should be no more than `width` columns as displayed.
    /// If they are longer, they will be truncated.
    ///
    /// The rendered version may contain ANSI escape sequences for coloring,
    /// etc, but should not move the cursor.
    ///
    /// Lines are separarated by `\n` and there may optionally be a final
    /// newline.
    ///
    /// ```
    /// struct Model { i: usize, total: usize }
    ///
    /// impl nutmeg::Model for Model {
    ///     fn render(&mut self, _width: usize) -> String {
    ///         format!("phase {}/{}", self.i, self.total)
    ///     }
    /// }
    /// ```
    fn render(&mut self, width: usize) -> String;
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
pub struct View<M: Model, Out: Write> {
    inner: Mutex<InnerView<M, Out>>,
}

impl<M, Out> View<M, Out>
where
    M: Model,
    Out: Write,
{
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

    /// Update the model, and queue a redraw of the screen for later.
    pub fn update(&self, update_fn: fn(&mut M) -> ()) {
        self.inner
            .lock()
            .unwrap()
            .update(update_fn)
            .expect("progress update failed")
    }

    /// Hide the progress bar if it's currently drawn.
    pub fn hide(&self) {
        self.inner
            .lock()
            .unwrap()
            .hide()
            .expect("failed to hide progress bar")
    }
}

impl<M: Model> View<M, io::Stdout> {
    /// Construct a new progress view, drawn to stdout.
    ///
    /// `model` is the application-defined initial model.
    pub fn new(model: M, mut options: ViewOptions) -> View<M, io::Stdout> {
        let out = io::stdout();
        if !out.is_tty() {
            options.progress_enabled = false;
        }
        let inner_view = InnerView {
            out,
            model,
            progress_drawn: false,
            // cursor_y: 0,
            incomplete_line: false,
            options,
        };
        // Should we paint now, or wait for the first update? Maybe we'll just wait...
        View {
            inner: Mutex::new(inner_view),
        }
    }
}

impl<M: Model, Out: Write> io::Write for View<M, Out> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let mut inner = self.inner.lock().unwrap();
        inner.hide()?;
        if !buf.ends_with(b"\n") {
            inner.incomplete_line = true;
        }
        inner.out.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<M: Model, Out: Write> Drop for View<M, Out> {
    fn drop(&mut self) {
        // Only try lock here: don't hang if it's locked or panic
        // if it's poisoned
        if let Ok(mut inner) = self.inner.try_lock() {
            let _ = inner.hide();
        }
    }
}

/// The real contents of a View, inside a mutex.
struct InnerView<M: Model, Out: Write> {
    /// Current application model.
    model: M,

    /// Stream to write to the terminal.
    out: Out,

    /// True if the progress output is currently drawn to the screen.
    // TODO: Maybe make this an actual state machine where drawing depends
    // on the state enum?
    progress_drawn: bool,

    // /// Number of lines the cursor is below the line where the progress bar
    // /// should next be drawn.
    // cursor_y: usize,
    /// True if there's an incomplete line of output printed, and the
    /// progress bar can't be drawn until it's completed.
    incomplete_line: bool,

    options: ViewOptions,
}

impl<M: Model, Out: Write> InnerView<M, Out> {
    fn paint_progress(&mut self) -> io::Result<()> {
        if !self.options.progress_enabled {
            return Ok(());
        }
        // TODO: Move up over any existing progress bar.
        // TODO: Throttle, and keep track of the last update.
        let width = terminal::size()?.0 as usize;

        let rendered = self.model.render(width);
        // Remove exactly one trailing newline, if there is one.
        let rendered = rendered.strip_suffix('\n').unwrap_or(&rendered);
        assert!(
            !rendered.contains('\n'),
            "multi-line progress is not implemented yet"
        );

        queue!(
            self.out,
            cursor::MoveToColumn(1),
            terminal::DisableLineWrap,
            style::Print(rendered),
            terminal::Clear(ClearType::UntilNewLine)
        )?;
        self.out.flush()?;

        self.progress_drawn = true;
        // TODO: Count lines; write one line at a time and erase to EOL; finally erase downwards.
        Ok(())
    }

    /// Clear the progress bars off the screen, leaving it ready to
    /// print other output.
    fn hide(&mut self) -> io::Result<()> {
        if self.progress_drawn {
            // todo!("move up the right number of lines then clear downwards, then update model");
            queue!(
                self.out,
                terminal::Clear(terminal::ClearType::CurrentLine),
                cursor::MoveToColumn(1),
                terminal::EnableLineWrap
            )?;
            self.progress_drawn = false;
        }
        Ok(())
    }

    fn update(&mut self, update_fn: fn(&mut M) -> ()) -> io::Result<()> {
        update_fn(&mut self.model);
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
