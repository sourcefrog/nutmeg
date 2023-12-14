// Copyright 2022 Martin Pool.

//! Demonstrate multiple threads writing to a single view.
//!
//! A single View is shared in an Arc across all threads. (A scoped thread
//! would also work.)
//!
//! Each thread periodically updates the model, which will make it repaint
//! subject to the update rate limit.

use std::fmt::Write;
use std::sync::Arc;
use std::thread::{self, sleep};
use std::time::Duration;

use rand::Rng;

const THREAD_WORK_MAX: usize = 20;

/// Per-thread progress.
struct JobState {
    x: usize,
    complete: bool,
}

/// Overall task progress.
struct Model {
    job_state: Vec<JobState>,
}

impl nutmeg::Model for Model {
    fn render(&mut self, _context: &nutmeg::RenderContext) -> String {
        let mut s = String::new();
        let n_jobs = self.job_state.len();
        let n_complete = self.job_state.iter().filter(|j| j.complete).count();
        writeln!(s, "{n_complete}/{n_jobs} complete").unwrap();
        for (i, js) in self.job_state.iter().enumerate() {
            let remains = THREAD_WORK_MAX - js.x;
            writeln!(s, "{:3}: {}{}", i, "#".repeat(js.x), "_".repeat(remains)).unwrap();
        }
        s
    }
}

fn work(i_thread: usize, arc_view: Arc<nutmeg::View<Model>>) {
    let mut rng = rand::thread_rng();
    for j in 0..=THREAD_WORK_MAX {
        arc_view.update(|model| model.job_state[i_thread].x = j);
        sleep(Duration::from_millis(rng.gen_range(100..600)));
    }
    arc_view.update(|model| model.job_state[i_thread].complete = true);
}

fn main() {
    let model = Model {
        job_state: Vec::new(),
    };
    let view = nutmeg::View::new(model, nutmeg::Options::default());
    view.update(|_m| ());
    let arc_view = Arc::new(view);
    let mut join_handles = Vec::new();
    for i_thread in 0..20 {
        arc_view.update(|model| {
            model.job_state.push(JobState {
                x: 0,
                complete: false,
            })
        });
        sleep(Duration::from_millis(100));
        let give_arc_view = arc_view.clone();
        join_handles.push(thread::spawn(move || work(i_thread, give_arc_view)));
    }
    for join_handle in join_handles {
        arc_view.update(|_m| ());
        join_handle.join().unwrap();
    }
    arc_view.update(|_| ());
    sleep(Duration::from_millis(500));
}
