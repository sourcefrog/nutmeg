// Copyright 2022 Martin Pool.

//! Rust's test framework captures output sent through `println!` but
//! not output sent through opening `stdout` or `stderr`.
//!
//! This module implements a bit of a hack to get default progress
//! output captured, by redirecting a `Write` into `print!`.

use std::fmt;
use std::io;
use std::str;

/// Routes output from `Write` to `print!` so that it will be captured in Rust
/// unit tests and otherwise go to stdout.
///
/// (Unit tests currently don't capture file handles opened by
/// [std::io::stdout].)
#[non_exhaustive]
pub struct WriteToPrint {}

impl fmt::Write for WriteToPrint {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        print!("{}", s);
        Ok(())
    }
}

impl io::Write for WriteToPrint {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        print!("{}", str::from_utf8(buf).unwrap());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
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
        Ok(())
    }
}
