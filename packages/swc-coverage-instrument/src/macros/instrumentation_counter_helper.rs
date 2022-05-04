#[macro_export]
macro_rules! instrumentation_branch_wrap_counter_helper {
    () => {
        // if (path.isExpression()) {
        //    path.replaceWith(T.sequenceExpression([increment, path.node]));
        //}
        #[tracing::instrument(skip_all)]
        fn replace_expr_with_stmt_counter(&mut self, expr: &mut Expr) {
            self.replace_expr_with_counter(expr, |cov, cov_fn_ident, range| {
                let idx = cov.new_statement(&range);
                crate::create_increase_counter_expr(
                    &crate::constants::idents::IDENT_S,
                    idx,
                    cov_fn_ident,
                    None,
                )
            });
        }

        #[tracing::instrument(skip_all)]
        fn replace_expr_with_branch_counter(&mut self, expr: &mut Expr, branch: u32) {
            self.replace_expr_with_counter(expr, |cov, cov_fn_ident, range| {
                let idx = cov.add_branch_path(branch, &range);

                crate::create_increase_counter_expr(
                    &crate::constants::idents::IDENT_B,
                    branch,
                    cov_fn_ident,
                    Some(idx),
                )
            });
        }

        // Base wrapper fn to replace given expr to wrapped paren expr with counter
        #[tracing::instrument(skip_all)]
        fn replace_expr_with_counter<F>(&mut self, expr: &mut Expr, get_counter: F)
        where
            F: core::ops::Fn(
                &mut crate::source_coverage::SourceCoverage,
                &Ident,
                &crate::Range,
            ) -> Expr,
        {
            let span = crate::lookup_range::get_expr_span(expr);
            if let Some(span) = span {
                let init_range = crate::lookup_range::get_range_from_span(&self.source_map, span);
                let prepend_expr =
                    get_counter(&mut self.cov.borrow_mut(), &self.cov_fn_ident, &init_range);

                let paren_expr = Expr::Paren(ParenExpr {
                    span: DUMMY_SP,
                    expr: Box::new(Expr::Seq(SeqExpr {
                        span: DUMMY_SP,
                        exprs: vec![Box::new(prepend_expr), Box::new(expr.take())],
                    })),
                });

                // replace init with increase expr + init seq
                *expr = paren_expr;
            }
        }

        /// Attempt to wrap expression with branch increase counter.
        /// Given Expr may be left, or right of the logical expression.
        #[tracing::instrument(skip_all)]
        fn wrap_bin_expr_with_branch_counter(&mut self, branch: u32, expr: &mut Expr) {
            let span = crate::lookup_range::get_expr_span(expr);
            let should_ignore = crate::hint_comments::should_ignore(&self.comments, span);

            if let Some(crate::hint_comments::IgnoreScope::Next) = should_ignore {
                return;
            }

            // Logical expression can have inner logical expression as non-direct child
            // (i.e `args[0] > 0 && (args[0] < 5 || args[0] > 10)`, logical || expr is child of ParenExpr.
            // Try to look up if current expr is the `leaf` of whole logical expr tree.
            let mut has_inner_logical_expr = crate::visitors::finders::LogicalExprLeafFinder(false);
            expr.visit_with(&mut has_inner_logical_expr);

            // If current expr have inner logical expr, traverse until reaches to the leaf
            if has_inner_logical_expr.0 {
                let mut visitor = crate::visitors::logical_expr_visitor::LogicalExprVisitor::new(
                    self.source_map.clone(),
                    self.comments.clone(),
                    self.cov.clone(),
                    self.instrument_options.clone(),
                    self.nodes.clone(),
                    should_ignore,
                    branch,
                );

                expr.visit_mut_children_with(&mut visitor);
            } else {
                // Now we believe this expr is the leaf of the logical expr tree.
                // Wrap it with branch counter.
                if self.instrument_options.report_logic {
                    if let Some(span) = span {
                        let range =
                            crate::lookup_range::get_range_from_span(&self.source_map, span);
                        let branch_path_index =
                            self.cov.borrow_mut().add_branch_path(branch, &range);

                        let increase_expr = crate::create_increase_counter_expr(
                            &crate::constants::idents::IDENT_B,
                            branch,
                            &self.cov_fn_ident,
                            Some(branch_path_index),
                        );
                        let increase_true_expr = crate::create_increase_true_expr(
                            branch,
                            branch_path_index,
                            &self.cov_fn_ident,
                            &self.cov_fn_temp_ident,
                            expr.take(),
                        );

                        // TODO: duplicated code with replace_expr_with_counter
                        let paren_expr = Expr::Seq(SeqExpr {
                            span: DUMMY_SP,
                            exprs: vec![Box::new(increase_expr), Box::new(increase_true_expr)],
                        });

                        // replace init with increase expr + init seq
                        *expr = paren_expr;
                    }
                } else {
                    self.replace_expr_with_branch_counter(expr, branch);
                }
            }
        }
    };
}

/// Interfaces to mark counters. Parent node visitor should pick up and insert marked counter accordingly.
/// Unlike istanbul we can't have single insert logic to be called in any arbitary child node.
#[macro_export]
macro_rules! instrumentation_counter_helper {
    () => {
        crate::instrumentation_branch_wrap_counter_helper!();

        #[tracing::instrument(skip(self, span, idx), fields(stmt_id))]
        fn create_stmt_increase_counter_expr(&mut self, span: &Span, idx: Option<u32>) -> Expr {
            let stmt_range = crate::lookup_range::get_range_from_span(&self.source_map, span);

            let stmt_id = self.cov.borrow_mut().new_statement(&stmt_range);

            tracing::Span::current().record("stmt_id", &stmt_id);

            crate::create_increase_counter_expr(
                &crate::constants::idents::IDENT_S,
                stmt_id,
                &self.cov_fn_ident,
                idx,
            )
        }

        // Mark to prepend statement increase counter to current stmt.
        // if (path.isStatement()) {
        //    path.insertBefore(T.expressionStatement(increment));
        // }
        #[tracing::instrument(skip_all)]
        fn mark_prepend_stmt_counter(&mut self, span: &Span) {
            let increment_expr = self.create_stmt_increase_counter_expr(span, None);
            self.before.push(Stmt::Expr(ExprStmt {
                span: DUMMY_SP,
                expr: Box::new(increment_expr),
            }));
        }

        /// Common logics for the fn-like visitors to insert fn instrumentation counters.
        #[tracing::instrument(skip_all)]
        fn create_fn_instrumentation(&mut self, ident: &Option<&Ident>, function: &mut Function) {
            let (span, name) = if let Some(ident) = &ident {
                (&ident.span, Some(ident.sym.to_string()))
            } else {
                (&function.span, None)
            };

            let range = crate::lookup_range::get_range_from_span(&self.source_map, span);
            let body_span = if let Some(body) = &function.body {
                body.span
            } else {
                // TODO: probably this should never occur
                function.span
            };

            let body_range = crate::lookup_range::get_range_from_span(&self.source_map, &body_span);
            let index = self
                .cov
                .borrow_mut()
                .new_function(&name, &range, &body_range);

            match &mut function.body {
                Some(blockstmt) => {
                    let b = crate::create_increase_counter_expr(
                        &crate::constants::idents::IDENT_F,
                        index,
                        &self.cov_fn_ident,
                        None,
                    );
                    let mut prepended_vec = vec![Stmt::Expr(ExprStmt {
                        span: DUMMY_SP,
                        expr: Box::new(b),
                    })];
                    prepended_vec.extend(blockstmt.stmts.take());
                    blockstmt.stmts = prepended_vec;
                }
                _ => {
                    unimplemented!("Unable to process function body node type")
                }
            }
        }

        fn is_injected_counter_expr(&self, expr: &Expr) -> bool {
            if let Expr::Update(UpdateExpr { arg, .. }) = expr {
                if let Expr::Member(MemberExpr { obj, .. }) = &**arg {
                    if let Expr::Member(MemberExpr { obj, .. }) = &**obj {
                        if let Expr::Call(CallExpr { callee, .. }) = &**obj {
                            if let Callee::Expr(expr) = callee {
                                if let Expr::Ident(ident) = &**expr {
                                    if ident == &self.cov_fn_ident {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            };
            false
        }

        /// Determine if given stmt is an injected counter by transform.
        fn is_injected_counter_stmt(&self, stmt: &Stmt) -> bool {
            if let Stmt::Expr(ExprStmt { expr, .. }) = stmt {
                self.is_injected_counter_expr(&**expr)
            } else {
                false
            }
        }

        fn cover_statement(&mut self, expr: &mut Expr) {
            let span = crate::lookup_range::get_expr_span(expr);
            // This is ugly, poor man's substitute to istanbul's `insertCounter` to determine
            // when to replace givn expr to wrapped Paren or prepend stmt counter.
            // We can't do insert parent node's sibling in downstream's child node.
            // TODO: there should be a better way.
            if let Some(span) = span {
                let mut block = crate::visitors::finders::BlockStmtFinder::new();
                expr.visit_with(&mut block);
                // TODO: this may not required as visit_mut_block_stmt recursively visits inner instead.
                if block.0 {
                    //path.node.body.unshift(T.expressionStatement(increment));
                    self.mark_prepend_stmt_counter(span);
                    return;
                }

                let mut stmt = crate::visitors::finders::StmtFinder::new();
                expr.visit_with(&mut stmt);
                if stmt.0 {
                    //path.insertBefore(T.expressionStatement(increment));
                    self.mark_prepend_stmt_counter(span);
                }

                let mut hoist = crate::visitors::finders::HoistingFinder::new();
                expr.visit_with(&mut hoist);
                let parent = self.nodes.last().unwrap().clone();
                if hoist.0 && parent == crate::Node::VarDeclarator {
                    // TODO: need to polish logic to determine prepending instead of traversing parent node types
                    let nodes_len = self.nodes.len();
                    let parent_index = if nodes_len >= (3 as usize) {
                        nodes_len - 3
                    } else {
                        0
                    };
                    let parent = self.nodes.get(parent_index);

                    if parent.is_some() {
                        let parent_index = if nodes_len >= (4 as usize) {
                            nodes_len - 4
                        } else {
                            0
                        };
                        let parent = self.nodes.get(parent_index);

                        if let Some(parent) = parent {
                            match parent {
                                crate::Node::BlockStmt | crate::Node::Program => {
                                    self.mark_prepend_stmt_counter(span);
                                }
                                _ => {}
                            }
                        }
                    } else {
                        self.replace_expr_with_stmt_counter(expr);
                    }

                    return;
                }

                let mut expr_finder = crate::visitors::finders::ExprFinder::new();
                expr.visit_with(&mut expr_finder);
                if expr_finder.0 {
                    self.replace_expr_with_stmt_counter(expr);
                }
            }
        }
    };
}
