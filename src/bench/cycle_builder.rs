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

// Builds one randomized cycle spanning all elements so traversal cannot get stuck in tiny hot loops.
pub fn build_single_cycle(working_set_bytes: u64, seed: u64) -> Option<Vec<u32>> {
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

#[cfg(test)]
mod tests {
    use super::build_single_cycle;

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
}
