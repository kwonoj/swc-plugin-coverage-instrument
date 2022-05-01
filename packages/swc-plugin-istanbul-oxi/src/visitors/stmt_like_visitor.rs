use istanbul_oxi_instrument::SourceCoverage;
use swc_plugin::{
    ast::*,
    syntax_pos::{Span, DUMMY_SP},
    utils::take::Take,
};
use tracing::instrument;

use crate::{
    constants::idents::*,
    create_instrumentation_visitor, insert_counter_helper, insert_logical_expr_helper,
    insert_stmt_counter,
    instrument::create_increase_expression_expr,
    utils::{
        lookup_range::{get_expr_span, get_range_from_span},
        node::Node,
    },
    visit_mut_coverage,
};

create_instrumentation_visitor!(StmtVisitor {});

impl<'a> StmtVisitor<'a> {
    insert_logical_expr_helper!();
    insert_counter_helper!();
    insert_stmt_counter!();
}

impl VisitMut for StmtVisitor<'_> {
    visit_mut_coverage!();
}
