use tracing::instrument;

#[cfg(not(feature = "plugin"))]
use swc_common::{util::take::Take, Span, DUMMY_SP};
#[cfg(not(feature = "plugin"))]
use swc_ecma_ast::*;
#[cfg(not(feature = "plugin"))]
use swc_ecma_visit::*;

#[cfg(feature = "plugin")]
use swc_plugin::{
    ast::*,
    syntax_pos::{Span, DUMMY_SP},
    utils::take::Take,
};

use crate::{
    constants::idents::IDENT_B, create_instrumentation_visitor, instrumentation_counter_helper,
    instrumentation_stmt_counter_helper, instrumentation_visitor,
};

create_instrumentation_visitor!(SwitchCaseVisitor { branch: u32 });

/// A visitor to traverse down given logical expr's value (left / right) with existing branch idx.
/// This is required to preserve branch id to recursively traverse logical expr's inner child.
impl<C: Clone + Comments> SwitchCaseVisitor<C> {
    instrumentation_counter_helper!();
    instrumentation_stmt_counter_helper!();
}

impl<C: Clone + Comments> VisitMut for SwitchCaseVisitor<C> {
    instrumentation_visitor!();

    // SwitchCase: entries(coverSwitchCase),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_switch_case(&mut self, switch_case: &mut SwitchCase) {
        let (old, ignore_current) = self.on_enter(switch_case);
        match ignore_current {
            Some(crate::hint_comments::IgnoreScope::Next) => {}
            _ => {
                // TODO: conslidate brach expr creation, i.e ifstmt
                let range =
                    crate::lookup_range::get_range_from_span(&self.source_map, &switch_case.span);
                let idx = self.cov.borrow_mut().add_branch_path(self.branch, &range);
                let expr = crate::create_increase_counter_expr(
                    &IDENT_B,
                    self.branch,
                    &self.cov_fn_ident,
                    Some(idx),
                );

                switch_case.visit_mut_children_with(self);

                let expr = Stmt::Expr(ExprStmt {
                    span: DUMMY_SP,
                    expr: Box::new(expr),
                });

                let mut new_stmts = vec![expr];
                new_stmts.extend(switch_case.cons.drain(..));

                switch_case.cons = new_stmts;
            }
        }
        self.on_exit(old);
    }
}