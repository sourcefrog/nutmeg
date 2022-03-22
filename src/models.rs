// Copyright 2022 Martin Pool

//! Generally reusable models for Nutmeg.
//!
//! These are provided because they may be easy to use for many applications
//! that do not (yet) want to customize the progress display. There is no
//! requirement to use them: they only implement the public [Model] interface.

use std::borrow::Cow;
use std::time::{Duration, Instant};

#[allow(unused)] // For docstrings
use crate::View;
use crate::{estimate_remaining, percent_done, Model};

/// A Nutmeg progress model that concatenates a pair of strings to render
/// the progress bar.
///
/// For example, the prefix could be a description of the operation, and the
/// suffix could be the name of the file or object that's being processed.
pub struct StringPair {
    prefix: Cow<'static, str>,
    suffix: Cow<'static, str>,
}

impl StringPair {
    /// Construct a new StringPair model, providing initial values for the
    /// two strings.
    ///
    /// ```
    /// let progress_bar = nutmeg::View::new(
    ///     nutmeg::models::StringPair::new("Copying: ",""),
    ///     nutmeg::Options::default(),
    /// );
    /// // ...
    /// progress_bar.update(|model| model.set_suffix("/etc/hostname"));
    /// ```
    pub fn new<S1, S2>(prefix: S1, suffix: S2) -> StringPair
    where
        S1: Into<Cow<'static, str>>,
        S2: Into<Cow<'static, str>>,
    {
        StringPair {
            prefix: prefix.into(),
            suffix: suffix.into(),
        }
    }

    /// Update the second string.
    ///
    /// Typically this should be called from a callback passed to [View::update].
    pub fn set_suffix<S>(&mut self, suffix: S)
    where
        S: Into<Cow<'static, str>>,
    {
        self.suffix = suffix.into();
    }
}

impl Model for StringPair {
    fn render(&mut self, _width: usize) -> String {
        format!("{}{}", self.prefix, self.suffix)
    }
}

/// A model for completion of a number of approximately equal-sized tasks,
/// with a percentage completion and extrapolated time to completion.
///
/// The rendered result looks like this:
///
/// ```text
/// Counting raindrops: 68/99, 68.7%, 3 sec remaining
/// ```
///
/// /// Run `cargo run --examples linear_model` in the Nutmeg source tree to see this in action.
///
/// # Example
///
/// ```
/// let total = 99;
/// let progress = nutmeg::View::new(
///     nutmeg::models::LinearModel::new("Counting raindrops", total),
///     nutmeg::Options::default(),
/// );
/// for i in 1..=total {
///     progress.update(|model| model.increment(1));
/// }
/// ```
pub struct LinearModel {
    done: usize,
    total: usize,
    message: Cow<'static, str>,
    start: Instant,
}

impl LinearModel {
    /// Construct a new model with a prefix string and number of total work items.
    pub fn new<S: Into<Cow<'static, str>>>(message: S, total: usize) -> LinearModel {
        LinearModel {
            done: 0,
            total,
            message: message.into(),
            start: Instant::now(),
        }
    }

    /// Update the total amount of expected work.
    pub fn set_total(&mut self, total: usize) {
        self.total = total
    }

    /// Update the amount of work done.
    ///
    /// This should normally be called from a callback passed to [View::update].
    pub fn set_done(&mut self, done: usize) {
        self.done = done
    }

    /// Update the amount of work done by an increment (typically 1).
    ///
    /// This should normally be called from a callback passed to [View::update].
    ///
    pub fn increment(&mut self, i: usize) {
        self.done += i
    }
}

impl Model for LinearModel {
    fn render(&mut self, _width: usize) -> String {
        format!(
            "{}: {}/{}, {}, {} remaining",
            self.message,
            self.done,
            self.total,
            percent_done(self.done, self.total),
            estimate_remaining(&self.start, self.done, self.total)
        )
    }
}

/// A model that counts up the amount of work done, with no known total, showing the elapsed time.
///
/// Run `cargo run --examples unbounded_model` in the Nutmeg source tree to see this in action.
///
/// # Example
/// ```
/// let progress = nutmeg::View::new(
///     nutmeg::models::UnboundedModel::new("Counting raindrops"),
///     nutmeg::Options::default(),
/// );
/// for _i in 0..=99 {
///     progress.update(|model| model.increment(1));
/// }
/// ```
pub struct UnboundedModel {
    message: Cow<'static, str>,
    done: usize,
    start: Instant,
}

impl UnboundedModel {
    /// Construct a model with a message describing the type of work being done.
    pub fn new<S: Into<Cow<'static, str>>>(message: S) -> UnboundedModel {
        UnboundedModel {
            done: 0,
            message: message.into(),
            start: Instant::now(),
        }
    }

    /// Update the amount of work done.
    ///
    /// This should normally be called from a callback passed to [View::update].
    pub fn set_done(&mut self, done: usize) {
        self.done = done
    }

    /// Update the amount of work done by an increment (typically 1).
    ///
    /// This should normally be called from a callback passed to [View::update].
    ///
    pub fn increment(&mut self, i: usize) {
        self.done += i
    }
}

impl Model for UnboundedModel {
    fn render(&mut self, _width: usize) -> String {
        format!(
            "{}: {} in {}",
            self.message,
            self.done,
            format_duration(self.start.elapsed())
        )
    }
}

fn format_duration(d: Duration) -> String {
    let elapsed_secs = d.as_secs();
    if elapsed_secs >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            elapsed_secs / 3600,
            (elapsed_secs / 60) % 60,
            elapsed_secs % 60
        )
    } else {
        format!("{}:{:02}", (elapsed_secs / 60) % 60, elapsed_secs % 60)
    }
}

/// A model that stores any user-provided type, and renders by calling a function
/// provided in the constructor.
///
/// For many simple cases this avoids any need to explicitly declare a model
/// class: instead the [View::new] call can, in-line, construct a BasicView
/// giving an initial value and a render function.
///
/// # Example
/// ```
/// let view = nutmeg::View::new(
///     nutmeg::models::BasicModel::new((0, 10), |(a, b)| format!("{}/{} complete", a, b)),
///     nutmeg::Options::default(),
/// );
/// for _i in 0..10 {
///     view.update(|model| model.value.0 += 1);
///     // ...
/// }
/// ```
pub struct BasicModel<T, R>
where
    R: FnMut(&mut T) -> String,
{
    /// The current inner value of the model.
    ///
    /// The type `T` and initial value are set by the first parameter to
    /// [BasicModel::new].
    ///
    /// The functions passed to [View::update] take a `model` as a parameter
    /// and should typically act on `model.value`.
    pub value: T,
    render_fn: R,
}

impl<T, R> BasicModel<T, R>
where
    R: FnMut(&mut T) -> String,
{
    /// Construct a new BasicModel.
    ///
    /// `value` is the initial inner value of the model. It may be any type
    /// but might typically be an integer, a string, or a tuple of simple
    /// values.
    ///
    /// `render_fn` takes an `&mut T` and renders it to a string to be
    /// drawn in the progress bar.
    pub fn new(value: T, render_fn: R) -> BasicModel<T, R> {
        BasicModel { value, render_fn }
    }
}

impl<T, R> Model for BasicModel<T, R>
where
    R: FnMut(&mut T) -> String,
{
    fn render(&mut self, _width: usize) -> String {
        (self.render_fn)(&mut self.value)
    }
}
