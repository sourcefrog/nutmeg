// Copyright 2022 Martin Pool.

//! Draw ANSI escape sequences.

//  ansi::MoveToStartOfLine, ansi::DisableLineWrap, rendered, ansi::ClearToEndOfLine)?;
pub(crate) const MOVE_TO_START_OF_LINE: &str = "\x1b[1G";

// https://vt100.net/docs/vt510-rm/DECAWM
pub(crate) const DISABLE_LINE_WRAP: &str = "\x1b[?7l";
pub(crate) const ENABLE_LINE_WRAP: &str = "\x1b[?7h";

pub(crate) const CLEAR_TO_END_OF_LINE: &str = "\x1b[0K";
pub(crate) const CLEAR_CURRENT_LINE: &str = "\x1b[2K";

#[cfg(windows)]
pub(crate) fn enable_windows_ansi() -> bool {
    windows::enable_windows_ansi()
}

#[cfg(not(windows))]
pub(crate) fn enable_windows_ansi() -> bool {
    true
}