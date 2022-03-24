use crate::percent;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum CoveragePercentage {
    Unknown,
    Value(f32),
}

impl Default for CoveragePercentage {
    fn default() -> Self {
        CoveragePercentage::Unknown
    }
}

#[derive(Default, Copy, Clone, PartialEq, Debug)]
pub struct Totals {
    pub total: u32,
    pub covered: u32,
    pub skipped: u32,
    pub pct: CoveragePercentage,
}

impl Totals {
    pub fn new(total: u32, covered: u32, skipped: u32, pct: CoveragePercentage) -> Totals {
        Totals {
            total,
            covered,
            skipped,
            pct,
        }
    }

    pub fn default() -> Totals {
        Totals {
            total: 0,
            covered: 0,
            skipped: 0,
            pct: CoveragePercentage::Unknown,
        }
    }
}

#[derive(Default, Copy, Clone)]
pub struct CoverageSummary {
    lines: Totals,
    statements: Totals,
    functions: Totals,
    branches: Totals,
    branches_true: Option<Totals>,
}

impl CoverageSummary {
    pub fn new(
        lines: Totals,
        statements: Totals,
        functions: Totals,
        branches: Totals,
        branches_true: Option<Totals>,
    ) -> CoverageSummary {
        CoverageSummary {
            lines,
            statements,
            functions,
            branches,
            branches_true,
        }
    }

    pub fn from(summary: &CoverageSummary) -> CoverageSummary {
        CoverageSummary {
            lines: summary.lines,
            statements: summary.statements,
            functions: summary.functions,
            branches: summary.branches,
            branches_true: summary.branches_true,
        }
    }

    pub fn default() -> CoverageSummary {
        CoverageSummary {
            lines: Default::default(),
            statements: Default::default(),
            functions: Default::default(),
            branches: Default::default(),
            branches_true: Some(Default::default()),
        }
    }

    /// Merges a second summary coverage object into this one
    pub fn merge(&mut self, summary: &CoverageSummary) {
        self.lines.total += summary.lines.total;
        self.lines.covered += summary.lines.covered;
        self.lines.skipped += summary.lines.skipped;
        self.lines.pct = CoveragePercentage::Value(percent(self.lines.covered, self.lines.total));

        self.statements.total += summary.statements.total;
        self.statements.covered += summary.statements.covered;
        self.statements.skipped += summary.statements.skipped;
        self.statements.pct =
            CoveragePercentage::Value(percent(self.statements.covered, self.statements.total));

        self.functions.total += summary.functions.total;
        self.functions.covered += summary.functions.covered;
        self.functions.skipped += summary.functions.skipped;
        self.functions.pct =
            CoveragePercentage::Value(percent(self.functions.covered, self.functions.total));

        self.branches.total += summary.branches.total;
        self.branches.covered += summary.branches.covered;
        self.branches.skipped += summary.branches.skipped;
        self.branches.pct =
            CoveragePercentage::Value(percent(self.branches.covered, self.branches.total));

        if let Some(branches_true) = summary.branches_true {
            let mut self_branches_true = if let Some(self_value) = self.branches_true {
                self_value
            } else {
                Default::default()
            };

            self_branches_true.total += branches_true.total;
            self_branches_true.covered += branches_true.covered;
            self_branches_true.skipped += branches_true.skipped;
            self_branches_true.pct = CoveragePercentage::Value(percent(
                self_branches_true.covered,
                self_branches_true.total,
            ));

            self.branches_true = Some(self_branches_true);
        }
    }

    pub fn to_json(&self) {
        unimplemented!("Not implemented yet")
    }

    pub fn is_empty(&self) -> bool {
        self.lines.total == 0
    }
}

#[cfg(test)]
mod tests {
    use crate::{CoveragePercentage, CoverageSummary, Totals};

    #[test]
    fn should_able_to_create_empty() {
        let summary = CoverageSummary::default();

        assert!(summary.is_empty());
    }

    #[test]
    fn should_able_to_merge() {
        let basic = Totals::new(5, 4, 0, crate::CoveragePercentage::Value(80.0));
        let empty = Totals::default();

        let mut first = CoverageSummary::new(basic, basic, basic, empty, Some(empty));
        let mut second = first.clone();

        second.statements.covered = 5;
        first.merge(&second);

        assert_eq!(
            first.statements,
            Totals::new(10, 9, 0, CoveragePercentage::Value(90.0))
        );

        assert_eq!(first.branches.pct, CoveragePercentage::Value(100.0));
        let branches_true = first.branches_true.expect("Should exist");
        assert_eq!(branches_true.pct, CoveragePercentage::Value(100.0));
    }
}
