// Copyright 2022 Martin Pool.

//! Rust's test framework captures output sent through `println!` but not output
//! sent through opening `stdout` or `stderr`.
//!
//! This module implements a bit of a hack to get default progress output
//! captured, by redirecting a `Write` into `print!`.
//! 
//! For context on this weird workaround see
//! <https://github.com/rust-lang/rust/issues/31343>.

use std::io;
use std::str;

/// Routes output from `Write` to `print!` so that it will be captured in Rust
/// unit tests and otherwise go to stdout.
///
/// (Unit tests currently don't capture file handles opened by
/// [std::io::stdout].)
#[non_exhaustive]
pub struct WriteToPrint {}

impl io::Write for WriteToPrint {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        print!("{}", str::from_utf8(buf).unwrap());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }
}

/// Routes output from `Write` to `eprint!` so that it will be captured in Rust
/// unit tests and otherwise go to stderr.
#[non_exhaustive]
pub struct WriteToStderr {}

impl io::Write for WriteToStderr {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        eprint!("{}", str::from_utf8(buf).unwrap());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // probably unnecessary
        io::stderr().flush()
    }
}
