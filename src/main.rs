const MIN_BYTES: u64 = 4 * 1024;
const MAX_BYTES: u64 = 512 * 1024 * 1024;
const TRIALS: u32 = 9;
const SEED: u64 = 0xC0FFEE;
const V2_TRIAL_SIZE_BYTES: u64 = 4 * 1024 * 1024;
const V2_ACCESS_COUNT: u64 = 20_000_000;

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

fn main() {
    let sizes = build_sizes(MIN_BYTES, MAX_BYTES);

    println!("Latency V2");
    println!("min_bytes={MIN_BYTES}");
    println!("max_bytes={MAX_BYTES}");
    println!("trials={TRIALS}");
    println!("seed={SEED}");
    println!("sizes:");

    for (i, size) in sizes.iter().enumerate() {
        println!("{}: {}", i + 1, size);
    }

    println!("total_sizes={}", sizes.len());

    let next = build_single_cycle(V2_TRIAL_SIZE_BYTES, SEED).expect("valid v2 trial cycle");
    let start = read_tsc_start();
    let end_idx = chase(&next, 0, V2_ACCESS_COUNT);
    let end = read_tsc_end();
    let elapsed_cycles = end - start;
    let cycles_per_access = elapsed_cycles as f64 / V2_ACCESS_COUNT as f64;

    println!("v2_single_trial:");
    println!("size_bytes={V2_TRIAL_SIZE_BYTES}");
    println!("accesses={V2_ACCESS_COUNT}");
    println!("elapsed_cycles={elapsed_cycles}");
    println!("cycles_per_access={cycles_per_access:.4}");
    println!("end_idx={end_idx}");
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
