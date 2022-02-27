//! Draw to stderr. Try this with stdout redirected to a file.

use std::thread::sleep;
use std::time::{Duration, Instant};

struct Model {
    i: usize,
    start_time: Instant,
}

impl nutmeg::Model for Model {
    fn render(&mut self, width: usize) -> String {
        let start = format!("i={} | ", self.i);
        let end = format!(" | {:.3}s", self.start_time.elapsed().as_secs_f64());
        let fill_len = width - start.len() - end.len();
        let mut fill: Vec<u8> = vec![b'.'; fill_len];
        fill[self.i % fill_len] = b'~';
        let fill: String = String::from_utf8(fill).unwrap();
        format!("{start}{fill}{end}")
    }
}

fn main() {
    let options = nutmeg::ViewOptions::default();
    let state = Model {
        i: 0,
        start_time: Instant::now(),
    };
    let view = nutmeg::View::to_stderr(state, options);
    for _ in 1..=120 {
        view.update(|state| state.i += 1);
        sleep(Duration::from_millis(100));
    }
}
