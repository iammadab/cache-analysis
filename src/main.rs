const MIN_BYTES: u64 = 4 * 1024;
const MAX_BYTES: u64 = 512 * 1024 * 1024;
const TRIALS: u32 = 9;
const SEED: u64 = 0xC0FFEE;

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

fn is_single_cycle(next: &[u32]) -> bool {
    if next.len() < 2 {
        return false;
    }

    let n = next.len();
    let mut idx = 0usize;

    for step in 1..=n {
        let next_idx = next[idx] as usize;
        if next_idx >= n {
            return false;
        }

        idx = next_idx;
        if idx == 0 {
            return step == n;
        }
    }

    false
}

fn main() {
    let sizes = build_sizes(MIN_BYTES, MAX_BYTES);

    println!("Latency V0");
    println!("min_bytes={MIN_BYTES}");
    println!("max_bytes={MAX_BYTES}");
    println!("trials={TRIALS}");
    println!("seed={SEED}");
    println!("sizes:");

    for (i, size) in sizes.iter().enumerate() {
        println!("{}: {}", i + 1, size);
    }

    println!("total_sizes={}", sizes.len());

    println!("v1_cycle_check:");
    for size in sizes {
        let element_count = (size / std::mem::size_of::<u32>() as u64) as usize;
        let cycle_ok = build_single_cycle(size, SEED)
            .map(|next| is_single_cycle(&next))
            .unwrap_or(false);

        println!(
            "size_bytes={} elements={} cycle_ok={}",
            size, element_count, cycle_ok
        );
    }
}
