use indexmap::IndexMap;

use crate::{CoverageSummary, FileCoverage};

/// a map of `FileCoverage` objects keyed by file paths
#[derive(Clone, PartialEq)]
pub struct CoverageMap {
    inner: IndexMap<String, FileCoverage>,
}

impl CoverageMap {
    pub fn new() -> CoverageMap {
        CoverageMap {
            inner: Default::default(),
        }
    }

    /// Merges a second coverage map into this one
    pub fn merge(&mut self, map: &CoverageMap) {
        for (_, coverage) in map.inner.iter() {
            self.add_covreage_for_file(coverage);
        }
    }

    /// Filter the coverage map with a predicate. If the predicate returns false,
    /// the coverage is removed from the map.
    pub fn filter(&mut self, predicate: impl Fn(&FileCoverage) -> bool) {
        let mut filtered: IndexMap<String, FileCoverage> = Default::default();

        for (_, coverage) in self.inner.drain(..) {
            if predicate(&coverage) {
                filtered.insert(coverage.path.clone(), coverage);
            }
        }

        self.inner = filtered;
    }

    pub fn to_json() {
        unimplemented!()
    }

    pub fn get_files(&self) -> Vec<&String> {
        self.inner.keys().collect()
    }

    pub fn get_coverage_for_file(&self, file_path: &str) -> Option<&FileCoverage> {
        self.inner.get(file_path)
    }

    pub fn add_covreage_for_file(&mut self, coverage: &FileCoverage) {
        if let Some(value) = self.inner.get_mut(coverage.path.as_str()) {
            value.merge(coverage);
        } else {
            self.inner.insert(coverage.path.clone(), coverage.clone());
        }
    }

    pub fn get_coverage_summary(&self) -> CoverageSummary {
        let mut ret: CoverageSummary = Default::default();

        for coverage in self.inner.values() {
            ret.merge(&coverage.to_summary());
        }

        ret
    }
}
