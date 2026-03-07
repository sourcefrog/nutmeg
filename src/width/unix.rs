use std::{
    io::{stderr, stdout},
    os::fd::AsFd,
};

use terminal_size::{terminal_size_of, Width};

pub(crate) fn stdout_width() -> Option<usize> {
    terminal_size_of(stdout().as_fd()).map(|(Width(w), _)| w as usize)
}

pub(crate) fn stderr_width() -> Option<usize> {
    terminal_size_of(stderr().as_fd()).map(|(Width(w), _)| w as usize)
}
