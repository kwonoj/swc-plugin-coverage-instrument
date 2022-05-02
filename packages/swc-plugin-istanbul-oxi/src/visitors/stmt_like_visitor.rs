use istanbul_oxi_instrument::SourceCoverage;
use swc_plugin::{
    ast::*,
    syntax_pos::{Span, DUMMY_SP},
    utils::take::Take,
};
use tracing::instrument;

use crate::{
    constants::idents::*,
    create_instrumentation_visitor,
    instrument::create_increase_expression_expr,
    instrumentation_counter_helper, instrumentation_stmt_counter_helper, instrumentation_visitor,
    utils::{
        lookup_range::{get_expr_span, get_range_from_span},
        node::Node,
    },
};

create_instrumentation_visitor!(StmtVisitor {});

impl<'a> StmtVisitor<'a> {
    instrumentation_counter_helper!();
    instrumentation_stmt_counter_helper!();
}

impl VisitMut for StmtVisitor<'_> {
    instrumentation_visitor!();
}
