use swc_plugin::ast::*;
use tracing::instrument;

use crate::{create_instrumentation_visitor, instrumentation_counter_helper, utils::node::Node};

create_instrumentation_visitor!(LogicalExprVisitor { branch: u32 });

/// A visitor to traverse down given logical expr's value (left / right) with existing branch idx.
/// This is required to preserve branch id to recursively traverse logical expr's inner child.
impl<'a> LogicalExprVisitor<'a> {
    instrumentation_counter_helper!();
}

impl VisitMut for LogicalExprVisitor<'_> {
    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        let (old, _ignore_current) = self.on_enter(expr);
        expr.visit_mut_children_with(self);
        self.on_exit(old);
    }

    // TODO: common logic between coveragevisitor::visit_mut_bin_expr
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_bin_expr(&mut self, bin_expr: &mut BinExpr) {
        // We don't use self.on_enter() here since Node::LogicalExpr is a dialect of BinExpr
        // which we can't pass directly via on_enter() macro
        let old = self.should_ignore;
        let ignore_current = match old {
            Some(crate::utils::hint_comments::IgnoreScope::Next) => old,
            _ => {
                self.should_ignore = crate::utils::hint_comments::should_ignore(
                    &self.comments,
                    Some(&bin_expr.span),
                );
                self.should_ignore
            }
        };

        match ignore_current {
            Some(crate::utils::hint_comments::IgnoreScope::Next) => {
                self.nodes.push(Node::BinExpr);
                bin_expr.visit_mut_children_with(self);
                self.on_exit(old);
            }
            _ => {
                match &bin_expr.op {
                    BinaryOp::LogicalOr | BinaryOp::LogicalAnd | BinaryOp::NullishCoalescing => {
                        self.nodes.push(Node::LogicalExpr);

                        // Iterate over each expr, wrap it with branch counter.
                        // This does not create new branch counter - should use parent's index instead.
                        self.wrap_bin_expr_with_branch_counter(self.branch, &mut *bin_expr.left);
                        self.wrap_bin_expr_with_branch_counter(self.branch, &mut *bin_expr.right);
                    }
                    _ => {
                        // iterate as normal for non loigical expr
                        self.nodes.push(Node::BinExpr);
                        bin_expr.visit_mut_children_with(self);
                        self.on_exit(old);
                    }
                }
            }
        }
    }
}
