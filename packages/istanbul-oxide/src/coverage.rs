#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Coverage {
    covered: u32,
    total: u32,
    coverage: f32,
}

impl Coverage {
    pub fn new(covered: u32, total: u32, coverage: f32) -> Coverage {
        Coverage {
            covered,
            total,
            coverage,
        }
    }
}
