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
}
