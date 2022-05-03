// Include prebuilt constant values with build script
include!(concat!(env!("OUT_DIR"), "/constants.rs"));
pub mod constants;
pub mod source_coverage;

mod utils;
//TODO: can this be private?
pub use utils::hint_comments;
pub use utils::lookup_range;
pub use utils::node::Node;

// Reexports
pub use istanbul_oxi_coverage::types::*;
pub use istanbul_oxi_coverage::FileCoverage;
pub use istanbul_oxi_coverage::Range;
pub use istanbul_oxi_coverage::SourceMap;
