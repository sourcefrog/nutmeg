// Copyright 2022 Martin Pool.

//! Draw ANSI escape sequences.

// References:
// * <https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797>

#![allow(unused)]

use std::borrow::Cow;

pub(crate) const MOVE_TO_START_OF_LINE: &str = "\x1b[1G";

// https://vt100.net/docs/vt510-rm/DECAWM
pub(crate) const DISABLE_LINE_WRAP: &str = "\x1b[?7l";
pub(crate) const ENABLE_LINE_WRAP: &str = "\x1b[?7h";

pub(crate) const CLEAR_TO_END_OF_LINE: &str = "\x1b[0K";
pub(crate) const CLEAR_CURRENT_LINE: &str = "\x1b[2K";
pub(crate) const CLEAR_TO_END_OF_SCREEN: &str = "\x1b[0J";

pub(crate) fn up_n_lines_and_home(n: usize) -> Cow<'static, str> {
    if n > 0 {
        format!("\x1b[{}F", n).into()
    } else {
        MOVE_TO_START_OF_LINE.into()
    }
}

#[cfg(windows)]
pub(crate) fn enable_windows_ansi() -> bool {
    crate::windows::enable_windows_ansi()
}

#[cfg(not(windows))]
pub(crate) fn enable_windows_ansi() -> bool {
    true
}
