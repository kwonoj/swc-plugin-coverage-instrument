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

#[macro_export]
macro_rules! insert_counter_helper {
    () => {
        fn visit_mut_fn(&mut self, ident: &Option<&Ident>, function: &mut Function) {
            let (span, name) = if let Some(ident) = &ident {
                (&ident.span, Some(ident.sym.to_string()))
            } else {
                (&function.span, None)
            };

            let range = get_range_from_span(self.source_map, span);
            let body_span = if let Some(body) = &function.body {
                body.span
            } else {
                // TODO: probably this should never occur
                function.span
            };

            let body_range = get_range_from_span(self.source_map, &body_span);
            let index = self.cov.new_function(&name, &range, &body_range);

            match &mut function.body {
                Some(blockstmt) => {
                    let b = create_increase_expression_expr(
                        &IDENT_F,
                        index,
                        &self.var_name_ident,
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

        /// Visit individual statements with stmt_visitor and update.
        fn insert_stmts_counter(&mut self, stmts: &mut Vec<Stmt>) {
            let mut new_stmts = vec![];

            for mut stmt in stmts.drain(..) {
                if !self.is_injected_counter_stmt(&stmt) {
                    let span = get_stmt_span(&stmt);
                    if let Some(span) = span {
                        let increment_expr = self.create_stmt_increase_counter_expr(span, None);

                        new_stmts.push(Stmt::Expr(ExprStmt {
                            span: DUMMY_SP,
                            expr: Box::new(increment_expr),
                        }));
                    } else {
                        // if given stmt is not a plain stmt and omit to insert stmt counter,
                        // visit it to collect inner stmt counters
                        stmt.visit_mut_with(self);
                        // Once visit completes, pick up stmt counter immediately - otherwise parent visitor will
                        // place this incorrect position outside of current scope.
                        // TODO: should we use new visitor instead? or should we need different storage property
                        // for better clarity?
                        if let Some(last) = self.before.pop() {
                            new_stmts.push(last);
                        }
                    }
                }

                new_stmts.push(stmt);
            }

            *stmts = new_stmts;
        }

        fn lookup_hint_comments(&mut self, span: Option<&Span>) -> Option<String> {
            if let Some(span) = span {
                let h = self.comments.get_leading(span.hi);
                let l = self.comments.get_leading(span.lo);

                if let Some(h) = h {
                    let h_value = h.iter().find_map(|c| {
                        if let Some(re_match) = COMMENT_RE.find_at(&c.text, 0) {
                            Some(re_match.as_str().to_string())
                        } else {
                            None
                        }
                    });

                    if let Some(h_value) = h_value {
                        return Some(h_value);
                    }
                }

                if let Some(l) = l {
                    let l_value = l.iter().find_map(|c| {
                        if let Some(re_match) = COMMENT_RE.find_at(&c.text, 0) {
                            Some(re_match.as_str().to_string())
                        } else {
                            None
                        }
                    });

                    if let Some(l_value) = l_value {
                        return Some(l_value);
                    }
                }
            }

            return None;
        }

        /// Determine if given stmt is an injected counter by transform.
        fn is_injected_counter_stmt(&self, stmt: &swc_plugin::ast::Stmt) -> bool {
            use swc_plugin::ast::*;

            if let Stmt::Expr(ExprStmt { expr, .. }) = stmt {
                if let Expr::Update(UpdateExpr { arg, .. }) = &**expr {
                    if let Expr::Member(MemberExpr { obj, .. }) = &**arg {
                        if let Expr::Member(MemberExpr { obj, .. }) = &**obj {
                            if let Expr::Call(CallExpr { callee, .. }) = &**obj {
                                if let Callee::Expr(expr) = callee {
                                    if let Expr::Ident(ident) = &**expr {
                                        if ident == &self.var_name_ident {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                };
            }
            false
        }
    };
}

/// Generate common visitors to visit stmt.
#[macro_export]
macro_rules! visit_mut_stmt_like {
    () => {};
}
