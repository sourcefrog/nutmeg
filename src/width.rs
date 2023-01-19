// Copyright 2022-2023 Martin Pool

//! Measure terminal width.

use terminal_size::Width;
#[cfg(unix)]
pub(crate) fn stdout_width() -> Option<usize> {
    terminal_size::terminal_size_using_fd(1).map(|(Width(w), _)| w as usize)
}

#[cfg(windows)]
pub(crate) fn stdout_width() -> Option<usize> {
    // TODO: We could get the handle for stderr to make this more precise...
    terminal_size::terminal_size().map(|(Width(w), _)| w as usize)
}

#[cfg(unix)]
pub(crate) fn stderr_width() -> Option<usize> {
    terminal_size::terminal_size_using_fd(2).map(|(Width(w), _)| w as usize)
}

#[cfg(windows)]
pub(crate) fn stderr_width() -> Option<usize> {
    // TODO: We could get the handle for stderr to make this more precise...
    terminal_size::terminal_size().map(|(Width(w), _)| w as usize)
}
