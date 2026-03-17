mod config;
mod cycle_builder;
mod output;
mod pinning;
mod size_grid;
mod stats;
mod timing;
mod types;

use config::Config;
use cycle_builder::build_single_cycle;
use output::write_results;
use pinning::pin_to_core;
use size_grid::build_sizes;
use stats::median_f64;
use timing::run_adaptive_trial;
use types::{AggregateRow, TrialRow};

fn run_sweep(config: &Config, sizes: &[u64]) -> (Vec<TrialRow>, Vec<AggregateRow>) {
    // Runs one warmup phase plus timed trials for each size, then records median cycles/access.
    let mut raw_rows = Vec::new();
    let mut aggregate_rows = Vec::new();

    println!("v4_full_sweep:");
    println!("chunk_accesses={}", config.chunk_accesses);
    println!("target_cycles={}", config.target_cycles);
    println!("warmup_trials={}", config.warmup_trials);

    for &size_bytes in sizes {
        let next = build_single_cycle(size_bytes, config.seed).expect("valid cycle");

        for _ in 0..config.warmup_trials {
            let _ = run_adaptive_trial(&next, 0, config.chunk_accesses, config.target_cycles);
        }

        let mut trial_cpa = Vec::with_capacity(config.trials as usize);
        for trial in 0..config.trials {
            let (accesses_done, elapsed_cycles, _) =
                run_adaptive_trial(&next, 0, config.chunk_accesses, config.target_cycles);
            let cpa = elapsed_cycles as f64 / accesses_done as f64;
            trial_cpa.push(cpa);
            raw_rows.push(TrialRow {
                size_bytes,
                trial,
                accesses: accesses_done,
                elapsed_cycles,
                cycles_per_access: cpa,
            });
        }

        let median_cpa = median_f64(&mut trial_cpa);
        aggregate_rows.push(AggregateRow {
            size_bytes,
            median_cpa,
        });
        println!("size_bytes={} median_cpa={:.4}", size_bytes, median_cpa);
    }

    (raw_rows, aggregate_rows)
}

fn main() {
    let config = Config::default();

    let pin_ok = match pin_to_core(config.pin_core) {
        Ok(()) => {
            println!("pinned_core={}", config.pin_core);
            true
        }
        Err(err) => {
            println!("pinning_warning={err}");
            false
        }
    };

    let sizes = build_sizes(config.min_bytes, config.max_bytes);

    println!("Latency V5");
    println!("min_bytes={}", config.min_bytes);
    println!("max_bytes={}", config.max_bytes);
    println!("trials={}", config.trials);
    println!("seed={}", config.seed);
    println!("sizes:");
    for (i, size) in sizes.iter().enumerate() {
        println!("{}: {}", i + 1, size);
    }
    println!("total_sizes={}", sizes.len());

    let (raw_rows, aggregate_rows) = run_sweep(&config, &sizes);
    write_results(&config, pin_ok, &raw_rows, &aggregate_rows).expect("write output files");

    println!("wrote=results/latency.csv");
    println!("wrote=results/latency_aggregate.csv");
    println!("wrote=results/meta.json");
}
