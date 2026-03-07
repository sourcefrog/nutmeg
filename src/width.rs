// Copyright 2022-2026 Martin Pool

//! Measure terminal width.

use std::{
    io::{stderr, stdout},
    os::fd::AsFd,
};

use terminal_size::{terminal_size_of, Width};

#[cfg(unix)]
pub(crate) fn stdout_width() -> Option<usize> {
    terminal_size_of(stdout().as_fd()).map(|(Width(w), _)| w as usize)
}

#[cfg(windows)]
pub(crate) fn stdout_width() -> Option<usize> {
    // TODO: We could get the handle for stdout to make this more precise...
    terminal_size::terminal_size().map(|(Width(w), _)| w as usize)
}

#[cfg(unix)]
pub(crate) fn stderr_width() -> Option<usize> {
    terminal_size_of(stderr().as_fd()).map(|(Width(w), _)| w as usize)
}

#[cfg(windows)]
pub(crate) fn stderr_width() -> Option<usize> {
    // TODO: We could get the handle for stderr to make this more precise...
    terminal_size::terminal_size().map(|(Width(w), _)| w as usize)
}
