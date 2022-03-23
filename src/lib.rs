// Copyright 2022 Martin Pool.

/*!

Nutmeg draws terminal progress bars whose appearance is completely controlled
by the application.

# Concept

By contrast to other Rust progress-bar libraries, Nutmeg has no built-in
concept of what the progress bar or indicator should look like: this is
entirely under the control of the application.

The application is responsible for:

1. Defining a "model" type that holds whatever information is relevant to drawing
   progress bars: the time elapsed, number of things processed, currently active tasks,
   total expected work, whatever...
2. Implementing the [Model] trait for your model. This has only one mandatory method,
   [Model::render], which renders the model into styled text.
3. Constructing a [View] to draw a progress bar.
4. Updating the model when appropriate by calling [View::update], passing a callback
   that mutates the state.
5. Printing any messages while the [View] is in use
   via `writeln!(view, ...)` or [View::message].

Some applications might find the provided [models] suit their needs, in which case they
can skip steps 1 and 2.

The application can control colors and styling by including ANSI
escape sequences in the rendered string, for example by using the
`yansi` crate.

The application is responsible for deciding whether or not to
color its output, for example by consulting `$CLICOLORS`.

The Nutmeg library is responsible for:

* Periodically drawing the progress bar in response to updates, including
  horizontally truncating output to fit on the screen.
* Removing the progress bar when the view is finished or dropped.
* Coordinating to hide the bar to print text output, and restore it
  afterwards.
* Limiting the rate at which updates are drawn to the screen.
* Disabling progress bars if stdout is not a terminal.

Errors in writing to the terminal cause a panic.

Nutmeg only supports ANSI terminals, which are supported on all Unix
and Windows 10 and later.

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

# Other features

The [models] module provides some predefined models, for example counting `i` of `n` items
of work complete with an extrapolated ETA.

Models can optionally provide a "final message" by implementing [Model::final_message], which
will be left on the screen when the view is finished.

This crate also provides a few free functions such as [estimate_remaining],
that can be helpful in rendering progress bars.

# Potential future features

* Draw updates from a background thread, so that it will keep ticking even
  if not actively updated, and to better handle applications that send a
  burst of updates followed by a long pause. The background thread will
  eventually paint the last drawn update.

* Also set the window title from the progress model, perhaps by a different
  render function?

* Better detection of when to draw progress or not. Possibly look at
  `TERM=dumb`; possibly hook in to a standard Rust mechanism e.g.
  <https://github.com/rust-cli/team/issues/15#issuecomment-891350115>.

# Changelog

## 0.0.3

Not released yet.

* API change: The "Write" type representing the destination is no longer
  part of the visible public signature of [View], to hide complexity and
  since it is not helpful to most callers.

* New: [percent_done] and [estimate_remaining] functions to help in rendering progress bars.

* New: The [models] mod provides some generally-useful basic models,
  specifically [models::StringPair], [models::UnboundedModel] and [models::LinearModel].
  These build only on the public interface of Nutmeg, so also constitute examples of what can be done in
  application-defined models.

* New: [View::finish] removes the progress bar (if painted) and returns the [Model].
  [View::abandon] now also returns the model.

* New: [Model::final_message] to let the model render a message to be printed when work
  is complete.

* New: The callback to [View::update] may return a value, and this is passed back to the caller
  of [View::update].

* New: [models::BasicModel] allows simple cases to supply both an intital value
  and a render function inline in the [View] constructor call, avoiding any
  need to define a [Model] struct.

## 0.0.2

Released 2022-03-07

* API change: Renamed `nutmeg::ViewOptions` to just `nutmeg::Options`.

* Fixed: A bug that caused leftover text when multi-line bars shrink in width.

* Fixed: The output from bars created with [View::new] and [View::to_stderr] in
  Rust tests is captured with the test output rather than leaking through
  to cargo's output.

* New method [View::message] to print a message to the terminal, as an alternative
  to using `write!()`.

* New `example/multithreaded.rs` showing how a View and Model can be shared
  across threads.

## 0.0.1

* Rate-limit updates to the terminal, controlled by
  `ViewOptions::update_interval` and `ViewOptions::print_holdoff`.

* Fix a bug where the bar was sometimes not correctly erased
  by [View::suspend].

* Change to [`parking_lot`](https://docs.rs/parking_lot) mutexes in the implementation.

## 0.0.0

* The application has complete control of styling, including coloring etc.
* Draw and erase progress bars.
* Write messages "under" the progress bar with `writeln!(view, ...)`. The
  bar is automatically suspended and restored. If the message has no final
  newline, the bar remains suspended until the line is completed.

*/

#![warn(missing_docs)]

use std::fmt::Display;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

mod ansi;
mod helpers;
pub mod models;
mod width;
#[cfg(windows)]
mod windows;

use crate::width::WidthStrategy;

pub use crate::helpers::*;

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
/// let view = nutmeg::View::new(0, nutmeg::Options::default());
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
    /// On Windows, this enables use of ANSI sequences for styling stdout.
    ///
    /// Even if progress bars are enabled in the [Options], they will be
    /// disabled if stdout is not a tty, or if it does not support ANSI
    /// sequences (on Windows).
    ///
    /// This constructor arranges that output from the progress view will be
    /// captured by the Rust test framework and not leak to stdout, but
    /// detection of whether to show progress bars may not work correctly.
    pub fn new(model: M, mut options: Options) -> View<M> {
        if atty::isnt(atty::Stream::Stdout) || !ansi::enable_windows_ansi() {
            options.progress_enabled = false;
        }
        View {
            inner: Mutex::new(Some(InnerView::new(
                model,
                WriteTo::Stdout,
                options,
                WidthStrategy::Stdout,
            ))),
        }
    }

    /// Construct a new progress view, drawn to stderr.
    ///
    /// This is the same as [View::new] except that the progress bar, and
    /// any messages emitted through it, are sent to stderr.
    pub fn to_stderr(model: M, mut options: Options) -> View<M> {
        if atty::isnt(atty::Stream::Stderr) || !ansi::enable_windows_ansi() {
            options.progress_enabled = false;
        }
        View {
            inner: Mutex::new(Some(InnerView::new(
                model,
                WriteTo::Stderr,
                options,
                WidthStrategy::Stderr,
            ))),
        }
    }

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
    pub fn write_to<W: Write + Send + 'static>(
        model: M,
        options: Options,
        out: W,
        width: usize,
    ) -> View<M> {
        View {
            inner: Mutex::new(Some(InnerView::new(
                model,
                WriteTo::Write(Box::new(out)),
                options,
                WidthStrategy::Fixed(width),
            ))),
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
    /// let view = nutmeg::View::new(10, nutmeg::Options::default());
    /// view.update(|model| *model += 3);
    /// assert_eq!(view.inspect_model(|m| *m), 13);
    /// ```
    pub fn inspect_model<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&M) -> R,
    {
        f(&self.inner.lock().as_mut().unwrap().model)
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
    /// If the last character of the message is *not* '\n' then the incomplete
    /// line remains on the terminal, and the progress bar will not be painted
    /// until it is completed by a message finishing in `\n`.
    ///
    /// This is equivalent to `write!(view, ...)` except:
    /// * [std::io::Write::write] requires a `&mut View`, while `message`
    ///   can be called on a `&View`.
    /// * `message` panics on an error writing to the terminal; `write!` requires
    ///   the caller to handle a `Result`.
    /// * `write!` integrates string formatting; `message` does not.
    ///
    /// ```
    /// let view = nutmeg::View::new(0, nutmeg::Options::default());
    /// // ...
    /// view.message(&format!("{} splines reticulated\n", 42));
    /// ```
    pub fn message(&self, message: &str) {
        self.inner
            .lock()
            .as_mut()
            .unwrap()
            .write(message.as_bytes())
            .expect("writing message");
    }
}

impl<M: Model> io::Write for View<M> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.inner.lock().as_mut().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
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

/// The real contents of a View, inside a mutex.
struct InnerView<M: Model> {
    /// Current application model.
    model: M,

    /// Where and how to write bars and messages.
    out: WriteTo,

    /// True if the progress bar is suspended, and should not be drawn.
    suspended: bool,

    /// Whether the progress bar is drawn, etc.
    state: State,

    options: Options,

    /// How to determine the terminal width before output is rendered.
    width_strategy: WidthStrategy,

    /// The current time on the fake clock, if it is enabled.
    fake_clock: Instant,
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

impl<M: Model> InnerView<M> {
    fn new(
        model: M,
        write_to: WriteTo,
        options: Options,
        width_strategy: WidthStrategy,
    ) -> InnerView<M> {
        InnerView {
            fake_clock: Instant::now(),
            model,
            options,
            out: write_to,
            state: State::None,
            suspended: false,
            width_strategy,
        }
    }

    fn finish(mut self) -> M {
        let _ = self.hide();
        let final_message = self.model.final_message();
        if !final_message.is_empty() {
            self.out.write_str(&format!("{}\n", final_message));
        }
        self.model
    }

    fn abandon(mut self) -> io::Result<M> {
        match self.state {
            State::ProgressDrawn { .. } => {
                self.out.write_str("\n");
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
            State::Printed { since } => {
                if now - since < self.options.print_holdoff {
                    return Ok(());
                }
            }
            State::ProgressDrawn { since, .. } => {
                if now - since < self.options.update_interval {
                    return Ok(());
                }
            }
        }
        if let Some(width) = self.width_strategy.width() {
            let rendered = self.model.render(width);
            let rendered = rendered.strip_suffix('\n').unwrap_or(&rendered);
            let mut buf = String::new();
            if let State::ProgressDrawn { cursor_y, .. } = self.state {
                buf.push_str(&ansi::up_n_lines_and_home(cursor_y));
            }
            buf.push_str(ansi::DISABLE_LINE_WRAP);
            buf.push_str(ansi::CLEAR_TO_END_OF_SCREEN);
            buf.push_str(rendered);
            self.out.write_str(&buf);
            self.out.flush();
            self.state = State::ProgressDrawn {
                since: now,
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
                self.out.write_str(&format!(
                    "{}{}{}",
                    ansi::up_n_lines_and_home(cursor_y),
                    ansi::CLEAR_TO_END_OF_SCREEN,
                    ansi::ENABLE_LINE_WRAP,
                ));
                self.out.flush();
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
                since: self.clock(),
            }
        } else {
            State::IncompleteLine
        };
        self.out.write_bytes(buf);
        self.out.flush();
        Ok(buf.len())
    }

    /// Set the value of the fake clock, for testing.
    fn set_fake_clock(&mut self, fake_clock: Instant) {
        assert!(self.options.fake_clock, "fake clock is not enabled");
        self.fake_clock = fake_clock;
    }
}

/// Options controlling a View.
///
/// These are supplied to a constructor like [View::new] and cannot be changed after the view is created.
///
/// The default options created by [Options::default] should be reasonable
/// for most applications.
///
/// # Example
/// ```
/// let options = nutmeg::Options::default()
///     .progress_enabled(false); // Don't draw bars, only print.
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Target interval to repaint the progress bar.
    update_interval: Duration,

    /// How long to wait after printing output before drawing the progress bar again.
    print_holdoff: Duration,

    /// Is the progress bar drawn at all?
    progress_enabled: bool,

    /// Use a fake clock for testing.
    fake_clock: bool,
}

impl Options {
    /// Set whether the progress bar will be drawn.
    ///
    /// By default it is drawn, except that this value will be ignored by [View::new] if stdout is not a terminal.
    pub fn progress_enabled(self, progress_enabled: bool) -> Options {
        Options {
            progress_enabled,
            ..self
        }
    }

    /// Set the minimal interval to repaint the progress bar.
    ///
    /// `Duration::ZERO` can be used to cause the bar to repaint on every update.
    pub fn update_interval(self, update_interval: Duration) -> Options {
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
    pub fn print_holdoff(self, print_holdoff: Duration) -> Options {
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
    pub fn fake_clock(self, fake_clock: bool) -> Options {
        Options { fake_clock, ..self }
    }
}

impl Default for Options {
    /// Create default reasonable view options.
    ///
    /// The update interval and print holdoff are 100ms, and the progress bar is enabled.
    fn default() -> Options {
        Options {
            update_interval: Duration::from_millis(100),
            print_holdoff: Duration::from_millis(100),
            progress_enabled: true,
            fake_clock: false,
        }
    }
}

/// Destinations for progress bar output.
enum WriteTo {
    Stdout,
    Stderr,
    Write(Box<dyn Write + Send + 'static>),
}

impl WriteTo {
    fn write_str(&mut self, buf: &str) {
        match self {
            WriteTo::Stdout => print!("{}", buf),
            WriteTo::Stderr => eprint!("{}", buf),
            WriteTo::Write(w) => write!(w, "{}", buf).unwrap(),
        }
    }

    fn write_bytes(&mut self, buf: &[u8]) {
        match self {
            WriteTo::Stdout | WriteTo::Stderr => self.write_str(&String::from_utf8_lossy(buf)),
            WriteTo::Write(w) => w.write_all(buf).unwrap(),
        }
    }

    fn flush(&mut self) {
        match self {
            WriteTo::Stdout => io::stdout().flush().unwrap(),
            WriteTo::Stderr => io::stderr().flush().unwrap(),
            WriteTo::Write(w) => w.flush().unwrap(),
        }
    }
}
