pub fn percent(covered: u32, total: u32) -> f32 {
    if total > 0 {
        let tmp: f64 = ((1000 * 100 * covered as u64) / total as u64) as f64;
        return (tmp as f32 / 10 as f32).floor() / 100 as f32;
    } else {
        return 100.0;
    }
}

#[cfg(test)]
mod tests {
    use crate::percent;

    #[test]
    fn calculate_percentage_covered_and_total() {
        let p = percent(1, 1);
        assert_eq!(p as i32, 100);
    }

    #[test]
    fn calculate_percentage_with_precision() {
        let p = percent(999998, 999999);
        assert_eq!(p < 100 as f32, true);
    }
}
