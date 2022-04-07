mod constants;
mod source_coverage;
mod visitor;

pub use source_coverage::SourceCoverage;
pub use visitor::*;

pub use constants::{COVERAGE_MAGIC_KEY, COVERAGE_MAGIC_VALUE};

// Reexport
pub use istanbul_oxi_coverage::FileCoverage;
