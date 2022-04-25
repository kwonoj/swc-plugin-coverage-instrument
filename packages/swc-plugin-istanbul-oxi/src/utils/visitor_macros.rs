/// A macro wraps a visitor fn create a statement AST to increase statement counter.
/// Created statement is stored in `before` property in the CoverageVisitor, will be prepended
/// via visit_mut_module_items.
#[macro_export]
macro_rules! visit_mut_prepend_statement_counter {
    ($name:ident, $N:tt) => {
        #[inline]
        fn $name(&mut self, n: &mut swc_plugin::ast::$N) {
            let stmt_range = get_range_from_span(self.source_map, &n.span);

            let idx = self.cov.new_statement(&stmt_range);
            let increment_expr = crate::instrument::build_increase_expression_expr(
                &IDENT_S,
                idx,
                &self.var_name_ident,
                None,
            );

            self.before.push(Stmt::Expr(ExprStmt {
                span: DUMMY_SP,
                expr: Box::new(increment_expr),
            }));

            n.visit_mut_children_with(self);
        }
    };
}
