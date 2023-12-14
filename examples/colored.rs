//! Example of colored progress bars and output.

use std::io::Write;
use std::thread;
use std::time;

use yansi::Color;
use yansi::Paint;

struct Model {
    i: usize,
}

impl nutmeg::Model for Model {
    fn render(&mut self, _context: &nutmeg::RenderContext) -> String {
        format!("count: {}", Paint::yellow(self.i))
    }
}

fn main() {
    let options = nutmeg::Options::default();
    let mut view = nutmeg::View::new(Model { i: 0 }, options);
    for i in 1..=45 {
        view.update(|state| state.i += 1);
        if i % 5 == 0 {
            writeln!(
                view,
                "{}",
                Paint::new(format!(
                    "{} {}",
                    Paint::new("item").italic(),
                    Paint::new(i).underline()
                ))
                .wrap()
                .fg(Color::White)
                .bold()
                .bg(Color::Blue)
            )
            .unwrap();
        }
        thread::sleep(time::Duration::from_millis(300));
    }
}
