use istanbul_oxi_instrument::SourceCoverage;
use swc_plugin::{
    ast::*,
    syntax_pos::{Span, DUMMY_SP},
    utils::take::Take,
};
use tracing::instrument;

use crate::{
    constants::idents::*,
    create_coverage_visitor, insert_counter_helper, insert_logical_expr_helper,
    instrument::create_increase_expression_expr,
    utils::{
        hint_comments::lookup_hint_comments,
        lookup_range::{get_expr_span, get_range_from_span},
        node::Node,
    },
};

create_coverage_visitor!(LogicalExprVisitor { branch: u32 });

/// A visitor to traverse down given logical expr's value (left / right) with existing branch idx.
/// This is required to preserve branch id to recursively traverse logical expr's inner child.
impl<'a> LogicalExprVisitor<'a> {
    insert_logical_expr_helper!();
    insert_counter_helper!();
}

impl VisitMut for LogicalExprVisitor<'_> {
    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        let (old, _ignore_current) = self.on_enter(expr);
        expr.visit_mut_children_with(self);
        self.on_exit(old);
    }

    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_bin_expr(&mut self, bin_expr: &mut BinExpr) {
        let (old, ignore_current) = self.on_enter(bin_expr);

        match ignore_current {
            Some(crate::utils::hint_comments::IgnoreScope::Next) => {
                bin_expr.visit_mut_children_with(self);
            }
            _ => {
                match &bin_expr.op {
                    BinaryOp::LogicalOr | BinaryOp::LogicalAnd | BinaryOp::NullishCoalescing => {
                        self.nodes.push(Node::LogicalExpr);

                        // escape if there's ignore comments
                        let hint =
                            lookup_hint_comments(&self.comments, Some(bin_expr.span).as_ref());
                        if hint.as_deref() == Some("next") {
                            bin_expr.visit_mut_children_with(self);
                        } else {
                            // Iterate over each expr, wrap it with branch counter.
                            // This does not create new branch counter - should use parent's index instead.
                            self.wrap_bin_expr_with_branch_counter(
                                self.branch,
                                &mut *bin_expr.left,
                            );
                            self.wrap_bin_expr_with_branch_counter(
                                self.branch,
                                &mut *bin_expr.right,
                            );
                        }
                    }
                    _ => {
                        self.nodes.push(Node::BinExpr);
                        bin_expr.visit_mut_children_with(self);
                    }
                }
            }
        }

        self.on_exit(old);
    }
}
