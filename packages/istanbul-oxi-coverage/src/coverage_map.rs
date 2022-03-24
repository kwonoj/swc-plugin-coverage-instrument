use crate::CoverageSummary;

#[derive(Copy, Clone)]
pub struct CoverageMap {}

impl CoverageMap {
    pub fn new() -> CoverageMap {
        CoverageMap {}
    }

    /// Merges a second coverage map into this one
    pub fn merge(&mut self, map: &CoverageMap) {
        unimplemented!()
    }

    /// Filter the coverage map with a predicate
    pub fn filter() {
        unimplemented!()
    }

    pub fn to_json() {
        unimplemented!()
    }

    pub fn get_files() {
        unimplemented!()
    }

    pub fn get_coverage_for_file() {
        unimplemented!()
    }

    pub fn add_covreage_for_file() {
        unimplemented!()
    }

    pub fn get_coverage_summary() -> CoverageSummary {
        unimplemented!()
    }
}
