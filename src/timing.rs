// Reads the TSC at trial start with an lfence to tighten ordering around the timestamp.
fn read_tsc_start() -> u64 {
    unsafe {
        core::arch::x86_64::_mm_lfence();
        core::arch::x86_64::_rdtsc()
    }
}

// Reads the TSC at trial end with an lfence to reduce boundary reordering noise.
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

// Executes pointer chasing in chunks until a minimum cycle budget is reached.
pub fn run_adaptive_trial(
    next: &[u32],
    start_idx: u32,
    chunk_accesses: u64,
    target_cycles: u64,
) -> (u64, u64, u32) {
    let start_cycles = read_tsc_start();
    let mut idx = start_idx;
    let mut accesses_done = 0u64;

    loop {
        idx = chase(next, idx, chunk_accesses);
        accesses_done += chunk_accesses;

        let now_cycles = read_tsc_end();
        if now_cycles - start_cycles >= target_cycles {
            return (accesses_done, now_cycles - start_cycles, idx);
        }
    }
}
