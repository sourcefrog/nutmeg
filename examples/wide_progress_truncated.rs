//! Example showing that progress bar that are too wide for the terminal are 
//! horizontally truncated, even if `State::render` ignores the advised width.

use std::thread::sleep;
use std::time::Duration;

struct State {
    i: usize,
    width: usize,
}

impl nutmeg::State for State {
    fn render<W: std::io::Write>(&self, _width: usize, write_to: &mut W) {
        write!(write_to, "i={} | ", self.i).unwrap();
        let ii = self.i % self.width;
        for _ in 0..ii {
            write_to.write(b"_").unwrap();
        }
        write_to.write("🦀".as_bytes()).unwrap();
        for _ in (ii+1)..self.width {
            write_to.write(b"_").unwrap();
        }
    }
}

fn main() {
    let stdout = std::io::stdout();
    let options = nutmeg::ViewOptions::default();
    let state = State { i: 0, width: 120 };
    let view = nutmeg::View::new(stdout, state, options);
    for _ in 1..=360 {
        view.update(|state| state.i += 1);
        sleep(Duration::from_millis(100));
    }
    // Should show nothing because progress is disabled
}
