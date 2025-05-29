use criterion::{Criterion, criterion_group, criterion_main};
use lib::{cfg::AnalysisConfig, state::AnalysisState};
use std::{
    fs,
    time::{Duration, Instant},
};

pub fn analysis(c: &mut Criterion) {
    c.bench_function("AnalysisState::from_prev", |b| {
        let cfg = fs::read_to_string("config.toml")
            .ok()
            .and_then(|s| toml::from_str::<AnalysisConfig>(&s).ok())
            .unwrap_or_default();
        let mut ebur = cfg.ebur();

        b.iter_custom(|reps| {
            let mut state = AnalysisState::blank(&cfg);
            let mut duration = Duration::ZERO;
            for _ in 0..reps {
                let data = rand::random_iter()
                    .take(cfg.fft.hop_len)
                    .collect::<Vec<_>>();

                let start = Instant::now();
                state = std::hint::black_box(AnalysisState::from_prev(
                    &cfg,
                    state,
                    data.iter().cloned(),
                    &mut ebur,
                ));
                duration += start.elapsed();
            }
            duration
        });
    });
}

criterion_group!(benches, analysis);
criterion_main!(benches);
