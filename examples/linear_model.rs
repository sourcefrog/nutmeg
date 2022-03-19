// Copyright 2022 Martin Pool

pub fn main() {
    let total = 99;
    let progress = nutmeg::View::new(
        nutmeg::models::LinearModel::new("Counting raindrops", total),
        nutmeg::Options::default(),
    );
    for _i in 0..=total {
        progress.update(|model| model.increment(1));
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
