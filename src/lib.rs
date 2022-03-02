// Copyright 2022 Martin Pool.

//! Nutmeg draws terminal progress bars whose appearance is completely controlled
//! by the application.
//!
//! # Concept
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
//! # Example
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
//! # Potential future features
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
//!
//! # Changelog
//!
//! ## 0.0.0
//!
//! * The application has complete control of styling, including coloring etc.
//! * Draw and erase progress bars.
//! * Write messages "under" the progress bar with `writeln!(view, ...)`. The
//!   bar is automatically suspended and restored. If the message has no final
//!   newline, the bar remains suspended until the line is completed.
//!
//! ## 0.0.1
//!
//! * Rate-limit updates to the terminal, controlled by
//!   [ViewOptions::update_interval] and [ViewOptions::print_holdoff].
//!
//! * Fix a bug where the bar was sometimes not correctly erased
//!   by [View::suspend].

#![warn(missing_docs)]

use std::fmt::Display;
use std::io::{self, Write};
use std::sync::Mutex;
use std::time::{Duration, Instant};

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
        self.inner.lock().unwrap().abandon().unwrap();
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

    /// Hide the progress bar if it's currently drawn, and leave it
    /// hidden until [View::resume] is called.
    pub fn suspend(&self) {
        self.inner.lock().unwrap().suspend().unwrap()
    }

    /// Allow the progress bar to be drawn again.
    pub fn resume(&self) {
        self.inner.lock().unwrap().resume().unwrap()
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

    /// True if the progress bar is suspended, and should not be drawn.
    suspended: bool,

    /// Whether the progress bar is drawn, etc.
    state: State,

    options: ViewOptions,

    /// How to determine the terminal width before output is rendered.
    width_strategy: WidthStrategy,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    /// Progress is not visible and nothing was recently printed.
    None,
    /// Progress bar is currently displayed.
    ProgressDrawn {
        since: Instant,
        /// Number of lines the cursor is below the line where the progress bar
        /// should next be drawn.
        cursor_y: usize,
    },
    /// Messages were written, and the progress bar is not visible.
    Printed { since: Instant },
    /// An incomplete message line has been printed, so the progress bar can't
    /// be drawn until it's removed.
    IncompleteLine,
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
            state: State::None,
            suspended: false,
        }
    }

    fn paint_progress(&mut self) -> io::Result<()> {
        if !self.options.progress_enabled || self.suspended {
            return Ok(());
        }
        match self.state {
            State::IncompleteLine => return Ok(()),
            State::None => (),
            State::Printed { since } => {
                if since.elapsed() < self.options.print_holdoff {
                    return Ok(());
                }
            }
            State::ProgressDrawn { since, .. } => {
                if since.elapsed() < self.options.update_interval {
                    return Ok(());
                }
            }
        }
        if let Some(width) = self.width_strategy.width() {
            let rendered = self.model.render(width);
            let rendered = rendered.strip_suffix('\n').unwrap_or(&rendered);
            if let State::ProgressDrawn { cursor_y, .. } = self.state {
                write!(self.out, "{}", ansi::up_n_lines_and_home(cursor_y))?;
            }
            write!(
                self.out,
                "{}{}{}",
                ansi::DISABLE_LINE_WRAP,
                ansi::CLEAR_TO_END_OF_LINE,
                rendered,
            )?;
            self.out.flush()?;
            self.state = State::ProgressDrawn {
                since: Instant::now(),
                cursor_y: rendered.as_bytes().iter().filter(|b| **b == b'\n').count(),
            };
        }
        Ok(())
    }

    /// Hide the progress bar and leave it hidden until it is resumed.
    fn suspend(&mut self) -> io::Result<()> {
        self.suspended = true;
        self.hide()
    }

    fn resume(&mut self) -> io::Result<()> {
        self.suspended = false;
        self.paint_progress()
    }

    /// Clear the progress bars off the screen, leaving it ready to
    /// print other output.
    fn hide(&mut self) -> io::Result<()> {
        match self.state {
            State::ProgressDrawn { cursor_y, .. } => {
                write!(
                    self.out,
                    "{}{}{}",
                    ansi::up_n_lines_and_home(cursor_y),
                    ansi::CLEAR_TO_END_OF_SCREEN,
                    ansi::ENABLE_LINE_WRAP,
                )
                .unwrap();
                self.out.flush()?;
                self.state = State::None;
            }
            State::None | State::IncompleteLine | State::Printed { .. } => {}
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
        if buf.is_empty() {
            return Ok(0);
        }
        self.hide()?;
        self.state = if buf.ends_with(b"\n") {
            State::Printed {
                since: Instant::now(),
            }
        } else {
            State::IncompleteLine
        };
        self.out.write_all(buf)?;
        self.out.flush()?;
        Ok(buf.len())
    }

    fn abandon(&mut self) -> io::Result<()> {
        match self.state {
            State::ProgressDrawn { .. } => {
                self.out.write_all(b"\n")?;
            }
            State::IncompleteLine | State::None | State::Printed { .. } => (),
        }
        self.state = State::None; // so that drop does not attempt to erase
        Ok(())
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
    progress_enabled: bool,
}

impl ViewOptions {
    /// Set whether the progress bar will be drawn.
    ///
    /// By default it is drawn, except that this value will be ignored by [View::new] if stdout is not a terminal.
    pub fn progress_enabled(self, progress_enabled: bool) -> ViewOptions {
        ViewOptions {
            progress_enabled,
            ..self
        }
    }

    /// Set the minimal interval to repaint the progress bar.
    ///
    /// `Duration::ZERO` can be used to cause the bar to repaint on every update.
    pub fn update_interval(self, update_interval: Duration) -> ViewOptions {
        ViewOptions {
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
    pub fn print_holdoff(self, print_holdoff: Duration) -> ViewOptions {
        ViewOptions {
            print_holdoff,
            ..self
        }
    }
}

impl Default for ViewOptions {
    /// Create default reasonable view options.
    ///
    /// The update interval and print holdoff are 100ms, and the progress bar is enabled.
    fn default() -> ViewOptions {
        ViewOptions {
            update_interval: Duration::from_millis(100),
            print_holdoff: Duration::from_millis(100),
            progress_enabled: true,
        }
    }
}
