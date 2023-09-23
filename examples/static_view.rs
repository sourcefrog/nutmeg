//! Demonstrates that you can have a View in a static global variable.
//!
//! This works even when the View is accessed by multiple threads, because
//! it synchronizes internally.

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::thread::{self, sleep};
use std::time::Duration;

// Note: The initial model must also be `const`, so you cannot call `Default::default()`.
// Note: Similarly, you can call `nutmeg::Options::new()` but not `Options::default()`.
static VIEW: nutmeg::View<Model> = nutmeg::View::new(
    Model {
        i: AtomicUsize::new(0),
    },
    nutmeg::Options::new(),
);

#[derive(Default)]
struct Model {
    i: AtomicUsize,
}

impl nutmeg::Model for Model {
    fn render(&mut self, _width: usize) -> String {
        format!("i={}", self.i.load(Relaxed))
    }
}

fn main() -> std::io::Result<()> {
    thread::scope(|scope| {
        for tid in 0..3 {
            scope.spawn(move || {
                VIEW.message(format!("thread {} starting\n", tid));
                for _i in 0..20 {
                    VIEW.update(|model| model.i.fetch_add(1, Relaxed));
                    sleep(Duration::from_millis(rand::random::<u64>() % 200));
                }
                VIEW.message(format!("thread {} done\n", tid));
            });
        }
    });
    Ok(())
}
