//! See how fast we can send view updates.
//!
//! (Run this with `--release` to get a fair estimate.)

use std::time::Instant;

struct IntModel(usize);

impl nutmeg::Model for IntModel {
    fn render(&mut self, _width: usize) -> String {
        format!("count: {}", self.0)
    }
}

fn main() {
    let start = Instant::now();
    let view = nutmeg::View::new(IntModel(0), nutmeg::Options::default());
    let n = 10_000_000;
    for i in 0..n {
        view.update(|IntModel(count)| *count = i);
    }
    view.message(format!(
        "{}ms to send {} updates; average {}ns/update\n",
        start.elapsed().as_millis(),
        n,
        start.elapsed().as_nanos() / n as u128,
    ));
}
