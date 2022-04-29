use istanbul_oxi_instrument::SourceCoverage;
use swc_plugin::{
    ast::*,
    syntax_pos::{Span, DUMMY_SP},
    utils::take::Take,
};
use tracing::instrument;

use crate::{
    constants::idents::*,
    create_coverage_visitor, enter_visitor, insert_counter_helper, insert_logical_expr_helper,
    instrument::create_increase_expression_expr,
    utils::{
        hint_comments::lookup_hint_comments,
        lookup_range::{get_expr_span, get_range_from_span},
        node::Node,
    },
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

create_coverage_visitor!(LogicalExprVisitor { branch: u32 });

/// A visitor to traverse down given logical expr's value (left / right) with existing branch idx.
/// This is required to preserve branch id to recursively traverse logical expr's inner child.
impl<'a> LogicalExprVisitor<'a> {
    insert_logical_expr_helper!();
    insert_counter_helper!();
}

impl VisitMut for LogicalExprVisitor<'_> {
    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        enter_visitor!(self, expr, || {
            expr.visit_mut_children_with(self);
        });
    }

    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_bin_expr(&mut self, bin_expr: &mut BinExpr) {
        if self.should_ignore_child {
            bin_expr.visit_mut_children_with(self);
            return;
        }

        let old = self.should_ignore_child;
        self.should_ignore_child =
            crate::utils::hint_comments::should_ignore_child(&self.comments, Some(&bin_expr.span));

        match &bin_expr.op {
            BinaryOp::LogicalOr | BinaryOp::LogicalAnd | BinaryOp::NullishCoalescing => {
                self.nodes.push(Node::LogicalExpr);

                // escape if there's ignore comments
                let hint = lookup_hint_comments(&self.comments, Some(bin_expr.span).as_ref());
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

        self.should_ignore_child = old;
        self.nodes.pop();
    }
}
