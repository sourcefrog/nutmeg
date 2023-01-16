# Nutmeg Changelog

## Unreleased

* New: [View::hide] temporarily hides the view, but allows it to pop back when
  the model is next updated.

* New: Better support for constructing a const or static [Options]: there's a new
  `const fn` [Options::new] constructor and the functions to set fields are also
  `const`.


## 0.1.2

Released 2022-07-27

* API change: Removed `View::new_stderr` and `View::write_to`. Instead, the view
  can be drawn on stderr or output can be captured using [Options::destination].
  This is better aligned with the idea that programs might have a central function
  that constructs a [Options], as they will probably want to consistently
  write to either stdout or stderr.

* New: Output can be captured for inspection in tests using [Options::destination],
  [Destination::Capture], and [View::captured_output].

* Improved: Nutmeg avoids redrawing if the model renders identical output to what
  is already displayed, to avoid flicker.

## 0.1.1

Released 2022-03-22

* API change: [View::message] takes the message as an `AsRef<str>`, meaning
  it may be either a `&str` or `String`. This makes the common case where
  the message is the result of `format!` a little easier.

## 0.1.0

Released 2022-03-22

* API change: The "Write" type representing the destination is no longer
  part of the visible public signature of [View], to hide complexity and
  since it is not helpful to most callers.

* API change: Renamed `View::to_stderr` to `View::new_stderr`.

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

* New: [models::BasicModel] allows simple cases to supply both an initial value
  and a render function inline in the [View] constructor call, avoiding any
  need to define a [Model] struct.

* New: [View::inspect_model] gives its callback a `&mut` to the model.

* New: Progress bars constructed by [View::new] and `View::new_stderr` are disabled when
  `$TERM=dumb`.

## 0.0.2

Released 2022-03-07

* API change: Renamed `nutmeg::ViewOptions` to just `nutmeg::Options`.

* Fixed: A bug that caused leftover text when multi-line bars shrink in width.

* Fixed: The output from bars created with [View::new] and `View::to_stderr` in
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

