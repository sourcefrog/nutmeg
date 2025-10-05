// Copyright 2022-2023 Martin Pool.

use std::env;
use std::result::Result;

#[allow(unused)] // for docstrings
use crate::View;
use crate::{ansi, width};

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
    /// Determine if this destination is possible, and, if necessary, enable Windows ANSI support.
    pub(crate) fn initalize(&self) -> Result<(), ()> {
        if match self {
            Destination::Stdout => {
                atty::is(atty::Stream::Stdout) && !is_dumb_term() && ansi::enable_windows_ansi()
            }
            Destination::Stderr => {
                atty::is(atty::Stream::Stderr) && !is_dumb_term() && ansi::enable_windows_ansi()
            }
            Destination::Capture => true,
        } {
            Ok(())
        } else {
            Err(())
        }
    }

    pub(crate) fn width(&self) -> Option<usize> {
        match self {
            Destination::Stdout => width::stdout_width(),
            Destination::Stderr => width::stderr_width(),
            Destination::Capture => Some(80),
        }
    }
}

fn is_dumb_term() -> bool {
    env::var("TERM").is_ok_and(|s| s.eq_ignore_ascii_case("dumb"))
}
