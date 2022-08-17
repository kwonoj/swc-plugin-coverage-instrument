use swc_core::{
    ast::*,
    common::{comments::Comments, util::take::Take, SourceMapper, DUMMY_SP},
    visit::{noop_visit_mut_type, VisitMut, VisitMutWith, VisitWith},
};
use tracing::instrument;

use crate::{
    constants::idents::IDENT_B, create_instrumentation_visitor, instrumentation_counter_helper,
    instrumentation_stmt_counter_helper, instrumentation_visitor,
};

create_instrumentation_visitor!(SwitchCaseVisitor { branch: u32 });

/// A visitor to traverse down given logical expr's value (left / right) with existing branch idx.
/// This is required to preserve branch id to recursively traverse logical expr's inner child.
impl<C: Clone + Comments, S: SourceMapper> SwitchCaseVisitor<C, S> {
    instrumentation_counter_helper!();
    instrumentation_stmt_counter_helper!();
}

impl<C: Clone + Comments, S: SourceMapper> VisitMut for SwitchCaseVisitor<C, S> {
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
