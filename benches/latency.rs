//! Latency simulation benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use llm_simulator::config::{LatencyConfig, LatencyDistribution};
use llm_simulator::latency::{LatencySimulator, DistributionSampler};

fn bench_distribution_sampling(c: &mut Criterion) {
    let sampler = DistributionSampler::with_seed(42);

    let mut group = c.benchmark_group("distribution_sampling");

    // Fixed distribution
    let fixed = LatencyDistribution::Fixed { value_ms: 100.0 };
    group.bench_function("fixed", |b| {
        b.iter(|| black_box(sampler.sample(&fixed)))
    });

    // Normal distribution
    let normal = LatencyDistribution::Normal {
        mean_ms: 100.0,
        std_dev_ms: 20.0,
    };
    group.bench_function("normal", |b| {
        b.iter(|| black_box(sampler.sample(&normal)))
    });

    // Log-normal distribution
    let log_normal = LatencyDistribution::LogNormal {
        mean_ms: 100.0,
        std_dev_ms: 50.0,
    };
    group.bench_function("log_normal", |b| {
        b.iter(|| black_box(sampler.sample(&log_normal)))
    });

    // Uniform distribution
    let uniform = LatencyDistribution::Uniform {
        min_ms: 50.0,
        max_ms: 150.0,
    };
    group.bench_function("uniform", |b| {
        b.iter(|| black_box(sampler.sample(&uniform)))
    });

    // Exponential distribution
    let exponential = LatencyDistribution::Exponential { mean_ms: 100.0 };
    group.bench_function("exponential", |b| {
        b.iter(|| black_box(sampler.sample(&exponential)))
    });

    // Pareto distribution
    let pareto = LatencyDistribution::Pareto {
        scale_ms: 10.0,
        shape: 2.0,
    };
    group.bench_function("pareto", |b| {
        b.iter(|| black_box(sampler.sample(&pareto)))
    });

    group.finish();
}

fn bench_latency_simulator(c: &mut Criterion) {
    let config = LatencyConfig::default();
    let simulator = LatencySimulator::with_seed(config, 42);

    let mut group = c.benchmark_group("latency_simulator");

    // TTFT sampling
    group.bench_function("sample_ttft", |b| {
        b.iter(|| black_box(simulator.sample_ttft(Some("standard"))))
    });

    // ITL sampling
    group.bench_function("sample_itl", |b| {
        b.iter(|| black_box(simulator.sample_itl(Some("standard"))))
    });

    // Schedule generation
    group.bench_function("generate_schedule_100", |b| {
        b.iter(|| black_box(simulator.generate_schedule(100, Some("standard"))))
    });

    group.bench_function("generate_schedule_500", |b| {
        b.iter(|| black_box(simulator.generate_schedule(500, Some("standard"))))
    });

    group.bench_function("generate_schedule_1000", |b| {
        b.iter(|| black_box(simulator.generate_schedule(1000, Some("standard"))))
    });

    group.finish();
}

fn bench_batch_sampling(c: &mut Criterion) {
    let sampler = DistributionSampler::with_seed(42);
    let dist = LatencyDistribution::Normal {
        mean_ms: 100.0,
        std_dev_ms: 20.0,
    };

    let mut group = c.benchmark_group("batch_sampling");

    for n in [100, 1000, 10000].iter() {
        group.bench_function(format!("sample_n_{}", n), |b| {
            b.iter(|| black_box(sampler.sample_n(&dist, *n)))
        });
    }

    group.finish();
}

fn bench_profile_lookup(c: &mut Criterion) {
    let config = LatencyConfig::default();
    let simulator = LatencySimulator::new(config);

    let mut group = c.benchmark_group("profile_lookup");

    group.bench_function("get_profile_hit", |b| {
        b.iter(|| black_box(simulator.get_profile("standard")))
    });

    group.bench_function("get_profile_miss", |b| {
        b.iter(|| black_box(simulator.get_profile("nonexistent")))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_distribution_sampling,
    bench_latency_simulator,
    bench_batch_sampling,
    bench_profile_lookup,
);

criterion_main!(benches);
