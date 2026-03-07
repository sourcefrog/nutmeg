use std::{
    io::{stderr, stdout},
    os::windows::io::AsHandle,
};

use terminal_size::{terminal_size_of, Width};

pub(crate) fn stdout_width() -> Option<usize> {
    terminal_size_of(stdout().as_handle()).map(|(Width(w), _)| w as usize)
}

pub(crate) fn stderr_width() -> Option<usize> {
    terminal_size_of(stderr().as_handle()).map(|(Width(w), _)| w as usize)
}
