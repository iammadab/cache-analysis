use crate::config::Config;
use crate::types::{AggregateRow, TrialRow};
use std::fs::{self, File};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn write_results(
    config: &Config,
    pin_ok: bool,
    raw_rows: &[TrialRow],
    aggregate_rows: &[AggregateRow],
) -> std::io::Result<()> {
    fs::create_dir_all("results")?;

    let mut raw_file = File::create("results/latency.csv")?;
    writeln!(
        raw_file,
        "size_bytes,trial,accesses,elapsed_cycles,cycles_per_access"
    )?;
    for row in raw_rows {
        writeln!(
            raw_file,
            "{},{},{},{},{:.6}",
            row.size_bytes, row.trial, row.accesses, row.elapsed_cycles, row.cycles_per_access
        )?;
    }

    let mut agg_file = File::create("results/latency_aggregate.csv")?;
    writeln!(agg_file, "size_bytes,median_cpa")?;
    for row in aggregate_rows {
        writeln!(agg_file, "{},{:.6}", row.size_bytes, row.median_cpa)?;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("valid system time")
        .as_secs();
    let mut meta_file = File::create("results/meta.json")?;
    writeln!(meta_file, "{{")?;
    writeln!(meta_file, "  \"timestamp_unix\": {},", timestamp)?;
    writeln!(meta_file, "  \"min_bytes\": {},", config.min_bytes)?;
    writeln!(meta_file, "  \"max_bytes\": {},", config.max_bytes)?;
    writeln!(meta_file, "  \"trials\": {},", config.trials)?;
    writeln!(meta_file, "  \"warmup_trials\": {},", config.warmup_trials)?;
    writeln!(meta_file, "  \"seed\": {},", config.seed)?;
    writeln!(
        meta_file,
        "  \"chunk_accesses\": {},",
        config.chunk_accesses
    )?;
    writeln!(meta_file, "  \"target_cycles\": {},", config.target_cycles)?;
    writeln!(meta_file, "  \"pin_core\": {},", config.pin_core)?;
    writeln!(meta_file, "  \"pinning_succeeded\": {},", pin_ok)?;
    writeln!(meta_file, "  \"timer_mode\": \"rdtsc\"")?;
    writeln!(meta_file, "}}")?;

    Ok(())
}
