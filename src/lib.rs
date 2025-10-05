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

1. One of the provided [models].
2. An application-defined struct (or enum or other type) that implements [Model].

The model is responsible for rendering itself into a String, optionally with ANSI styling,
by implementing [Model::render].  Applications might
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
        // 5. Interleave text output lines by writing messages to the view.
        if i % 10 == 3 {
            view.message(format!("reached {}", i));
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

Each call to [View::update] will take a mutex lock and check the
system time, in addition to running the callback and some function-call overhead.

The model is only rendered to a string, and the string printed to a terminal, if
sufficient time has passed since it was last painted.

The `examples/bench.rs` sends updates as fast as possible to a model containing a
single `u64`, from a single thread. As of 2022-03-22, on a 2019 Core i9 Macbook Pro,
it takes about 500ms to send 10e6 updates, or 50ns/update.

# Integration with `tracing`

Nutmeg can be used to draw progress bars in a terminal interleaved with
[tracing](https://docs.rs/tracing/) messages. The progress bar is automatically
temporarily removed to show messages, and repainted after the next update,
subject to rate limiting and the holdoff time configured in [Options].

`Arc<View<M>>` implicitly implements [`tracing_subscriber::fmt::writer::MakeWriter`](https://docs.rs/tracing-subscriber/0.3.17/tracing_subscriber/fmt/writer/trait.MakeWriter.html)
and so can be passed to `tracing_subscriber::fmt::layer().with_writer()`.

For example:

```rust
    use std::sync::Arc;
    use tracing::Level;
    use tracing_subscriber::prelude::*;

    struct Model { count: usize }
    impl nutmeg::Model for Model {
         fn render(&mut self, _width: usize) -> String { todo!() }
    }

    let model = Model {
        count: 0,
    };
    let view = Arc::new(nutmeg::View::new(model, nutmeg::Options::new()));
    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_writer(Arc::clone(&view))
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::uptime())
        .with_filter(tracing_subscriber::filter::LevelFilter::from_level(
            Level::INFO,
        ));
    tracing_subscriber::registry().with(layer).init();

    for i in 0..10 {
        if i % 10 == 0 {
            tracing::info!(i, "cats adored");
        }
        view.update(|m| m.count += 1);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
```

See `examples/tracing` for a runnable example.

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

use std::io::{self, Write};
use std::mem::take;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::time::Instant;

mod ansi;
mod destination;
mod helpers;
pub mod models;
mod options;
mod width;
#[cfg(windows)]
mod windows;

pub mod _changelog {
    #![doc = include_str!("../NEWS.md")]
    #[allow(unused_imports)]
    use super::*; // so that hyperlinks work
}

use crate::ansi::insert_codes;
pub use crate::destination::Destination;
pub use crate::helpers::*;
pub use crate::options::Options;

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
///
/// ## Static views
///
/// Views can be constructed as static variables, and used from multiple threads.
///
/// Note that `Default::default()` is not `const` so cannot be used to construct
/// either your model or the `Options`.
///
/// For example:
/// ```
/// static VIEW: nutmeg::View<Model> = nutmeg::View::new(Model { i: 0 }, nutmeg::Options::new());
///
/// struct Model {
///     i: usize,
/// }
///
/// impl nutmeg::Model for Model {
///     fn render(&mut self, _width: usize) -> String {
///         format!("i={}", self.i)
///     }
/// }
///
/// fn main() -> std::io::Result<()> {
///     for i in 0..20 {
///         VIEW.update(|model| model.i = i);
///         if i % 5 == 0 {
///             // Note: You cannot use writeln!() here, because its argument must be
///             // `&mut`, but you can send messages.
///             VIEW.message(&format!("message: i={i}\n"));
///         }
///         std::thread::sleep(std::time::Duration::from_millis(20));
///     }
///     Ok(())
/// }
///
/// ```
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
    pub const fn new(model: M, options: Options) -> View<M> {
        View {
            inner: Mutex::new(Some(InnerView::new(model, options))),
        }
    }

    /// Call this function on the locked inner view.
    ///
    /// If the view has been destroyed, do nothing.
    fn call_inner<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut InnerView<M>) -> R,
    {
        f(self
            .inner
            .lock()
            .expect("View mutex is not poisoned")
            .as_mut()
            .expect("View is not already destroyed"))
    }

    /// Extract the inner view, destroying this object: updates on it will
    /// no longer succeed.
    fn take_inner(self) -> InnerView<M> {
        self.inner
            .lock()
            .expect("View mutex is not poisoned")
            .take()
            .expect("View is not already destroyed")
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
        self.take_inner().abandon().expect("Abandoned view")
    }

    /// Erase the model from the screen (if drawn), destroy it, and return the model.
    pub fn finish(self) -> M {
        self.take_inner().finish()
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
        self.call_inner(|inner| inner.update(update_fn))
    }

    /// Hide the progress bar if it's currently drawn, and leave it
    /// hidden until [View::resume] is called.
    pub fn suspend(&self) {
        self.call_inner(|v| v.suspend().expect("suspend succeeds"))
    }

    /// Remove the progress bar if it's currently drawn, but allow it
    /// to be redrawn when the model is next updated.
    pub fn clear(&self) {
        self.call_inner(|v| v.clear().expect("clear succeeds"))
    }

    /// Allow the progress bar to be drawn again, reversing the effect
    /// of [View::suspend].
    pub fn resume(&self) {
        self.call_inner(|v| v.resume().expect("resume succeeds"))
    }

    /// Set the value of the fake clock, for testing.
    ///
    /// Panics if [Options::fake_clock] was not previously set.
    ///
    /// Moving the clock backwards in time may cause a panic.
    pub fn set_fake_clock(&self, fake_clock: Instant) {
        self.call_inner(|v| v.set_fake_clock(fake_clock))
    }

    /// Inspect the view's model.
    ///
    /// The function `f` is applied to the model, and then the result
    /// of `f` is returned by `inspect_model`.
    ///
    /// ```
    /// use nutmeg::{Options, View};
    /// use nutmeg::models::LinearModel;
    ///
    /// let mut model = LinearModel::new("Things done", 100);
    /// model.set_done(10);
    /// let view = View::new(model, Options::default());
    /// view.update(|model| model.increment(3));
    /// assert_eq!(view.inspect_model(|m| m.done()), 13);
    /// ```
    pub fn inspect_model<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut M) -> R,
    {
        self.call_inner(|v| f(&mut v.model))
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
    /// use nutmeg::models::LinearModel;
    ///
    /// let view = View::new(LinearModel::new("Splines reticulated", 100), Options::default());
    /// for i in 0..20 {
    ///     view.update(|model| model.increment(1));
    ///     if i == 12 {
    ///         view.message("Some quality splines here!\n");
    ///     }
    /// }
    /// ```
    pub fn message<S: AsRef<str>>(&self, message: S) {
        self.message_bytes(message.as_ref().as_bytes())
    }

    /// Print a message from a byte buffer.
    ///
    /// This is the same as [View::message] but takes an `AsRef<[u8]>`, such as a slice.
    ///
    /// Most destinations will expect the contents to be UTF-8.
    ///
    /// ```
    /// use nutmeg::{Options, View};
    /// use nutmeg::models::LinearModel;
    ///
    /// let view = View::new(LinearModel::new("Things done", 100), Options::default());
    /// view.message_bytes(b"hello crow\n");
    /// ```
    pub fn message_bytes<S: AsRef<[u8]>>(&self, message: S) {
        self.call_inner(|v| v.write(message.as_ref()).expect("write message"));
    }

    /// If the view's destination is [Destination::Capture], returns the buffer
    /// of captured output.
    ///
    /// Panics if the destination is not [Destination::Capture].
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
    /// use nutmeg::models::DisplayModel;
    ///
    /// let view = View::new(
    ///     DisplayModel("unchanging message"),
    ///     Options::default().destination(Destination::Capture));
    /// let output = view.captured_output();
    /// view.message("Captured message\n");
    /// drop(view);
    /// assert_eq!(output.lock().unwrap().as_str(), "Captured message\n");
    /// ```
    pub fn captured_output(&self) -> Arc<Mutex<String>> {
        self.call_inner(|v| v.captured_output())
    }

    /// Return a copy of the captured output, if any, and clear the captured output buffer.
    ///
    /// This is intended for use in testing, so that tests can incrementally check
    /// the output.
    ///
    /// # Panics
    ///
    /// This function panics if output capture is not enabled.
    pub fn take_captured_output(&self) -> String {
        self.call_inner(|v| v.take_captured_output())
    }
}

impl<M: Model> io::Write for &View<M> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.call_inner(|v| v.write(buf))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<M: Model> io::Write for View<M> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.call_inner(|v| v.write(buf))
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
        if let Ok(mut inner_guard) = self.inner.try_lock() {
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

    /// True if the progress bar is suspended, and should not be drawn.
    suspended: bool,

    /// Whether the progress bar is drawn, etc.
    state: State,

    options: Options,

    /// The current time on the fake clock, if it is enabled.
    fake_clock: Option<Instant>,

    /// Captured output, if active.
    capture_buffer: Option<Arc<Mutex<String>>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum State {
    /// Nothing has ever been painted, and the screen has not yet been initialized.
    New,
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
    const fn new(model: M, options: Options) -> InnerView<M> {
        InnerView {
            capture_buffer: None,
            fake_clock: None,
            model,
            options,
            state: State::New,
            suspended: false,
        }
    }

    fn finish(mut self) -> M {
        let _ = self.clear();
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
            State::New | State::IncompleteLine | State::None | State::Printed { .. } => (),
        }
        self.state = State::None; // so that drop does not attempt to erase
        Ok(self.model)
    }

    /// Return the real or fake clock.
    fn clock(&self) -> Instant {
        self.fake_clock.unwrap_or_else(Instant::now)
    }

    fn init_destination(&mut self) {
        if self.state == State::New {
            if self.options.destination.initalize().is_err() {
                // This destination doesn't want to draw progress bars, so stay off forever.
                self.options.progress_enabled = false;
            }
            self.state = State::None;
        }
    }

    fn paint_progress(&mut self) -> io::Result<()> {
        self.init_destination();
        if !self.options.progress_enabled || self.suspended {
            return Ok(());
        }
        let now = self.clock();
        match self.state {
            State::IncompleteLine => return Ok(()),
            State::New | State::None => (),
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
        if let Some(width) = self.options.destination.width() {
            let mut rendered = self.model.render(width);
            if rendered.ends_with('\n') {
                // Handle models that incorrectly add a trailing newline, rather than
                // leaving a blank line. (Maybe we should just let them fix it, and
                // be simpler?)
                rendered.pop();
            }
            let cursor_y = match self.state {
                State::ProgressDrawn {
                    ref last_drawn_string,
                    ..
                } if *last_drawn_string == rendered => {
                    return Ok(());
                }
                State::ProgressDrawn { cursor_y, .. } => Some(cursor_y),
                _ => None,
            };
            let (buf, cursor_y) = insert_codes(&rendered, cursor_y);
            self.write_output(&buf);
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
        self.clear()
    }

    fn resume(&mut self) -> io::Result<()> {
        self.suspended = false;
        self.paint_progress()
    }

    /// Clear the progress bars off the screen, leaving it ready to
    /// print other output.
    fn clear(&mut self) -> io::Result<()> {
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
            State::None | State::New | State::IncompleteLine | State::Printed { .. } => {}
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
        self.init_destination();
        self.clear()?;
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
        assert!(self.options.fake_clock, "Options.fake_clock is not enabled");
        self.fake_clock = Some(fake_clock);
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
                    .get_or_insert_with(|| Arc::new(Mutex::new(String::new())))
                    .lock()
                    .expect("lock capture_buffer")
                    .push_str(buf);
            }
        }
    }

    fn captured_output(&mut self) -> Arc<Mutex<String>> {
        self.capture_buffer
            .get_or_insert_with(|| Arc::new(Mutex::new(String::new())))
            .clone()
    }

    fn take_captured_output(&mut self) -> String {
        take(
            self.capture_buffer
                .as_mut()
                .expect("output capture is not enabled")
                .lock()
                .expect("lock capture_buffer")
                .deref_mut(),
        )
    }
}
