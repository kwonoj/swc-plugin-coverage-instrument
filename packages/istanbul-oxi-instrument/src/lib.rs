// Include prebuilt constant values with build script
include!(concat!(env!("OUT_DIR"), "/constants.rs"));
pub mod constants;
pub mod source_coverage;

mod utils;
//TODO: can this be private?
pub use utils::hint_comments;
pub use utils::lookup_range;
pub use utils::node::Node;
mod instrument;
pub use instrument::create_increase_counter_expr::create_increase_counter_expr;
pub use instrument::create_increase_true_expr::create_increase_true_expr;
mod coverage_template;
pub use coverage_template::create_coverage_data_object;
pub use coverage_template::create_coverage_fn_decl;
pub use coverage_template::create_global_stmt_template;

// Reexports
pub use istanbul_oxi_coverage::types::*;
pub use istanbul_oxi_coverage::FileCoverage;
pub use istanbul_oxi_coverage::Range;
pub use istanbul_oxi_coverage::SourceMap;
