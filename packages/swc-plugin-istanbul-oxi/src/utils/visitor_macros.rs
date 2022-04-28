use once_cell::sync::Lazy;
use regex::Regex as Regexp;
use swc_plugin::ast::*;

/// pattern for istanbul to ignore a section
pub static COMMENT_RE: Lazy<Regexp> =
    Lazy::new(|| Regexp::new(r"^\s*istanbul\s+ignore\s+(if|else|next)(\W|$)").unwrap());

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

/// Interfaces to mark counters. Parent node visitor should pick up and insert marked counter accordingly.
/// Unlike istanbul we can't have single insert logic to be called in any arbitary child node.
#[macro_export]
macro_rules! insert_counter_helper {
    () => {
        fn print_node(&self) -> String {
            if self.nodes.len() > 0 {
                format!(
                    "{}",
                    self.nodes
                        .iter()
                        .map(|n| n.to_string())
                        .collect::<Vec<String>>()
                        .join(":")
                )
            } else {
                "unexpected".to_string()
            }
        }

        #[tracing::instrument(skip(self, span, idx), fields(stmt_id))]
        fn create_stmt_increase_counter_expr(&mut self, span: &Span, idx: Option<u32>) -> Expr {
            let stmt_range = get_range_from_span(self.source_map, span);

            let stmt_id = self.cov.new_statement(&stmt_range);

            tracing::Span::current().record("stmt_id", &stmt_id);

            crate::instrument::create_increase_expression_expr(
                &IDENT_S,
                stmt_id,
                &self.var_name_ident,
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

        // if (path.isExpression()) {
        //    path.replaceWith(T.sequenceExpression([increment, path.node]));
        //}
        #[tracing::instrument(skip_all)]
        fn replace_expr_with_stmt_counter(&mut self, expr: &mut Expr) {
            self.replace_expr_with_counter(expr, |cov, var_name_ident, range| {
                let idx = cov.new_statement(&range);
                create_increase_expression_expr(&IDENT_S, idx, var_name_ident, None)
            });
        }

        #[tracing::instrument(skip_all)]
        fn replace_expr_with_branch_counter(&mut self, expr: &mut Expr, branch: u32) {
            self.replace_expr_with_counter(expr, |cov, var_name_ident, range| {
                let idx = cov.add_branch_path(branch, &range);

                create_increase_expression_expr(&IDENT_B, branch, var_name_ident, Some(idx))
            });
        }

        // Base wrapper fn to replace given expr to wrapped paren expr with counter
        #[tracing::instrument(skip_all)]
        fn replace_expr_with_counter<F>(&mut self, expr: &mut Expr, get_counter: F)
        where
            F: core::ops::Fn(&mut SourceCoverage, &Ident, &istanbul_oxi_instrument::Range) -> Expr,
        {
            let span = get_expr_span(expr);
            if let Some(span) = span {
                let init_range = get_range_from_span(self.source_map, span);
                let prepend_expr = get_counter(&mut self.cov, &self.var_name_ident, &init_range);

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

        // if (path.isBlockStatement()) {
        //    path.node.body.unshift(T.expressionStatement(increment));
        // }
        fn mark_prepend_stmt_counter_for_body(&mut self) {
            todo!("not implemented");
        }

        fn mark_prepend_stmt_counter_for_hoisted(&mut self) {}

        #[tracing::instrument(skip_all)]
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

        fn lookup_hint_comments(&mut self, span: Option<&Span>) -> Option<String> {
            use swc_plugin::comments::Comments;

            if let Some(span) = span {
                let h = self.comments.get_leading(span.hi);
                let l = self.comments.get_leading(span.lo);

                if let Some(h) = h {
                    let h_value = h.iter().find_map(|c| {
                        if let Some(re_match) =
                            crate::utils::visitor_macros::COMMENT_RE.find_at(&c.text, 0)
                        {
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
                        if let Some(re_match) =
                            crate::utils::visitor_macros::COMMENT_RE.find_at(&c.text, 0)
                        {
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

        fn is_injected_counter_expr(&self, expr: &swc_plugin::ast::Expr) -> bool {
            use swc_plugin::ast::*;

            if let Expr::Update(UpdateExpr { arg, .. }) = expr {
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
            false
        }

        /// Determine if given stmt is an injected counter by transform.
        fn is_injected_counter_stmt(&self, stmt: &swc_plugin::ast::Stmt) -> bool {
            use swc_plugin::ast::*;

            if let Stmt::Expr(ExprStmt { expr, .. }) = stmt {
                self.is_injected_counter_expr(&**expr)
            } else {
                false
            }
        }

        fn cover_statement(&mut self, expr: &mut Expr) {
            let span = get_expr_span(expr);
            // This is ugly, poor man's substitute to istanbul's `insertCounter` to determine
            // when to replace givn expr to wrapped Paren or prepend stmt counter.
            // We can't do insert parent node's sibling in downstream's child node.
            // TODO: there should be a better way.
            if let Some(span) = span {
                let mut block = crate::utils::visitor_macros::BlockStmtFinder::new();
                expr.visit_with(&mut block);
                if block.0 {
                    //path.node.body.unshift(T.expressionStatement(increment));
                    self.mark_prepend_stmt_counter(span);
                    return;
                }

                let mut stmt = crate::utils::visitor_macros::StmtFinder::new();
                expr.visit_with(&mut stmt);
                if stmt.0 {
                    //path.insertBefore(T.expressionStatement(increment));
                    self.mark_prepend_stmt_counter(span);
                }

                let mut hoist = crate::utils::visitor_macros::HoistingFinder::new();
                expr.visit_with(&mut hoist);
                let parent = self.nodes.last().unwrap().clone();
                if hoist.0 && parent == Node::VarDeclarator {
                    let parent = self.nodes.get(self.nodes.len() - 3);
                    if let Some(parent) = parent {
                        /*if (parent && T.isExportNamedDeclaration(parent.parentPath)) {
                            parent.parentPath.insertBefore(
                                T.expressionStatement(increment)
                            );
                        }  */
                        let parent = self.nodes.get(self.nodes.len() - 4);
                        if let Some(parent) = parent {
                            match parent {
                                Node::BlockStmt | Node::Program => {
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

                let mut expr_finder = crate::utils::visitor_macros::ExprFinder::new();
                expr.visit_with(&mut expr_finder);
                if expr_finder.0 {
                    self.replace_expr_with_stmt_counter(expr);
                }
            }
        }
    };
}

/// Generate common visitors to visit stmt.
#[macro_export]
macro_rules! visit_mut_coverage {
    () => {
        // BlockStatement: entries(), // ignore processing only
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_block_stmt(&mut self, block_stmt: &mut BlockStmt) {
            self.nodes.push(Node::BlockStmt);

            //self.insert_stmts_counter(&mut block_stmt.stmts);
            block_stmt.visit_mut_children_with(self);
            self.nodes.pop();
        }

        // FunctionDeclaration: entries(coverFunction),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_fn_decl(&mut self, fn_decl: &mut FnDecl) {
            self.nodes.push(Node::FnDecl);
            self.visit_mut_fn(&Some(&fn_decl.ident), &mut fn_decl.function);
            fn_decl.visit_mut_children_with(self);
            self.nodes.pop();
        }

        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
            self.nodes.push(Node::Stmts);
            self.insert_stmts_counter(stmts);
            self.nodes.pop();
        }

        // FunctionExpression: entries(coverFunction),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_fn_expr(&mut self, fn_expr: &mut FnExpr) {
            self.nodes.push(Node::FnExpr);
            // We do insert counter _first_, then iterate child:
            // Otherwise inner stmt / fn will get the first idx to the each counter.
            // StmtVisitor filters out injected counter internally.
            self.visit_mut_fn(&fn_expr.ident.as_ref(), &mut fn_expr.function);
            fn_expr.visit_mut_children_with(self);
            self.nodes.pop();
        }

        // ExpressionStatement: entries(coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_expr_stmt(&mut self, expr_stmt: &mut ExprStmt) {
            self.nodes.push(Node::ExprStmt);

            if !self.is_injected_counter_expr(&*expr_stmt.expr) {
                self.mark_prepend_stmt_counter(&expr_stmt.span);
            }
            expr_stmt.visit_mut_children_with(self);

            self.nodes.pop();
        }

        // ReturnStatement: entries(coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_return_stmt(&mut self, return_stmt: &mut ReturnStmt) {
            self.nodes.push(Node::ReturnStmt);
            self.mark_prepend_stmt_counter(&return_stmt.span);
            return_stmt.visit_mut_children_with(self);
            self.nodes.pop();
        }

        // VariableDeclarator: entries(coverVariableDeclarator),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_var_declarator(&mut self, declarator: &mut VarDeclarator) {
            self.nodes.push(Node::VarDeclarator);

            if let Some(init) = &mut declarator.init {
                let init = &mut **init;
                self.cover_statement(init);
            }

            declarator.visit_mut_children_with(self);
            self.nodes.pop();
        }

        // ForStatement: entries(blockProp('body'), coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_for_stmt(&mut self, for_stmt: &mut ForStmt) {
            self.nodes.push(Node::ForStmt);

            self.mark_prepend_stmt_counter(&for_stmt.span);

            let mut body = *for_stmt.body.take();
            // if for stmt body is not block, wrap it before insert statement counter
            let body = if let Stmt::Block(mut body) = body {
                //self.insert_stmts_counter(&mut body.stmts);
                body
            } else {
                let mut stmts = vec![body];
                //self.insert_stmts_counter(&mut stmts);

                BlockStmt {
                    span: DUMMY_SP,
                    stmts,
                }
            };

            for_stmt.body = Box::new(Stmt::Block(body));
            for_stmt.visit_mut_children_with(self);

            self.nodes.pop();
        }
    };
}

#[derive(Debug)]
pub struct HoistingFinder(pub bool);

impl HoistingFinder {
    pub fn new() -> HoistingFinder {
        HoistingFinder(false)
    }
}

impl Visit for HoistingFinder {
    fn visit_fn_expr(&mut self, fn_expr: &FnExpr) {
        self.0 = true;
    }

    fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr) {
        self.0 = true;
    }

    fn visit_class_expr(&mut self, class_expr: &ClassExpr) {
        self.0 = true;
    }
}

#[derive(Debug)]
pub struct BlockStmtFinder(pub bool);

impl BlockStmtFinder {
    pub fn new() -> BlockStmtFinder {
        BlockStmtFinder(false)
    }
}

impl Visit for BlockStmtFinder {
    fn visit_block_stmt(&mut self, block: &BlockStmt) {
        self.0 = true;
    }
}

#[derive(Debug)]
pub struct StmtFinder(pub bool);

impl StmtFinder {
    pub fn new() -> StmtFinder {
        StmtFinder(false)
    }
}

impl Visit for StmtFinder {
    fn visit_stmt(&mut self, block: &Stmt) {
        self.0 = true;
    }
}

#[derive(Debug)]
pub struct ExprFinder(pub bool);

impl ExprFinder {
    pub fn new() -> ExprFinder {
        ExprFinder(false)
    }
}

impl Visit for ExprFinder {
    fn visit_expr(&mut self, block: &Expr) {
        self.0 = true;
    }
}
