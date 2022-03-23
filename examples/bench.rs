//! See how fast we can send view updates.
//!
//! (Run this with `--release` to get a fair estimate.)

use std::time::Instant;

fn main() {
    let start = Instant::now();
    let view = nutmeg::View::new(0u64, nutmeg::Options::default());
    let n = 10_000_000;
    for i in 0..n {
        view.update(|model| *model = i);
    }
    view.message(format!(
        "{}ms to send {} updates; average {}ns/update",
        start.elapsed().as_millis(),
        n,
        start.elapsed().as_nanos() / n as u128,
    ));
}
