use istanbul_oxi_instrument::SourceCoverage;
use swc_plugin::{
    ast::*,
    comments::PluginCommentsProxy,
    source_map::PluginSourceMapProxy,
    syntax_pos::{Span, DUMMY_SP},
    utils::take::Take,
};
use tracing::instrument;

use crate::{
    constants::idents::*,
    insert_counter_helper, insert_logical_expr_helper,
    instrument::create_increase_expression_expr,
    utils::{
        lookup_range::{get_expr_span, get_range_from_span},
        node::Node,
    },
    InstrumentOptions,
};

/// Traverse down given nodes to check if it's leaf of the logical expr,
/// or have inner logical expr to recurse.
pub struct LogicalExprLeafFinder(pub bool);

impl Visit for LogicalExprLeafFinder {
    fn visit_bin_expr(&mut self, bin_expr: &BinExpr) {
        match &bin_expr.op {
            BinaryOp::LogicalOr | BinaryOp::LogicalAnd | BinaryOp::NullishCoalescing => {
                self.0 = true;
                // short curcuit, we know it's not leaf
                return;
            }
            _ => {}
        }

        bin_expr.visit_children_with(self);
    }
}

pub struct LogicalExprVisitor<'a> {
    pub source_map: &'a PluginSourceMapProxy,
    pub comments: Option<&'a PluginCommentsProxy>,
    pub cov: &'a mut SourceCoverage,
    pub var_name_ident: Ident,
    pub instrument_options: InstrumentOptions,
    pub before: Vec<Stmt>,
    pub nodes: Vec<Node>,
    pub branch: u32,
}

/// A visitor to traverse down given logical expr's value (left / right) with existing branch idx.
/// This is required to preserve branch id to recursively traverse logical expr's inner child.
impl<'a> LogicalExprVisitor<'a> {
    insert_logical_expr_helper!();
    insert_counter_helper!();

    pub fn new(
        source_map: &'a PluginSourceMapProxy,
        comments: Option<&'a PluginCommentsProxy>,
        cov: &'a mut SourceCoverage,
        var_name_ident: &'a Ident,
        instrument_options: &InstrumentOptions,
        current_node: &[Node],
        branch: u32,
    ) -> LogicalExprVisitor<'a> {
        LogicalExprVisitor {
            source_map,
            comments,
            cov,
            var_name_ident: var_name_ident.clone(),
            instrument_options: instrument_options.clone(),
            before: vec![],
            nodes: current_node.to_vec(),
            branch,
        }
    }
}

impl VisitMut for LogicalExprVisitor<'_> {
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_bin_expr(&mut self, bin_expr: &mut BinExpr) {
        match &bin_expr.op {
            BinaryOp::LogicalOr | BinaryOp::LogicalAnd | BinaryOp::NullishCoalescing => {
                self.nodes.push(Node::LogicalExpr);

                // escape if there's ignore comments
                let hint = self.lookup_hint_comments(Some(bin_expr.span).as_ref());
                if hint.as_deref() == Some("next") {
                    bin_expr.visit_mut_children_with(self);
                    self.nodes.pop();
                    return;
                }

                // Iterate over each expr, wrap it with branch counter.
                // This does not create new branch counter - should use parent's index instead.
                self.wrap_bin_expr_with_branch_counter(self.branch, &mut *bin_expr.left);
                self.wrap_bin_expr_with_branch_counter(self.branch, &mut *bin_expr.right);
            }
            _ => {
                self.nodes.push(Node::BinExpr);
                bin_expr.visit_mut_children_with(self);
            }
        }
        self.nodes.pop();
    }
}
