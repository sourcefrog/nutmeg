// Copyright 2022 Martin Pool

//! General-purpose models for Nutmeg.

use std::borrow::Cow;

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
    /// Typically this should be called from a callback passed to [nutmeg::Model::update].
    pub fn set_suffix<S>(&mut self, suffix: S)
    where
        S: Into<Cow<'static, str>>,
    {
        self.suffix = suffix.into();
    }
}

impl crate::Model for StringPair {
    fn render(&mut self, _width: usize) -> String {
        format!("{}{}", self.prefix, self.suffix)
    }
}
