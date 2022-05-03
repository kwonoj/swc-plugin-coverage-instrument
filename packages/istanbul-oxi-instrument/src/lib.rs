// Include prebuilt constant values with build script
include!(concat!(env!("OUT_DIR"), "/constants.rs"));
pub mod constants;
pub mod source_coverage;

// Reexports
pub use istanbul_oxi_coverage::types::*;
pub use istanbul_oxi_coverage::FileCoverage;
pub use istanbul_oxi_coverage::Range;
pub use istanbul_oxi_coverage::SourceMap;
