// Copyright 2022 Martin Pool.

//! Nutmeg draws terminal progress bars whose appearance is completely controlled
//! by the application.
//!
//! ## Concept
//!
//! By contrast to other Rust progress-bar libraries, Nutmeg has no built-in
//! concept of what the progress bar or indicator should look like: this is
//! entirely under the control of the application.
//!
//! Nutmeg only supports ANSI terminals, which are supported on all Unix
//! and Windows 10 and later.
//!
//! The application is responsible for:
//!
//! 1. Defining a type holds whatever information is relevant to drawing
//!    progress bars.
//! 2. Rendering that information into styled text lines, by implementing the
//!    single-method trait [Model::render].
//!    * The application can control colors and styling by including ANSI
//!      escape sequences in the rendered string, for example by using the
//!      `yansi` crate.
//!    * The application is responsible for deciding whether or not to   
//!      color its output, for example by consulting `$CLICOLORS`.
//! 3. Constructing a [View] to draw a progress bar.
//! 4. Updating the model when appropriate by calling [View::update].
//! 5. Printing text output via the [View] while it is in use, to avoid the
//!    display getting scrambled.
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
//! ## Example
//!
//! ```
//! use std::io::Write;
//!
//! // 1. Define a struct holding all the application state necessary to
//! // render the progress bar.
//! #[derive(Default)]
//! struct Model {
//!     i: usize,
//!     total: usize,
//!     last_file_name: String,
//! }
//!
//! // 2. Define how to render the progress bar as a String.
//! impl nutmeg::Model for Model {
//!     fn render(&mut self, _width: usize) -> String {
//!         format!("{}/{}: {}", self.i, self.total, self.last_file_name)
//!     }
//! }
//!
//! fn main() -> std::io::Result<()> {
//!     // 3. Create a View when you want to draw a progress bar.
//!     let mut view = nutmeg::View::new(Model::default(),
//!         nutmeg::ViewOptions::default());
//!
//!     // 4. As the application runs, update the model via the view.
//!     for i in 0..100 {
//!         view.update(|model| {
//!             model.i += 1;
//!             model.last_file_name = format!("file{}.txt", i);
//!         });
//!         // 5. Interleave text output lines by writing to the view.
//!         if i % 10 == 3 {
//!             writeln!(view, "reached {}", i)?;
//!         }
//!     }
//!
//!     // 5. The bar is automatically erased when dropped.
//!     Ok(())
//! }
//! ```
//!
//! See the `examples/` directory for more.
//!
//! ## Potential future features
//!
//! * Draw updates from a background thread, so that it will keep ticking even
//!   if not actively updated, and to better handle applications that send a
//!   burst of updates followed by a long pause. The background thread will
//!   eventually paint the last drawn update.
//!
//! * Also set the window title from the progress model, perhaps by a different
//!   render function?
//!
//! * Better detection of when to draw progress or not. Possibly look at
//!   `TERM=dumb`; possibly hook in to a standard Rust mechanism e.g.
//!   <https://github.com/rust-cli/team/issues/15#issuecomment-891350115>.

#![warn(missing_docs)]

use std::fmt::Display;
use std::io::{self, Write};
use std::sync::Mutex;
use std::time::Duration;

mod ansi;
mod width;
#[cfg(windows)]
mod windows;

use crate::width::WidthStrategy;

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

/// Blanket implementation of Model for Display.
///
/// `self` is converted to a display string without regard for
/// the terminal width.
///
/// This allows direct use of e.g. a String or integer as a model
/// for very basic progress indications.
///
/// ```
/// let view = nutmeg::View::new(0, nutmeg::ViewOptions::default());
/// view.update(|model| *model += 1);
/// ```
impl<T> Model for T
where
    T: Display,
{
    fn render(&mut self, _width: usize) -> String {
        self.to_string()
    }
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
/// # Printing text lines
///
/// The View implements [std::io::Write] and so can be used by e.g.
/// [std::writeln] to print non-progress output lines.
///
/// The progress bar is removed from the screen to make room
/// for the printed output.
///
/// Printed output is emitted even if the progress bar is not enabled.
///
/// It is OK to print incomplete lines, i.e. without a final `\n`
/// character. In this case the progress bar remains suspended
/// until the line is completed.
pub struct View<M: Model, Out: Write> {
    inner: Mutex<InnerView<M, Out>>,
}

impl<M, Out> View<M, Out>
where
    M: Model,
    Out: Write,
{
    /// Stop using this progress view.
    ///
    /// If the progress bar is currently visible, it will be left behind on the
    /// screen.
    pub fn abandon(self) {
        // Mark it as not drawn (even if it is) so that Drop will not try to
        // hide it.
        self.inner.lock().unwrap().progress_drawn = false;
        // Nothing to do; consuming it is enough?
    }

    /// Update the model, and queue a redraw of the screen for later.
    pub fn update<U>(&self, update_fn: U)
    where
        U: FnOnce(&mut M),
    {
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
        if atty::isnt(atty::Stream::Stdout) || !ansi::enable_windows_ansi() {
            options.progress_enabled = false;
        }
        View {
            inner: Mutex::new(InnerView::new(
                model,
                io::stdout(),
                options,
                WidthStrategy::Stdout,
            )),
        }
    }
}

impl<M: Model> View<M, io::Stderr> {
    /// Construct a new progress view, drawn to stderr.
    ///
    /// `model` is the application-defined initial model.
    pub fn to_stderr(model: M, mut options: ViewOptions) -> View<M, io::Stderr> {
        if atty::isnt(atty::Stream::Stderr) || !ansi::enable_windows_ansi() {
            options.progress_enabled = false;
        }
        View {
            inner: Mutex::new(InnerView::new(
                model,
                io::stderr(),
                options,
                WidthStrategy::Stderr,
            )),
        }
    }
}

impl<M: Model, W: Write> View<M, W> {
    /// Construct a new progress view writing to an arbitrary
    /// [std::io::Write] stream.
    ///
    /// This is probably mostly useful for testing: most applications
    /// will want [View::new].
    ///
    /// This function assumes the stream is a tty and capable of drawing
    /// progress bars through ANSI sequences, and does not try to
    /// detect whether this is true, as [View::new] does.
    ///
    /// Views constructed by this model use a fixed terminal width, rather
    /// than trying to dynamically measure the terminal width.
    pub fn write_to(model: M, options: ViewOptions, out: W, width: usize) -> View<M, W> {
        View {
            inner: Mutex::new(InnerView::new(
                model,
                out,
                options,
                WidthStrategy::Fixed(width),
            )),
        }
    }
}

impl<M: Model, Out: Write> io::Write for View<M, Out> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.inner.lock().unwrap().write(buf)
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
    progress_drawn: bool,

    /// Number of lines the cursor is below the line where the progress bar
    /// should next be drawn.
    cursor_y: usize,

    /// True if there's an incomplete line of output printed, and the
    /// progress bar can't be drawn until it's completed.
    incomplete_line: bool,

    options: ViewOptions,

    /// How to determine the terminal width before output is rendered.
    width_strategy: WidthStrategy,
}

impl<M: Model, Out: Write> InnerView<M, Out> {
    fn new(
        model: M,
        out: Out,
        options: ViewOptions,
        width_strategy: WidthStrategy,
    ) -> InnerView<M, Out> {
        InnerView {
            out,
            model,
            options,
            width_strategy,
            progress_drawn: false,
            cursor_y: 0,
            incomplete_line: false,
        }
    }

    fn paint_progress(&mut self) -> io::Result<()> {
        if !self.options.progress_enabled || self.incomplete_line {
            return Ok(());
        }
        if let Some(width) = self.width_strategy.width() {
            // TODO: Throttle, and keep track of the last update.
            let rendered = self.model.render(width);
            let rendered = rendered.strip_suffix('\n').unwrap_or(&rendered);
            write!(
                self.out,
                "{}{}{}{}",
                ansi::up_n_lines_and_home(self.cursor_y),
                ansi::DISABLE_LINE_WRAP,
                ansi::CLEAR_TO_END_OF_LINE,
                rendered,
            )?;
            self.out.flush()?;

            self.progress_drawn = true;
            self.cursor_y = rendered.as_bytes().iter().filter(|b| **b == b'\n').count();
        }
        Ok(())
    }

    /// Clear the progress bars off the screen, leaving it ready to
    /// print other output.
    fn hide(&mut self) -> io::Result<()> {
        if self.progress_drawn {
            // todo!("move up the right number of lines then clear downwards, then update model");
            write!(
                self.out,
                "{}{}{}",
                ansi::up_n_lines_and_home(self.cursor_y),
                ansi::CLEAR_TO_END_OF_SCREEN,
                ansi::ENABLE_LINE_WRAP,
            )
            .unwrap();
            self.progress_drawn = false;
            self.cursor_y = 0;
        }
        Ok(())
    }

    fn update<U>(&mut self, update_fn: U) -> io::Result<()>
    where
        U: FnOnce(&mut M),
    {
        update_fn(&mut self.model);
        self.paint_progress()
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(last) = buf.last() {
            self.incomplete_line = *last != b'\n';
        } else {
            return Ok(0);
        }
        self.hide()?;
        if !buf.ends_with(b"\n") {
            self.incomplete_line = true;
        }
        self.out.write_all(buf)?;
        self.out.flush()?;
        Ok(buf.len())
    }
}

/// Options controlling a View.
///
/// These are supplied to a constructor like [View::new] and cannot be changed after the view is created.
///
/// The default options created by [ViewOptions::default] should be reasonable
/// for most applications.
///
/// # Example
/// ```
/// let options = nutmeg::ViewOptions::default()
///     .progress_enabled(false); // Don't draw bars, only print.
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewOptions {
    /// Target interval to repaint the progress bar.
    update_interval: Duration,

    /// How long to wait after printing output before drawing the progress bar again.
    print_holdoff: Duration,

    /// Is the progress bar drawn at all?
    ///
    /// This value will be ignored by [View::new] if stdout is not a terminal.
    progress_enabled: bool,
}

impl ViewOptions {
    /// Set whether the progress bar will be drawn.
    ///
    /// By default it is drawn.
    pub fn progress_enabled(self, progress_enabled: bool) -> ViewOptions {
        ViewOptions {
            progress_enabled,
            ..self
        }
    }
}

impl Default for ViewOptions {
    /// Create default reasonable view options.
    ///
    /// The update interval and print holdoff are 250ms, and the progress bar is enabled.
    fn default() -> ViewOptions {
        ViewOptions {
            update_interval: Duration::from_millis(250),
            print_holdoff: Duration::from_millis(250),
            progress_enabled: true,
        }
    }
}
