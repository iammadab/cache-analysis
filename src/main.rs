use std::fs::{self, File};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

const MIN_BYTES: u64 = 4 * 1024;
const MAX_BYTES: u64 = 512 * 1024 * 1024;
const TRIALS: u32 = 9;
const SEED: u64 = 0xC0FFEE;
const CHUNK_ACCESSES: u64 = 65_536;
const TARGET_CYCLES: u64 = 200_000_000;
const PIN_CORE: usize = 0;
const WARMUP_TRIALS: u32 = 1;

// Builds a working-set grid with powers of two plus one midpoint between each pair.
fn build_sizes(min_bytes: u64, max_bytes: u64) -> Vec<u64> {
    let mut powers = Vec::new();
    let mut value = min_bytes;

    while value <= max_bytes {
        powers.push(value);
        value *= 2;
    }

    let mut sizes = Vec::with_capacity(powers.len() * 2);
    for window in powers.windows(2) {
        let a = window[0];
        let b = window[1];
        sizes.push(a);
        sizes.push((a + b) / 2);
    }

    if let Some(last) = powers.last() {
        sizes.push(*last);
    }

    sizes
}

fn next_u64(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

fn shuffle_u32(values: &mut [u32], seed: u64) {
    let mut rng_state = seed;
    if values.len() < 2 {
        return;
    }

    for i in (1..values.len()).rev() {
        let j = (next_u64(&mut rng_state) % ((i + 1) as u64)) as usize;
        values.swap(i, j);
    }
}

fn build_single_cycle(working_set_bytes: u64, seed: u64) -> Option<Vec<u32>> {
    let element_size = std::mem::size_of::<u32>() as u64;
    let n_u64 = working_set_bytes / element_size;
    if n_u64 < 2 || n_u64 > u32::MAX as u64 {
        return None;
    }

    let n = n_u64 as usize;
    let mut perm: Vec<u32> = (0..(n as u32)).collect();
    shuffle_u32(&mut perm, seed ^ working_set_bytes);

    let mut next = vec![0u32; n];
    for i in 0..(n - 1) {
        next[perm[i] as usize] = perm[i + 1];
    }
    next[perm[n - 1] as usize] = perm[0];

    Some(next)
}

fn read_tsc_start() -> u64 {
    unsafe {
        core::arch::x86_64::_mm_lfence();
        core::arch::x86_64::_rdtsc()
    }
}

fn read_tsc_end() -> u64 {
    unsafe {
        core::arch::x86_64::_mm_lfence();
        core::arch::x86_64::_rdtsc()
    }
}

fn chase(next: &[u32], mut idx: u32, accesses: u64) -> u32 {
    for _ in 0..accesses {
        idx = next[idx as usize];
    }
    std::hint::black_box(idx)
}

fn run_adaptive_trial(next: &[u32], start_idx: u32) -> (u64, u64, u32) {
    let start_cycles = read_tsc_start();
    let mut idx = start_idx;
    let mut accesses_done = 0u64;

    loop {
        idx = chase(next, idx, CHUNK_ACCESSES);
        accesses_done += CHUNK_ACCESSES;

        let now_cycles = read_tsc_end();
        if now_cycles - start_cycles >= TARGET_CYCLES {
            return (accesses_done, now_cycles - start_cycles, idx);
        }
    }
}

fn median_f64(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).expect("no NaN values expected"));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) / 2.0
    }
}

fn pin_to_core(core_id: usize) -> Result<(), std::io::Error> {
    let mut set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
    unsafe {
        libc::CPU_ZERO(&mut set);
        libc::CPU_SET(core_id, &mut set);

        let rc = libc::sched_setaffinity(
            0,
            std::mem::size_of::<libc::cpu_set_t>(),
            &set as *const libc::cpu_set_t,
        );

        if rc == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}

fn main() {
    let pin_ok = match pin_to_core(PIN_CORE) {
        Ok(()) => {
            println!("pinned_core={PIN_CORE}");
            true
        }
        Err(err) => {
            println!("pinning_warning={err}");
            false
        }
    };

    let sizes = build_sizes(MIN_BYTES, MAX_BYTES);
    let mut raw_rows: Vec<(u64, u32, u64, u64, f64)> = Vec::new();
    let mut aggregate_rows: Vec<(u64, f64)> = Vec::new();

    println!("Latency V4");
    println!("min_bytes={MIN_BYTES}");
    println!("max_bytes={MAX_BYTES}");
    println!("trials={TRIALS}");
    println!("seed={SEED}");
    println!("sizes:");

    for (i, size) in sizes.iter().enumerate() {
        println!("{}: {}", i + 1, size);
    }

    println!("total_sizes={}", sizes.len());

    println!("v4_full_sweep:");
    println!("chunk_accesses={CHUNK_ACCESSES}");
    println!("target_cycles={TARGET_CYCLES}");
    println!("warmup_trials={WARMUP_TRIALS}");

    for size_bytes in sizes {
        let next = build_single_cycle(size_bytes, SEED).expect("valid cycle");

        for _ in 0..WARMUP_TRIALS {
            let _ = run_adaptive_trial(&next, 0);
        }

        let mut trial_cpa = Vec::with_capacity(TRIALS as usize);
        for trial in 0..TRIALS {
            let (accesses_done, elapsed_cycles, _) = run_adaptive_trial(&next, 0);
            let cpa = elapsed_cycles as f64 / accesses_done as f64;
            trial_cpa.push(cpa);
            raw_rows.push((size_bytes, trial, accesses_done, elapsed_cycles, cpa));
        }

        let median_cpa = median_f64(&mut trial_cpa);
        aggregate_rows.push((size_bytes, median_cpa));
        println!("size_bytes={} median_cpa={:.4}", size_bytes, median_cpa);
    }

    fs::create_dir_all("results").expect("create results dir");

    let mut raw_file = File::create("results/latency.csv").expect("create latency.csv");
    writeln!(
        raw_file,
        "size_bytes,trial,accesses,elapsed_cycles,cycles_per_access"
    )
    .expect("write latency.csv header");
    for (size_bytes, trial, accesses, elapsed_cycles, cpa) in raw_rows {
        writeln!(
            raw_file,
            "{},{},{},{},{:.6}",
            size_bytes, trial, accesses, elapsed_cycles, cpa
        )
        .expect("write latency.csv row");
    }

    let mut agg_file =
        File::create("results/latency_aggregate.csv").expect("create latency_aggregate.csv");
    writeln!(agg_file, "size_bytes,median_cpa").expect("write latency_aggregate.csv header");
    for (size_bytes, median_cpa) in aggregate_rows {
        writeln!(agg_file, "{},{:.6}", size_bytes, median_cpa)
            .expect("write latency_aggregate.csv row");
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("valid system time")
        .as_secs();
    let mut meta_file = File::create("results/meta.json").expect("create meta.json");
    writeln!(meta_file, "{{").expect("write meta start");
    writeln!(meta_file, "  \"timestamp_unix\": {},", timestamp).expect("write meta timestamp");
    writeln!(meta_file, "  \"min_bytes\": {},", MIN_BYTES).expect("write meta min");
    writeln!(meta_file, "  \"max_bytes\": {},", MAX_BYTES).expect("write meta max");
    writeln!(meta_file, "  \"trials\": {},", TRIALS).expect("write meta trials");
    writeln!(meta_file, "  \"warmup_trials\": {},", WARMUP_TRIALS).expect("write meta warmup");
    writeln!(meta_file, "  \"seed\": {},", SEED).expect("write meta seed");
    writeln!(meta_file, "  \"chunk_accesses\": {},", CHUNK_ACCESSES).expect("write meta chunk");
    writeln!(meta_file, "  \"target_cycles\": {},", TARGET_CYCLES).expect("write meta target");
    writeln!(meta_file, "  \"pin_core\": {},", PIN_CORE).expect("write meta pin core");
    writeln!(meta_file, "  \"pinning_succeeded\": {},", pin_ok).expect("write meta pin ok");
    writeln!(meta_file, "  \"timer_mode\": \"rdtsc\"").expect("write meta timer");
    writeln!(meta_file, "}}").expect("write meta end");

    println!("wrote=results/latency.csv");
    println!("wrote=results/latency_aggregate.csv");
    println!("wrote=results/meta.json");
}

#[cfg(test)]
mod tests {
    use super::{build_single_cycle, build_sizes};

    #[test]
    fn single_cycle_visits_all_elements_once() {
        let size_bytes = 4 * 1024;
        let next = build_single_cycle(size_bytes, 0xC0FFEE).expect("valid cycle");
        let n = next.len();
        let mut seen = vec![false; n];
        let mut idx = 0usize;

        for _ in 0..n {
            assert!(!seen[idx]);
            seen[idx] = true;
            idx = next[idx] as usize;
            assert!(idx < n);
        }

        assert_eq!(idx, 0);
        assert!(seen.into_iter().all(|x| x));
    }

    #[test]
    fn size_grid_is_strictly_increasing() {
        let sizes = build_sizes(4 * 1024, 512 * 1024 * 1024);
        assert!(!sizes.is_empty());

        for pair in sizes.windows(2) {
            assert!(pair[0] < pair[1]);
        }
    }
}
