//! Progress bar updates are delayed after printing.
//!
//! This example sets a very long print_holdoff to make
//! the effect more noticeable. After each message, the updates
//! to the model do take effect but they're not drawn to the
//! screen.

use std::io;
use std::io::Write;
use std::thread;
use std::time;
use std::time::Duration;

fn main() -> io::Result<()> {
    let options = nutmeg::Options::default()
        .print_holdoff(Duration::from_millis(1000))
        .update_interval(Duration::from_millis(0));
    let mut view = nutmeg::View::new(0usize, options);
    for _i in 0..5 {
        for j in 0..4 {
            writeln!(view, "message {j}")?;
            thread::sleep(time::Duration::from_millis(100));
        }
        for j in 0..20 {
            view.update(|state| {
                // Previous updates were applied even though
                // they may not have been painted.
                assert!(j == 0 || *state == (j - 1));
                *state = j
            });
            thread::sleep(time::Duration::from_millis(100));
        }
    }
    Ok(())
}
