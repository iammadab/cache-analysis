// Returns the median after sorting in place; for even counts it averages the two middle values.
pub fn median_f64(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).expect("no NaN values expected"));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::median_f64;

    #[test]
    fn median_even_count() {
        let mut values = vec![4.0, 1.0, 3.0, 2.0];
        assert_eq!(median_f64(&mut values), 2.5);
    }
}
