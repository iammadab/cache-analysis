// Builds a working-set schedule with powers of two and one midpoint between each adjacent pair.
pub fn build_sizes(min_bytes: u64, max_bytes: u64) -> Vec<u64> {
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

#[cfg(test)]
mod tests {
    use super::build_sizes;

    #[test]
    fn size_grid_is_strictly_increasing() {
        let sizes = build_sizes(4 * 1024, 512 * 1024 * 1024);
        assert!(!sizes.is_empty());

        for pair in sizes.windows(2) {
            assert!(pair[0] < pair[1]);
        }
    }
}
