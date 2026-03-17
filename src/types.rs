pub struct TrialRow {
    pub size_bytes: u64,
    pub trial: u32,
    pub accesses: u64,
    pub elapsed_cycles: u64,
    pub cycles_per_access: f64,
}

pub struct AggregateRow {
    pub size_bytes: u64,
    pub median_cpa: f64,
}
