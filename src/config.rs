pub struct Config {
    pub min_bytes: u64,
    pub max_bytes: u64,
    pub trials: u32,
    pub warmup_trials: u32,
    pub seed: u64,
    pub chunk_accesses: u64,
    pub target_cycles: u64,
    pub pin_core: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            min_bytes: 4 * 1024,
            max_bytes: 512 * 1024 * 1024,
            trials: 9,
            warmup_trials: 1,
            seed: 0xC0FFEE,
            chunk_accesses: 65_536,
            target_cycles: 200_000_000,
            pin_core: 0,
        }
    }
}
