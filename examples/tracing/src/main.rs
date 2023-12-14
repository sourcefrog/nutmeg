//! Show integration of Nutmeg and tracing: tracing console messages
//! are interspersed nicely with the progress bar.

use std::sync::Arc;
use std::time::Instant;

use nutmeg::View;
use tracing::Level;
use tracing_subscriber::prelude::*;

fn main() {
    let model = Model {
        count: 0,
        start: Instant::now(),
    };
    let view = Arc::new(View::new(model, nutmeg::Options::new()));
    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_writer(Arc::clone(&view))
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::uptime())
        .with_filter(tracing_subscriber::filter::LevelFilter::from_level(
            Level::INFO,
        ));
    tracing_subscriber::registry().with(layer).init();

    for i in 0..100 {
        if i % 10 == 0 {
            tracing::info!("count: {}", i);
        } else if i % 37 == 0 {
            tracing::warn!(i, "spooky!");
        }
        view.update(|m| m.count += 1);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

struct Model {
    count: usize,
    start: Instant,
}

impl nutmeg::Model for Model {
    fn render(&mut self, _context: &nutmeg::RenderContext) -> String {
        let spin = match self.count % 4 {
            0 => "|",
            1 => "/",
            2 => "-",
            3 => "\\",
            _ => unreachable!(),
        };
        format!(
            "{spin} count: {}, elapsed: {:.1}s",
            self.count,
            self.start.elapsed().as_secs_f64()
        )
    }
}
