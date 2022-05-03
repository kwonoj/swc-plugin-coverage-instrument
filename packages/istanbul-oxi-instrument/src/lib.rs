// Include prebuilt constant values with build script
include!(concat!(env!("OUT_DIR"), "/constants.rs"));
mod source_coverage;

pub use source_coverage::{SourceCoverage, SourceCoverageMeta, SourceCoverageMetaHitCount};

// Reexports
pub use istanbul_oxi_coverage::types::*;
pub use istanbul_oxi_coverage::FileCoverage;
pub use istanbul_oxi_coverage::Range;
pub use istanbul_oxi_coverage::SourceMap;
