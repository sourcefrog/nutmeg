// Copyright 2022-2026 Martin Pool

//! Measure terminal width.

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use unix::{stderr_width, stdout_width};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::{stderr_width, stdout_width};
