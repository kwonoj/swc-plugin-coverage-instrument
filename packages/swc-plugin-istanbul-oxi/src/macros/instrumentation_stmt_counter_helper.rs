/// Create a fn inserts stmt counter for each stmt
#[macro_export]
macro_rules! instrumentation_stmt_counter_helper {
    () => {
        /// Visit individual statements with stmt_visitor and update.
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn insert_stmts_counter(&mut self, stmts: &mut Vec<Stmt>) {
            let mut new_stmts = vec![];

            for mut stmt in stmts.drain(..) {
                if !self.is_injected_counter_stmt(&stmt) {
                    let (old, ignore_current) = self.on_enter(&mut stmt);

                    match ignore_current {
                        Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                        _ => {
                            let mut visitor = crate::visitors::stmt_like_visitor::StmtVisitor::new(
                                self.source_map,
                                self.comments,
                                &mut self.cov,
                                &self.instrument_options,
                                &self.nodes,
                                ignore_current,
                            );
                            stmt.visit_mut_children_with(&mut visitor);

                            new_stmts.extend(visitor.before.drain(..));
                        }
                    }
                    self.on_exit(old);
                }

                new_stmts.push(stmt);
            }

            *stmts = new_stmts;
        }
    };
}
