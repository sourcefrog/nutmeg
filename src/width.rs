// Copyright 2022 Martin Pool

//! Measure terminal width etc.

use terminal_size::Width;

/// How to determine the terminal width.
pub(crate) enum WidthStrategy {
    Fixed(usize),
    Stdout,
    Stderr,
}

impl WidthStrategy {
    pub(crate) fn width(&self) -> Option<usize> {
        match self {
            WidthStrategy::Fixed(width) => Some(*width),
            WidthStrategy::Stdout => stdout_width(),
            WidthStrategy::Stderr => stderr_width(),
        }
    }
}

#[cfg(unix)]
fn stdout_width() -> Option<usize> {
    terminal_size::terminal_size_using_fd(1).map(|(Width(w), _)| w as usize)
}

#[cfg(windows)]
fn stdout_width() -> Option<usize> {
    // TODO: We could get the handle for stderr to make this more precise...
    terminal_size::terminal_size().map(|(Width(w), _)| w as usize)
}

#[cfg(unix)]
fn stderr_width() -> Option<usize> {
    terminal_size::terminal_size_using_fd(2).map(|(Width(w), _)| w as usize)
}

#[cfg(windows)]
fn stderr_width() -> Option<usize> {
    // TODO: We could get the handle for stderr to make this more precise...
    terminal_size::terminal_size().map(|(Width(w), _)| w as usize)
}
