use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use istanbul_oxi_coverage::{FileCoverage, Range};
use once_cell::sync::Lazy;

static COVERAGE_MAGIC_VALUE: Lazy<String> = Lazy::new(|| {
    let mut s = DefaultHasher::new();
    let name = "istanbul-oxi-instrument";
    format!("{}@{}", name, 4).hash(&mut s);
    return format!("cov_{}", s.finish());
});

/// SourceCoverage provides mutation methods to manipulate the structure of
/// a file coverage object. Used by the instrumenter to create a full coverage
/// object for a file incrementally.
pub struct SourceCoverage {
    inner: FileCoverage,
}

pub struct UnknownReserved;

impl SourceCoverage {
    pub fn new(file_path: String, report_logic: bool) -> Self {
        SourceCoverage {
            inner: FileCoverage::from_file_path(file_path, report_logic),
        }
    }
}

impl SourceCoverage {
    pub fn new_statement(&mut self, loc: Range) {}
    pub fn new_function(&mut self, decl: Range, loc: Range) {}
    pub fn new_branch(&mut self, loc: Range, is_report_logic: bool) {}
    pub fn maybe_new_branch_true(&mut self, name: u32, is_report_logic: bool) {}
    pub fn add_branch_path(&mut self, name: u32, location: Range) {}
    pub fn maybe_add_branch_true(&mut self, name: u32) {}
    pub fn set_input_source_map(&mut self, source_map: UnknownReserved) {}
    pub fn freeze(&mut self) {}
}
