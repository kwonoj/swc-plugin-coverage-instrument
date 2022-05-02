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
                    let span = crate::utils::lookup_range::get_stmt_span(&stmt);

                    let should_ignore =
                        crate::utils::hint_comments::should_ignore(&self.comments, span);

                    let mut visitor = StmtVisitor::new(
                        self.source_map,
                        self.comments,
                        &mut self.cov,
                        &self.instrument_options,
                        &self.nodes,
                        should_ignore,
                    );
                    stmt.visit_mut_children_with(&mut visitor);

                    if visitor.before.len() == 0 {
                        //println!("{:#?}", stmt);
                    }

                    new_stmts.extend(visitor.before.drain(..));

                    /*
                    if let Some(span) = span {
                        // if given stmt is not a plain stmt and omit to insert stmt counter,
                        // visit it to collect inner stmt counters


                    } else {
                        //stmt.visit_mut_children_with(self);
                        //new_stmts.extend(visitor.before.drain(..));
                    } */
                }

                new_stmts.push(stmt);
            }

            *stmts = new_stmts;
        }
    };
}
