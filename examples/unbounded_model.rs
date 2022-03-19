// Copyright 2022 Martin Pool

pub fn main() {
    let progress = nutmeg::View::new(
        nutmeg::models::UnboundedModel::new("Counting raindrops"),
        nutmeg::Options::default(),
    );
    for _i in 0..=99 {
        progress.update(|model| model.increment(1));
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
