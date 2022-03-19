// Copyright 2022 Martin Pool

//! Generally reusable models for Nutmeg.
//!
//! These are provided because they may be easy to use for many applications
//! that do not (yet) want to customize the progress display. There is no
//! requirement to use them: they only implement the public [Model] interface.

use std::borrow::Cow;
use std::time::Instant;

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
