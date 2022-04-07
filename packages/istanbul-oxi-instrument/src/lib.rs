// Include prebuilt constant values with build script
include!(concat!(env!("OUT_DIR"), "/constants.rs"));

mod source_coverage;
mod visitor;

pub use source_coverage::SourceCoverage;
pub use visitor::*;

// Reexport
pub use istanbul_oxi_coverage::FileCoverage;
