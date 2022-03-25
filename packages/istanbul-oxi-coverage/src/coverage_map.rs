use indexmap::IndexMap;

use crate::{CoverageSummary, FileCoverage};

/// a map of `FileCoverage` objects keyed by file paths
#[derive(Clone, PartialEq, Default)]
pub struct CoverageMap {
    inner: IndexMap<String, FileCoverage>,
}

impl CoverageMap {
    pub fn new() -> CoverageMap {
        CoverageMap {
            inner: Default::default(),
        }
    }

    pub fn default() -> CoverageMap {
        CoverageMap {
            inner: Default::default(),
        }
    }

    pub fn from_iter<'a>(value: impl IntoIterator<Item = &'a FileCoverage>) -> CoverageMap {
        let mut ret = CoverageMap {
            inner: Default::default(),
        };

        for coverage in value.into_iter() {
            ret.add_coverage_for_file(coverage)
        }

        ret
    }

    /// Merges a second coverage map into this one
    pub fn merge(&mut self, map: &CoverageMap) {
        for (_, coverage) in map.inner.iter() {
            self.add_coverage_for_file(coverage);
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

    pub fn add_coverage_for_file(&mut self, coverage: &FileCoverage) {
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

#[cfg(test)]
mod tests {
    use crate::{CoverageMap, FileCoverage};

    #[test]
    fn should_able_to_merge_another_coverage_map() {
        let mut base = CoverageMap::from_iter(vec![
            &FileCoverage::from_file_path("foo.js".to_string(), false),
            &FileCoverage::from_file_path("bar.js".to_string(), false),
        ]);

        let mut second = CoverageMap::from_iter(vec![
            &FileCoverage::from_file_path("foo.js".to_string(), false),
            &FileCoverage::from_file_path("baz.js".to_string(), false),
        ]);
        base.merge(&second);
        assert_eq!(
            base.get_files(),
            vec![
                &"foo.js".to_string(),
                &"bar.js".to_string(),
                &"baz.js".to_string()
            ]
        );
    }

    #[test]
    fn should_able_to_return_file_coverage() {
        let mut base = CoverageMap::from_iter(vec![
            &FileCoverage::from_file_path("foo.js".to_string(), false),
            &FileCoverage::from_file_path("bar.js".to_string(), false),
        ]);

        assert!(base.get_coverage_for_file("foo.js").is_some());
        assert!(base.get_coverage_for_file("bar.js").is_some());

        assert!(base.get_coverage_for_file("baz.js").is_none());
    }

    #[test]
    fn should_able_to_filter_coverage() {
        let mut base = CoverageMap::from_iter(vec![
            &FileCoverage::from_file_path("foo.js".to_string(), false),
            &FileCoverage::from_file_path("bar.js".to_string(), false),
        ]);

        assert_eq!(
            base.get_files(),
            vec![&"foo.js".to_string(), &"bar.js".to_string()]
        );

        base.filter(|x| x.path == "foo.js");
        assert_eq!(base.get_files(), vec![&"foo.js".to_string()]);
    }

    #[test]
    fn should_return_coverage_summary_for_all_files() {
        let mut base = CoverageMap::from_iter(vec![
            &FileCoverage::from_file_path("foo.js".to_string(), false),
            &FileCoverage::from_file_path("bar.js".to_string(), false),
        ]);

        base.add_coverage_for_file(&FileCoverage::from_file_path("foo.js".to_string(), false));
        base.add_coverage_for_file(&FileCoverage::from_file_path("baz.js".to_string(), false));

        let summary = base.get_coverage_summary();
        assert_eq!(summary.statements.total, 0);
    }
}
