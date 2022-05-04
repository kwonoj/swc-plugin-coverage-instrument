// Include prebuilt constant values with build script
include!(concat!(env!("OUT_DIR"), "/constants.rs"));
mod constants;
mod source_coverage;

mod instrument;
use instrument::create_increase_counter_expr::create_increase_counter_expr;
use instrument::create_increase_true_expr::create_increase_true_expr;

mod coverage_template;
use coverage_template::create_assignment_stmt::create_assignment_stmt;
use coverage_template::create_coverage_data_object::create_coverage_data_object;
use coverage_template::create_coverage_fn_decl::*;
use coverage_template::create_global_stmt_template::create_global_stmt_template;
use source_coverage::SourceCoverage;

#[macro_use]
mod macros;

mod visitors;
pub use visitors::coverage_visitor::create_coverage_instrumentation_visitor;
mod options;
pub use options::instrument_options::*;

mod utils;
use utils::hint_comments;
use utils::lookup_range;
pub use utils::node::Node;

// Reexports
pub use istanbul_oxi_coverage::types::*;
pub use istanbul_oxi_coverage::FileCoverage;
pub use istanbul_oxi_coverage::Range;
pub use istanbul_oxi_coverage::SourceMap;
