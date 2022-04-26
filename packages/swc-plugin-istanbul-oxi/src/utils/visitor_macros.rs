/// A macro wraps a visitor fn create a statement AST to increase statement counter.
/// Created statement is stored in `before` property in the CoverageVisitor, will be prepended
/// via visit_mut_module_items.
#[macro_export]
macro_rules! visit_mut_prepend_statement_counter {
    ($name:ident, $N:tt) => {
        #[inline]
        #[instrument(skip_all)]
        fn $name(&mut self, n: &mut swc_plugin::ast::$N) {
            self.nodes.push(Node::$N);
            //self.insert_stmt_counter_will_this_work();
            n.visit_mut_children_with(self);
            self.nodes.pop();
        }
    };
}
