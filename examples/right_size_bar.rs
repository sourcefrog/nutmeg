//! The render function is passed the terminal width and can use it to make things
//! fit nicely.

use std::thread::sleep;
use std::time::{Duration, Instant};

struct State {
    i: usize,
    start_time: Instant,
}

impl nutmeg::Model for State {
    fn render<W: std::io::Write>(&self, width: usize, w: &mut W) {
        let start = format!("i={} | ", self.i);
        let end = format!(" | {:.3}s", self.start_time.elapsed().as_secs_f64());
        let fill_len = width - start.len() - end.len();
        let mut fill: Vec<u8> = vec![b'.'; fill_len];
        fill[self.i % fill_len] = b'~';
        let fill: String = String::from_utf8(fill).unwrap();
        write!(w, "{start}{fill}{end}").unwrap();
    }
}

fn main() {
    let stdout = std::io::stdout();
    let options = nutmeg::ViewOptions::default();
    let state = State {
        i: 0,
        start_time: Instant::now(),
    };
    let view = nutmeg::View::new(stdout, state, options);
    for _ in 1..=120 {
        view.update(|state| state.i += 1);
        sleep(Duration::from_millis(100));
    }
    // Should show nothing because progress is disabled
}
