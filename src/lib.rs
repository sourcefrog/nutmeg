// Copyright 2022-2023 Martin Pool.

/*!

Nutmeg draws multi-line terminal progress bars to an ANSI terminal.

By contrast to other Rust progress-bar libraries, Nutmeg has no built-in
concept of what the progress bar or indicator should look like: the application
has complete control.

# Concept

Nutmeg has three key types: Model, View, and Options.

## Model

A type implementing the [Model] trait holds whatever information is needed to draw the
progress bars. This might be the start time of the operation, the number of things
processed, the amount of data transmitted or received, the currently active tasks, whatever...

The Model can be any of these things, from simplest to most powerful:

1. Any type that implements [std::fmt::Display], such as a String or integer.
2. One of the provided [models].
3. An application-defined struct (or enum or other type) that implements [Model].

The model is responsible for rendering itself into a String, optionally with ANSI styling,
by implementing [Model::render] (or [std::fmt::Display]).  Applications might
choose to use any of the Rust crates that can render ANSI control codes into a
string, such as yansi.

The application is responsible for deciding whether or not to
color its output, for example by consulting `$CLICOLORS` or its own command line.

Models can optionally provide a "final message" by implementing
[Model::final_message], which will be left on the screen when the view is finished.

If one overall operation represents several concurrent operations then the
application can, for example, represent them in a collection within the Model, and
render them into multiple lines, or multiple sections in a single line.
(See `examples/multithreaded.rs`.)

## View

To get the model on to the terminal the application must create a [View], typically
with [View::new], passing the initial model. The view takes ownership of the model.

The application then updates the model state via [View::update], which may decide
to paint the view to the terminal, subject to rate-limiting and other constraints.

The view has an internal mutex and is `Send` and `Sync`,
so it can be shared freely across threads.

The view automatically erases itself from the screen when it is dropped.

While the view is on the screen, the application can print messages interleaved
with the progress bar by either calling [View::message], or treating it as a [std::io::Write]
destination, for example for [std::writeln].

Errors in writing to the terminal cause a panic.

## Options

A small [Options] type, passed to the View constructor, allows turning progress bars
off, setting rate limits, etc.

In particular applications might choose to construct all [Options] from a single function
that respects an application-level option for whether progress bars should be drawn.

## Utility functions

This crate also provides a few free functions such as [estimate_remaining],
that can be helpful in implementing [Model::render].

# Example

```
use std::io::Write; // to support write!()

// 1. Define a struct holding all the application state necessary to
// render the progress bar.
#[derive(Default)]
struct Model {
    i: usize,
    total: usize,
    last_file_name: String,
}

// 2. Define how to render the progress bar as a String.
impl nutmeg::Model for Model {
    fn render(&mut self, _width: usize) -> String {
        format!("{}/{}: {}", self.i, self.total, self.last_file_name)
    }
}

fn main() -> std::io::Result<()> {
    // 3. Create a View when you want to draw a progress bar.
    let mut view = nutmeg::View::new(Model::default(),
        nutmeg::Options::default());

    // 4. As the application runs, update the model via the view.
    let total_work = 100;
    view.update(|model| model.total = total_work);
    for i in 0..total_work {
        view.update(|model| {
            model.i += 1;
            model.last_file_name = format!("file{}.txt", i);
        });
        // 5. Interleave text output lines by writing to the view.
        if i % 10 == 3 {
            writeln!(view, "reached {}", i)?;
        }
    }

    // 5. The bar is automatically erased when dropped.
    Ok(())
}
```

See the `examples/` directory for more.

# Performance

Nutmeg's goal is that [View::update] is cheap enough that applications can call it
fairly freely when there are small updates. The library takes care of rate-limiting
updates to the terminal, as configured in the [Options].

Each call to [View::update] will take a `parking_lot` mutex and check the
system time, in addition to running the callback and some function-call overhead.

The model is only rendered to a string, and the string printed to a terminal, if
sufficient time has passed since it was last painted.

The `examples/bench.rs` sends updates as fast as possible to a model containing a
single `u64`, from a single thread. As of 2022-03-22, on a 2019 Core i9 Macbook Pro,
it takes about 500ms to send 10e6 updates, or 50ns/update.

# Project status

Nutmeg is a young library. Although the API will not break gratuitously,
it may evolve in response to experience and feedback in every pre-1.0 release.

If the core ideas prove useful and the API remains stable for an extended period
then the author intends to promote it to 1.0, after which the API will respect
Rust stability conventions.

Changes are described in the [changelog](#Changelog) in the top-level Rustdoc,
below.

Constructive feedback on integrations that work well, or that don't work well,
is welcome.

# Potential future features

* Draw updates from a background thread, so that it will keep ticking even
  if not actively updated, and to better handle applications that send a
  burst of updates followed by a long pause. The background thread will
  eventually paint the last drawn update.

* Also set the window title from the progress model, perhaps by a different
  render function?

*/

#![warn(missing_docs)]

use std::env;
use std::fmt::Display;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

mod ansi;
mod helpers;
pub mod models;
mod width;
#[cfg(windows)]
mod windows;

pub mod _changelog {
    #![doc = include_str!("../NEWS.md")]
    #[allow(unused_imports)]
    use super::*; // so that hyperlinks work
}

use crate::width::WidthStrategy;

pub use crate::helpers::*;

/// An application-defined type that holds whatever state is relevant to the
/// progress bar, and that can render it into one or more lines of text.
pub trait Model {
    /// Render this model into a string to draw on the console.
    ///
    /// This is called by the View when it wants to repaint the screen
    /// after [View::update] was called.
    ///
    /// Future versions of this library may call this function from a different
    /// thread.
    ///
    /// The `width` argument advises the model rendering code of the width of
    /// the terminal. The `render` implementation may make us of this to, for
    /// example, draw a full-width progress bar, or to selectively truncate
    /// sections within the line.
    ///
    /// The model may also ignore the `width` parameter and return a string
    /// of any width, in which case it will be truncated to fit on the
    /// screen.
    ///
    /// The rendered version may contain ANSI escape sequences for coloring,
    /// etc, but should not move the cursor.
    ///
    /// Lines are separarated by `\n`. If there is a final `\n` it is ignored.
    ///
    /// # Example
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

    /// Optionally render a final message when the view is finished.
    ///
    /// For example this could be used to print the amount of work done
    /// after the work is complete.
    ///
    /// By default this prints nothing.
    ///
    /// The final message may contain ANSI styling and may be multiple lines,
    /// but it should not have a final newline, unless a trailing blank line
    /// is desired.
    ///
    /// This is called by [View::finish] or when the view is dropped.
    /// The final message is not printed when the view is abandoned by
    /// [View::abandon].
    fn final_message(&mut self) -> String {
        String::new()
    }
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
/// use nutmeg::{Options, View};
///
/// let view = View::new(0, Options::default());
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
pub struct View<M: Model> {
    /// The real state of the view.
    ///
    /// The contents are always Some unless the View has been explicitly destroyed,
    /// in which case this makes Drop a no-op.
    inner: Mutex<Option<InnerView<M>>>,
}

impl<M: Model> View<M> {
    /// Construct a new progress view, drawn to stdout.
    ///
    /// `model` is the application-defined initial model. The View takes
    /// ownership of the model, after which the application can update
    /// it through [View::update].
    ///
    /// `options` can typically be `Options::default`.
    ///
    /// On Windows, this enables use of ANSI sequences for styling stdout.
    ///
    /// Even if progress bars are enabled in the [Options], they will be
    /// disabled under some conditions:
    /// * If stdout is not a tty,
    /// * On Windows, if ANSI sequences cannot be enabled.
    /// * If the `$TERM` environment variable is `DUMB`.
    ///
    /// This constructor arranges that output from the progress view will be
    /// captured by the Rust test framework and not leak to stdout, but
    /// detection of whether to show progress bars may not work correctly.
    pub fn new(model: M, mut options: Options) -> View<M> {
        if !options.destination.is_possible() {
            options.progress_enabled = false;
        }
        View::from_inner(InnerView::new(model, options))
    }

    /// Private constructor from an InnerView.
    fn from_inner(inner_view: InnerView<M>) -> View<M> {
        View {
            inner: Mutex::new(Some(inner_view)),
        }
    }

    /// Stop using this progress view.
    ///
    /// If the progress bar is currently visible, it will be left behind on the
    /// screen.
    ///
    /// Returns the model.
    pub fn abandon(self) -> M {
        // Mark it as not drawn (even if it is) so that Drop will not try to
        // hide it.
        self.inner
            .lock()
            .take()
            .expect("inner state is still present")
            .abandon()
            .unwrap()
    }

    /// Erase the model from the screen (if drawn), destroy it, and return the model.
    pub fn finish(self) -> M {
        self.inner
            .lock()
            .take()
            .expect("inner state is still present")
            .finish()
    }

    /// Update the model, and possibly redraw the screen to reflect the
    /// update.
    ///
    /// The progress bar may be repainted with the results of the update,
    /// if all these conditions are true:
    ///
    /// * The view is not suspended (by [View::suspend]).
    /// * Progress bars are enabled by [Options::progress_enabled].
    /// * The terminal seems capable of drawing progress bars.
    /// * The progress bar was not drawn too recently, as controlled by
    ///   [Options::update_interval].
    /// * A message was not printed too recently, as controlled by
    ///   [Options::print_holdoff].
    /// * An incomplete message line isn't pending: in other words the
    ///   last message written to the view, if any, had a final newline.
    ///
    /// If the view decides to repaint the progress bar it will call
    /// [Model::render]. In a future release redrawing may be done on a
    /// different thread.
    ///
    /// The `update_fn` may return a value, and this is returned from
    /// `update`.
    pub fn update<U, R>(&self, update_fn: U) -> R
    where
        U: FnOnce(&mut M) -> R,
    {
        self.inner.lock().as_mut().unwrap().update(update_fn)
    }

    /// Hide the progress bar if it's currently drawn, and leave it
    /// hidden until [View::resume] is called.
    pub fn suspend(&self) {
        self.inner.lock().as_mut().unwrap().suspend().unwrap()
    }

    /// Hide the progress bar if it's currently drawn, but allow it
    /// to be redrawn when the model is next updated.
    pub fn hide(&self) {
        self.inner.lock().as_mut().unwrap().hide().unwrap()
    }

    /// Allow the progress bar to be drawn again, reversing the effect
    /// of [View::suspend].
    pub fn resume(&self) {
        self.inner.lock().as_mut().unwrap().resume().unwrap()
    }

    /// Set the value of the fake clock, for testing.
    ///
    /// Panics if [Options::fake_clock] was not previously set.
    ///
    /// Moving the clock backwards in time may cause a panic.
    pub fn set_fake_clock(&self, fake_clock: Instant) {
        self.inner
            .lock()
            .as_mut()
            .unwrap()
            .set_fake_clock(fake_clock)
    }

    /// Inspect the view's model.
    ///
    /// The function `f` is applied to the model, and then the result
    /// of `f` is returned by `inspect_model`.
    ///
    /// ```
    /// use nutmeg::{Options, View};
    ///
    /// let view = View::new(10, Options::default());
    /// view.update(|model| *model += 3);
    /// assert_eq!(view.inspect_model(|m| *m), 13);
    /// ```
    pub fn inspect_model<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut M) -> R,
    {
        f(&mut self.inner.lock().as_mut().unwrap().model)
    }

    /// Print a message to the view.
    ///
    /// The progress bar, if present, is removed to print the message
    /// and then remains off for a time controlled by [Options::print_holdoff].
    ///
    /// The message may contain ANSI control codes for styling.
    ///
    /// The message may contain multiple lines.
    ///
    /// Typically the message should end with `\n`.
    ///
    /// If the last character of the message is *not* `\n` then the incomplete
    /// line remains on the terminal, and the progress bar will not be painted
    /// until it is completed by a message finishing in `\n`.
    ///
    /// This is equivalent to `write!(view, ...)` except:
    /// * [std::io::Write::write] requires a `&mut View`, while `message`
    ///   can be called on a `&View`.
    /// * `message` panics on an error writing to the terminal; `write!` requires
    ///   the caller to handle a `Result`.
    /// * `write!` integrates string formatting; `message` does not, and typically
    ///   would be called with the results of `format!()`.
    ///
    /// ```
    /// use nutmeg::{Options, View};
    ///
    /// let view = View::new(0, Options::default());
    /// // ...
    /// view.message(format!("{} splines reticulated\n", 42));
    /// ```
    pub fn message<S: AsRef<str>>(&self, message: S) {
        self.inner
            .lock()
            .as_mut()
            .unwrap()
            .write(message.as_ref().as_bytes())
            .expect("writing message");
    }

    /// Print a message from a byte buffer.
    ///
    /// This is the same as [View::message] but takes an `AsRef<[u8]>`, such as a slice.
    ///
    /// Most destinations will expect the contents to be UTF-8.
    ///
    /// ```
    /// use nutmeg::{Options, View};
    ///
    /// let view = View::new("model content", Options::default());
    /// view.message_bytes(b"hello crow\n");
    /// ```
    pub fn message_bytes<S: AsRef<[u8]>>(&self, message: S) {
        self.inner
            .lock()
            .as_mut()
            .unwrap()
            .write(message.as_ref())
            .expect("writing message");
    }

    /// If the view's destination is [Destination::Capture], returns the buffer
    /// of captured output. Panics if the destination is not [Destination::Capture].
    ///
    /// The buffer is returned in an Arc so that it remains valid after the View
    /// is dropped.
    ///
    /// This is intended for use in testing.
    ///
    /// # Example
    ///
    /// ```
    /// use nutmeg::{Destination, Options, View};
    ///
    /// let view = View::new(0, Options::default().destination(Destination::Capture));
    /// let output = view.captured_output();
    /// view.message("Captured message\n");
    /// drop(view);
    /// assert_eq!(output.lock().as_str(), "Captured message\n");
    /// ```
    pub fn captured_output(&self) -> Arc<Mutex<String>> {
        self.inner.lock().as_mut().unwrap().captured_output()
    }
}

impl<M: Model> io::Write for View<M> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.inner.lock().as_mut().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<M: Model> Drop for View<M> {
    fn drop(&mut self) {
        // Only try lock here: don't hang if it's locked or panic
        // if it's poisoned. And, do nothing if the View has already been
        // finished, in which case the contents of the Mutex will be None.
        if let Some(mut inner_guard) = self.inner.try_lock() {
            if let Some(inner) = Option::take(&mut inner_guard) {
                inner.finish();
            }
        }
    }
}

fn is_dumb_term() -> bool {
    env::var("TERM").map_or(false, |s| s.eq_ignore_ascii_case("dumb"))
}

/// The real contents of a View, inside a mutex.
struct InnerView<M: Model> {
    /// Current application model.
    model: M,

    /// True if the progress bar is suspended, and should not be drawn.
    suspended: bool,

    /// Whether the progress bar is drawn, etc.
    state: State,

    options: Options,

    /// How to determine the terminal width before output is rendered.
    width_strategy: WidthStrategy,

    /// The current time on the fake clock, if it is enabled.
    fake_clock: Instant,

    /// Captured output, if active.
    capture_buffer: Option<Arc<Mutex<String>>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum State {
    /// Progress is not visible and nothing was recently printed.
    None,
    /// Progress bar is currently displayed.
    ProgressDrawn {
        /// Last time it was drawn.
        last_drawn_time: Instant,
        /// Number of lines the cursor is below the line where the progress bar
        /// should next be drawn.
        cursor_y: usize,
        /// The rendered string last drawn.
        last_drawn_string: String,
    },
    /// Messages were written, and the progress bar is not visible.
    Printed { last_printed: Instant },
    /// An incomplete message line has been printed, so the progress bar can't
    /// be drawn until it's removed.
    IncompleteLine,
}

impl<M: Model> InnerView<M> {
    fn new(model: M, options: Options) -> InnerView<M> {
        let width_strategy = options.destination.width_strategy();
        let capture_buffer = if options.destination == Destination::Capture {
            Some(Arc::new(Mutex::new(String::new())))
        } else {
            None
        };
        InnerView {
            capture_buffer,
            fake_clock: Instant::now(),
            model,
            options,
            state: State::None,
            suspended: false,
            width_strategy,
        }
    }

    fn finish(mut self) -> M {
        let _ = self.hide();
        let final_message = self.model.final_message();
        if !final_message.is_empty() {
            self.write_output(&format!("{final_message}\n"));
        }
        self.model
    }

    fn abandon(mut self) -> io::Result<M> {
        match self.state {
            State::ProgressDrawn { .. } => {
                self.write_output("\n");
            }
            State::IncompleteLine | State::None | State::Printed { .. } => (),
        }
        self.state = State::None; // so that drop does not attempt to erase
        Ok(self.model)
    }

    /// Return the real or fake clock.
    fn clock(&self) -> Instant {
        if self.options.fake_clock {
            self.fake_clock
        } else {
            Instant::now()
        }
    }

    fn paint_progress(&mut self) -> io::Result<()> {
        if !self.options.progress_enabled || self.suspended {
            return Ok(());
        }
        let now = self.clock();
        match self.state {
            State::IncompleteLine => return Ok(()),
            State::None => (),
            State::Printed { last_printed } => {
                if now - last_printed < self.options.print_holdoff {
                    return Ok(());
                }
            }
            State::ProgressDrawn {
                last_drawn_time, ..
            } => {
                if now - last_drawn_time < self.options.update_interval {
                    return Ok(());
                }
            }
        }
        if let Some(width) = self.width_strategy.width() {
            let mut rendered = self.model.render(width);
            if rendered.ends_with('\n') {
                // Handle models that incorrectly add a trailing newline, rather than
                // leaving a blank line. (Maybe we should just let them fix it, and
                // be simpler?)
                rendered.pop();
            }
            let mut buf = String::new();
            if let State::ProgressDrawn {
                ref last_drawn_string,
                cursor_y,
                ..
            } = self.state
            {
                if *last_drawn_string == rendered {
                    return Ok(());
                }
                buf.push_str(&ansi::up_n_lines_and_home(cursor_y));
            }
            buf.push_str(ansi::DISABLE_LINE_WRAP);
            buf.push_str(ansi::CLEAR_TO_END_OF_SCREEN);
            buf.push_str(&rendered);
            self.write_output(&buf);
            let cursor_y = rendered.as_bytes().iter().filter(|b| **b == b'\n').count();
            self.state = State::ProgressDrawn {
                last_drawn_time: now,
                last_drawn_string: rendered,
                cursor_y,
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
                self.write_output(&format!(
                    "{}{}{}",
                    ansi::up_n_lines_and_home(cursor_y),
                    ansi::CLEAR_TO_END_OF_SCREEN,
                    ansi::ENABLE_LINE_WRAP,
                ));
                self.state = State::None;
            }
            State::None | State::IncompleteLine | State::Printed { .. } => {}
        }
        Ok(())
    }

    fn update<U, R>(&mut self, update_fn: U) -> R
    where
        U: FnOnce(&mut M) -> R,
    {
        let r = update_fn(&mut self.model);
        self.paint_progress().unwrap();
        r
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.hide()?;
        self.state = if buf.ends_with(b"\n") {
            State::Printed {
                last_printed: self.clock(),
            }
        } else {
            State::IncompleteLine
        };
        self.write_output(std::str::from_utf8(buf).expect("message is not UTF-8"));
        Ok(buf.len())
    }

    /// Set the value of the fake clock, for testing.
    fn set_fake_clock(&mut self, fake_clock: Instant) {
        assert!(self.options.fake_clock, "fake clock is not enabled");
        self.fake_clock = fake_clock;
    }

    fn write_output(&mut self, buf: &str) {
        match &mut self.options.destination {
            Destination::Stdout => {
                print!("{buf}");
                io::stdout().flush().unwrap();
            }
            Destination::Stderr => {
                eprint!("{buf}");
                io::stderr().flush().unwrap();
            }
            Destination::Capture => {
                self.capture_buffer
                    .as_mut()
                    .expect("capture buffer is not allocated")
                    .lock()
                    .push_str(buf);
            }
        }
    }

    fn captured_output(&mut self) -> Arc<Mutex<String>> {
        self.capture_buffer
            .as_ref()
            .expect("capture buffer allocated")
            .clone()
    }
}

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
    update_interval: Duration,

    /// How long to wait after printing output before drawing the progress bar again.
    print_holdoff: Duration,

    /// Is the progress bar drawn at all?
    progress_enabled: bool,

    /// Use a fake clock for testing.
    fake_clock: bool,

    /// Write progress and messages to stdout, stderr, or a capture buffer for tests?
    destination: Destination,
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

/// Destinations for progress bar output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    /// Draw to stdout.
    Stdout,
    /// Draw to stderr.
    Stderr,
    /// Draw to an internal capture buffer, which can be retrieved with [View::captured_output].
    ///
    /// This is intended for testing.
    ///
    /// A width of 80 columns is used.
    Capture,
}

impl Destination {
    fn is_possible(&self) -> bool {
        match self {
            Destination::Stdout => {
                atty::is(atty::Stream::Stdout) && !is_dumb_term() && ansi::enable_windows_ansi()
            }
            Destination::Stderr => {
                atty::is(atty::Stream::Stderr) && !is_dumb_term() && ansi::enable_windows_ansi()
            }
            Destination::Capture => true,
        }
    }

    fn width_strategy(&self) -> WidthStrategy {
        match self {
            Destination::Stdout => WidthStrategy::Stdout,
            Destination::Stderr => WidthStrategy::Stderr,
            Destination::Capture => WidthStrategy::Fixed(80),
        }
    }
}
