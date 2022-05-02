pub(crate) const DIRECTIVES: &[&str] = &["use strict", "use asm", "use strong"];

/// Generate common visitors to visit stmt.
#[macro_export]
macro_rules! instrumentation_visitor {
    () => {
        noop_visit_mut_type!();

        // BlockStatement: entries(), // ignore processing only
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_block_stmt(&mut self, block_stmt: &mut BlockStmt) {
            let (old, ignore_current) = self.on_enter(block_stmt);
            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    // Visit inner for the block stmt
                    block_stmt.visit_mut_children_with(self);
                }
            }
            self.on_exit(old);
        }

        // FunctionDeclaration: entries(coverFunction),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_fn_decl(&mut self, fn_decl: &mut FnDecl) {
            let (old, ignore_current) = self.on_enter(fn_decl);
            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    self.create_fn_instrumentation(&Some(&fn_decl.ident), &mut fn_decl.function);
                    fn_decl.visit_mut_children_with(self);
                }
            }
            self.on_exit(old);
        }

        // ArrowFunctionExpression: entries(convertArrowExpression, coverFunction),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_arrow_expr(&mut self, arrow_expr: &mut ArrowExpr) {
            let (old, ignore_current) = self.on_enter(arrow_expr);
            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                _ => match &mut arrow_expr.body {
                    BlockStmtOrExpr::BlockStmt(block_stmt) => {
                        let range = get_range_from_span(self.source_map, &arrow_expr.span);
                        let body_range = get_range_from_span(self.source_map, &block_stmt.span);
                        let index = self.cov.new_function(&None, &range, &body_range);
                        let b = create_increase_expression_expr(
                            &IDENT_F,
                            index,
                            &self.cov_fn_ident,
                            None,
                        );

                        // insert fn counter expression
                        let mut new_stmts = vec![Stmt::Expr(ExprStmt {
                            span: DUMMY_SP,
                            expr: Box::new(b),
                        })];
                        // if arrow fn body is already blockstmt, insert stmt counter for each
                        self.insert_stmts_counter(&mut block_stmt.stmts);
                        new_stmts.extend(block_stmt.stmts.drain(..));
                        block_stmt.stmts = new_stmts;
                    }
                    BlockStmtOrExpr::Expr(expr) => {
                        // TODO: refactor common logics creates a blockstmt from single expr
                        let range = get_range_from_span(self.source_map, &arrow_expr.span);
                        let span = get_expr_span(expr);
                        if let Some(span) = span {
                            let body_range = get_range_from_span(self.source_map, &span);
                            let index = self.cov.new_function(&None, &range, &body_range);
                            let b = create_increase_expression_expr(
                                &IDENT_F,
                                index,
                                &self.cov_fn_ident,
                                None,
                            );

                            // insert fn counter expression
                            let mut stmts = vec![Stmt::Expr(ExprStmt {
                                span: DUMMY_SP,
                                expr: Box::new(b),
                            })];

                            // single line expr in arrow fn need to be converted into return stmt
                            // Note we should preserve original expr's span, otherwise statementmap will lose correct
                            // code location
                            let ret = Stmt::Return(ReturnStmt {
                                span: span.clone(),
                                arg: Some(expr.take()),
                            });
                            stmts.push(ret);

                            let mut new_stmts = vec![];
                            // insert stmt counter for the returnstmt we made above
                            self.insert_stmts_counter(&mut stmts);
                            new_stmts.extend(stmts.drain(..));

                            arrow_expr.body = BlockStmtOrExpr::BlockStmt(BlockStmt {
                                span: DUMMY_SP,
                                stmts: new_stmts,
                            });
                        }
                    }
                },
            }
            self.on_exit(old);
        }

        /*
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_stmt(&mut self, stmt: &mut Stmt) {
            if !self.is_injected_counter_stmt(stmt) {
                let span = crate::utils::lookup_range::get_stmt_span(&stmt);
                if let Some(span) = span {
                    let increment_expr = self.create_stmt_increase_counter_expr(span, None);

                    self.before.push(Stmt::Expr(ExprStmt {
                        span: DUMMY_SP,
                        expr: Box::new(increment_expr),
                    }));
                }
            }
        } */

        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
            // Each Stmt looks up own comments for the hint, we don't
            // do self.on_enter() in here.
            self.nodes.push(Node::Stmts);
            self.insert_stmts_counter(stmts);
            self.nodes.pop();
        }

        // FunctionExpression: entries(coverFunction),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_fn_expr(&mut self, fn_expr: &mut FnExpr) {
            let (old, ignore_current) = self.on_enter(fn_expr);
            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    let fn_ident = &fn_expr.ident.as_ref();

                    let should_ignore_via_options = if let Some(ident) = fn_ident {
                        self.instrument_options
                            .ignore_class_methods
                            .iter()
                            .any(|v| v.as_str() == &*ident.sym)
                    } else {
                        false
                    };

                    if !should_ignore_via_options {
                        // We do insert counter _first_, then iterate child:
                        // Otherwise inner stmt / fn will get the first idx to the each counter.
                        // StmtVisitor filters out injected counter internally.
                        self.create_fn_instrumentation(&fn_ident, &mut fn_expr.function);
                        fn_expr.visit_mut_children_with(self);
                    }
                }
            }
            self.on_exit(old);
        }

        // ExpressionStatement: entries(coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_expr_stmt(&mut self, expr_stmt: &mut ExprStmt) {
            let (old, ignore_current) = self.on_enter(expr_stmt);

            if let Expr::Lit(Lit::Str(Str { value, .. })) = expr_stmt.expr.as_ref() {
                let value: &str = &*value;

                if crate::macros::instrumentation_visitor::DIRECTIVES.contains(&value) {
                    self.on_exit(old);
                    return;
                }
            }

            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    if !self.is_injected_counter_expr(&*expr_stmt.expr) {
                        self.mark_prepend_stmt_counter(&expr_stmt.span);
                    }
                }
            }
            expr_stmt.visit_mut_children_with(self);

            self.on_exit(old);
        }

        // BreakStatement: entries(coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_break_stmt(&mut self, break_stmt: &mut BreakStmt) {
            let (old, ignore_current) = self.on_enter(break_stmt);

            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    self.mark_prepend_stmt_counter(&break_stmt.span);
                }
            }
            break_stmt.visit_mut_children_with(self);

            self.on_exit(old);
        }

        // ReturnStatement: entries(coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_return_stmt(&mut self, return_stmt: &mut ReturnStmt) {
            let (old, ignore_current) = self.on_enter(return_stmt);
            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    self.mark_prepend_stmt_counter(&return_stmt.span);
                    return_stmt.visit_mut_children_with(self);
                }
            }

            self.on_exit(old);
        }

        // VariableDeclaration: entries(), // ignore processing only
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_var_decl(&mut self, var_decl: &mut VarDecl) {
            let (old, _ignore_current) = self.on_enter(var_decl);
            //noop?
            var_decl.visit_mut_children_with(self);
            self.on_exit(old);
        }

        // ClassDeclaration: entries(parenthesizedExpressionProp('superClass')),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_class_decl(&mut self, class_decl: &mut ClassDecl) {
            let (old, ignore_current) = self.on_enter(class_decl);
            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    //self.mark_prepend_stmt_counter(&class_decl.class.span);
                    class_decl.visit_mut_children_with(self);
                }
            }

            self.on_exit(old);
        }

        // ClassProperty: entries(coverClassPropDeclarator),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_class_prop(&mut self, class_prop: &mut ClassProp) {
            let (old, ignore_current) = self.on_enter(class_prop);
            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    if let Some(value) = &mut class_prop.value {
                        self.cover_statement(&mut *value);
                    }
                }
            }
            self.on_exit(old);
        }

        // ClassPrivateProperty: entries(coverClassPropDeclarator),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_private_prop(&mut self, private_prop: &mut PrivateProp) {
            // TODO: this is same as visit_mut_class_prop
            let (old, ignore_current) = self.on_enter(private_prop);
            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    if let Some(value) = &mut private_prop.value {
                        self.cover_statement(&mut *value);
                    }
                }
            }
            self.on_exit(old);
        }

        // ClassMethod: entries(coverFunction),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_class_method(&mut self, class_method: &mut ClassMethod) {
            let (old, ignore_current) = self.on_enter(class_method);
            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    // TODO: this does not cover all of PropName enum yet
                    // TODO: diplicated logic between fn_expr
                    if let PropName::Ident(ident) = &class_method.key {
                        let should_ignore_via_options = self
                            .instrument_options
                            .ignore_class_methods
                            .iter()
                            .any(|v| v.as_str() == &*ident.sym);

                        if !should_ignore_via_options {
                            self.create_fn_instrumentation(
                                &Some(&ident),
                                &mut class_method.function,
                            );
                            class_method.visit_mut_children_with(self);
                        }
                    }
                }
            }
            self.on_exit(old);
        }

        // VariableDeclarator: entries(coverVariableDeclarator),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_var_declarator(&mut self, declarator: &mut VarDeclarator) {
            let (old, ignore_current) = self.on_enter(declarator);

            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    if let Some(init) = &mut declarator.init {
                        let init = &mut **init;
                        self.cover_statement(init);
                    }

                    declarator.visit_mut_children_with(self);
                }
            }

            self.on_exit(old);
        }

        // ForStatement: entries(blockProp('body'), coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_for_stmt(&mut self, for_stmt: &mut ForStmt) {
            crate::visit_mut_for_like!(self, for_stmt);
        }

        // ForInStatement: entries(blockProp('body'), coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_for_in_stmt(&mut self, for_in_stmt: &mut ForInStmt) {
            crate::visit_mut_for_like!(self, for_in_stmt);
        }

        // ForOfStatement: entries(blockProp('body'), coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_for_of_stmt(&mut self, for_of_stmt: &mut ForOfStmt) {
            crate::visit_mut_for_like!(self, for_of_stmt);
        }

        // WhileStatement: entries(blockProp('body'), coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_while_stmt(&mut self, while_stmt: &mut WhileStmt) {
            crate::visit_mut_for_like!(self, while_stmt);
        }

        // DoWhileStatement: entries(blockProp('body'), coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_do_while_stmt(&mut self, do_while_stmt: &mut DoWhileStmt) {
            crate::visit_mut_for_like!(self, do_while_stmt);
        }

        //LabeledStatement: entries(coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_labeled_stmt(&mut self, labeled_stmt: &mut LabeledStmt) {
            let (old, ignore_current) = self.on_enter(labeled_stmt);

            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    // cover_statement's is_stmt prepend logic for individual child stmt visitor
                    self.mark_prepend_stmt_counter(&labeled_stmt.span);
                }
            }

            labeled_stmt.visit_mut_children_with(self);

            self.on_exit(old);
        }

        // ContinueStatement: entries(coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_continue_stmt(&mut self, continue_stmt: &mut ContinueStmt) {
            let (old, ignore_current) = self.on_enter(continue_stmt);

            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    // cover_statement's is_stmt prepend logic for individual child stmt visitor
                    self.mark_prepend_stmt_counter(&continue_stmt.span);
                }
            }

            continue_stmt.visit_mut_children_with(self);
            self.on_exit(old);
        }

        // SwitchStatement: entries(createSwitchBranch, coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_switch_stmt(&mut self, switch_stmt: &mut SwitchStmt) {
            let (old, ignore_current) = self.on_enter(switch_stmt);
            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    // Insert stmt counter for `switch` itself, then create a new branch
                    self.mark_prepend_stmt_counter(&switch_stmt.span);

                    let range = get_range_from_span(self.source_map, &switch_stmt.span);
                    let branch = self.cov.new_branch(
                        istanbul_oxi_instrument::BranchType::Switch,
                        &range,
                        false,
                    );

                    // traverse `case` with a visitor contains branch idx, insert new
                    // branch increase counter accordingly
                    let mut visitor = crate::visitors::switch_case_visitor::SwitchCaseVisitor::new(
                        self.source_map,
                        self.comments,
                        &mut self.cov,
                        &self.instrument_options,
                        &self.nodes,
                        ignore_current,
                        branch,
                    );

                    switch_stmt.visit_mut_children_with(&mut visitor);
                }
            }
            self.on_exit(old);
        }

        // IfStatement: entries(blockProp('consequent'), blockProp('alternate'), coverStatement, coverIfBranches)
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_if_stmt(&mut self, if_stmt: &mut IfStmt) {
            let (old, ignore_current) = self.on_enter(if_stmt);

            match ignore_current {
                Some(crate::utils::hint_comments::IgnoreScope::Next) => {
                    self.on_exit(old);
                }
                _ => {
                    // cover_statement's is_stmt prepend logic for individual child stmt visitor
                    self.mark_prepend_stmt_counter(&if_stmt.span);

                    let range = get_range_from_span(self.source_map, &if_stmt.span);
                    let branch =
                        self.cov
                            .new_branch(istanbul_oxi_instrument::BranchType::If, &range, false);

                    let mut wrap_with_counter = |stmt: &mut Box<Stmt>| {
                        let mut stmt_body = *stmt.take();

                        // create a branch path counter
                        let idx = self.cov.add_branch_path(branch, &range);
                        let expr = create_increase_expression_expr(
                            &IDENT_B,
                            branch,
                            &self.cov_fn_ident,
                            Some(idx),
                        );

                        let expr = Stmt::Expr(ExprStmt {
                            span: DUMMY_SP,
                            expr: Box::new(expr),
                        });

                        let body = if let Stmt::Block(mut block_stmt) = stmt_body {
                            // if cons / alt is already blockstmt, insert stmt counter for each
                            self.insert_stmts_counter(&mut block_stmt.stmts);

                            let mut new_stmts = vec![expr];
                            new_stmts.extend(block_stmt.stmts.drain(..));

                            block_stmt.stmts = new_stmts;
                            block_stmt
                        } else {
                            let mut stmts = vec![expr];
                            let mut visitor = crate::visitors::stmt_like_visitor::StmtVisitor::new(
                                self.source_map,
                                self.comments,
                                &mut self.cov,
                                &self.instrument_options,
                                &self.nodes,
                                ignore_current,
                            );
                            stmt_body.visit_mut_with(&mut visitor);
                            stmts.extend(visitor.before.drain(..));

                            stmts.push(stmt_body);

                            BlockStmt {
                                span: DUMMY_SP,
                                stmts,
                            }
                        };

                        *stmt = Box::new(Stmt::Block(body));
                    };

                    if ignore_current == Some(crate::utils::hint_comments::IgnoreScope::If) {
                        //setAttr(if_stmt.cons, 'skip-all', true);
                    } else {
                        wrap_with_counter(&mut if_stmt.cons);
                    }

                    if ignore_current == Some(crate::utils::hint_comments::IgnoreScope::Else) {
                        //setAttr(if_stmt.alt, 'skip-all', true);
                    } else {
                        if let Some(alt) = &mut if_stmt.alt {
                            wrap_with_counter(alt);
                        } else {
                            // alt can be none (`if some {}` without else).
                            // Inject empty blockstmt then insert branch counters
                            let mut alt = Box::new(Stmt::Block(BlockStmt::dummy()));
                            wrap_with_counter(&mut alt);
                            if_stmt.alt = Some(alt);

                            // We visit individual cons / alt depends on its state, need to run visitor for the `test` as well
                            if_stmt.test.visit_mut_with(self);

                            self.on_exit(old);
                            return;
                        }
                    }

                    // We visit individual cons / alt depends on its state, need to run visitor for the `test` as well
                    if_stmt.test.visit_mut_with(self);

                    self.on_exit(old);
                }
            };
        }

        // LogicalExpression: entries(coverLogicalExpression)
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
                        BinaryOp::LogicalOr
                        | BinaryOp::LogicalAnd
                        | BinaryOp::NullishCoalescing => {
                            self.nodes.push(Node::LogicalExpr);

                            // Create a new branch. This id should be reused for any inner logical expr.
                            let range = get_range_from_span(self.source_map, &bin_expr.span);
                            let branch = self.cov.new_branch(
                                istanbul_oxi_instrument::BranchType::BinaryExpr,
                                &range,
                                self.instrument_options.report_logic,
                            );

                            // Iterate over each expr, wrap it with branch counter.
                            self.wrap_bin_expr_with_branch_counter(branch, &mut *bin_expr.left);
                            self.wrap_bin_expr_with_branch_counter(branch, &mut *bin_expr.right);
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
    };
}
