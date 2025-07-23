pub(crate) const DIRECTIVES: &[&str] = &["use strict", "use asm", "use strong"];

/// Generate common visitors to visit stmt.
#[macro_export]
macro_rules! instrumentation_visitor {
    () => {
        noop_visit_mut_type!();

        // BlockStatement: entries(), // ignore processing only
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_block_stmt(&mut self, block_stmt: &mut BlockStmt) {
            let (old, ignore_current) = self.on_enter(block_stmt);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    // Visit inner for the block stmt
                    block_stmt.visit_mut_children_with(self);
                }
            }
            self.on_exit(old);
        }

        // FunctionDeclaration: entries(coverFunction),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_fn_decl(&mut self, fn_decl: &mut FnDecl) {
            let (old, ignore_current) = self.on_enter(fn_decl);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    self.create_fn_instrumentation(&Some(&fn_decl.ident), &mut fn_decl.function);
                    fn_decl.visit_mut_children_with(self);
                }
            }
            self.on_exit(old);
        }

        // ArrowFunctionExpression: entries(convertArrowExpression, coverFunction),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_arrow_expr(&mut self, arrow_expr: &mut ArrowExpr) {
            let (old, ignore_current) = self.on_enter(arrow_expr);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => match &mut *arrow_expr.body {
                    BlockStmtOrExpr::BlockStmt(block_stmt) => {
                        let range = crate::lookup_range::get_range_from_span(
                            &self.source_map,
                            &arrow_expr.span,
                        );
                        let body_range = crate::lookup_range::get_range_from_span(
                            &self.source_map,
                            &block_stmt.span,
                        );
                        let index = self
                            .cov
                            .borrow_mut()
                            .new_function(&None, &range, &body_range);
                        let b = crate::create_increase_counter_expr(
                            &crate::constants::idents::IDENT_F,
                            index,
                            &self.cov_fn_ident,
                            None,
                        );

                        // insert fn counter expression
                        let mut new_stmts = vec![Stmt::Expr(ExprStmt {
                            span: swc_core::common::DUMMY_SP,
                            expr: Box::new(b),
                        })];
                        // if arrow fn body is already blockstmt, insert stmt counter for each
                        self.insert_stmts_counter(&mut block_stmt.stmts);
                        new_stmts.extend(block_stmt.stmts.drain(..));
                        block_stmt.stmts = new_stmts;
                    }
                    BlockStmtOrExpr::Expr(expr) => {
                        // TODO: refactor common logics creates a blockstmt from single expr
                        let range = crate::lookup_range::get_range_from_span(
                            &self.source_map,
                            &arrow_expr.span,
                        );
                        let span = expr.span();
                        let body_range =
                            crate::lookup_range::get_range_from_span(&self.source_map, &span);
                        let index = self
                            .cov
                            .borrow_mut()
                            .new_function(&None, &range, &body_range);
                        let b = crate::create_increase_counter_expr(
                            &crate::constants::idents::IDENT_F,
                            index,
                            &self.cov_fn_ident,
                            None,
                        );

                        // insert fn counter expression
                        let mut stmts = vec![Stmt::Expr(ExprStmt {
                            span: swc_core::common::DUMMY_SP,
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

                        arrow_expr.body = Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
                            span: swc_core::common::DUMMY_SP,
                            stmts: new_stmts,
                            ..BlockStmt::dummy()
                        }));
                    }
                },
            }
            self.on_exit(old);
        }

        /*
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_stmt(&mut self, stmt: &mut Stmt) {
            if !self.is_injected_counter_stmt(stmt) {
                let span = crate::lookup_range::get_stmt_span(&stmt);
                if let Some(span) = span {
                    let increment_expr = self.create_stmt_increase_counter_expr(span, None);

                    self.before.push(Stmt::Expr(ExprStmt {
                        span: DUMMY_SP,
                        expr: Box::new(increment_expr),
                    }));
                }
            }
        } */

        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
            // Each Stmt looks up own comments for the hint, we don't
            // do self.on_enter() in here.
            self.nodes.push(crate::Node::Stmts);
            self.insert_stmts_counter(stmts);
            self.nodes.pop();
        }

        // FunctionExpression: entries(coverFunction),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_fn_expr(&mut self, fn_expr: &mut FnExpr) {
            let (old, ignore_current) = self.on_enter(fn_expr);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
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
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
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
                Some(crate::hint_comments::IgnoreScope::Next) => {}
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
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_break_stmt(&mut self, break_stmt: &mut BreakStmt) {
            let (old, ignore_current) = self.on_enter(break_stmt);

            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    self.mark_prepend_stmt_counter(&break_stmt.span);
                }
            }
            break_stmt.visit_mut_children_with(self);

            self.on_exit(old);
        }

        // ConditionalExpression: entries(coverTernary),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_cond_expr(&mut self, cond_expr: &mut CondExpr) {
            let (old, ignore_current) = self.on_enter(cond_expr);

            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    let range =
                        crate::lookup_range::get_range_from_span(&self.source_map, &cond_expr.span);
                    let branch = self.cov.borrow_mut().new_branch(
                        istanbul_oxide::BranchType::CondExpr,
                        &range,
                        false,
                    );

                    let c_hint = crate::hint_comments::lookup_hint_comments(
                        &self.comments,
                        Some(&cond_expr.cons.span()),
                    );
                    let a_hint = crate::hint_comments::lookup_hint_comments(
                        &self.comments,
                        Some(&cond_expr.alt.span()),
                    );

                    if c_hint.as_deref() != Some("next") {
                        // TODO: do we need this?
                        // cond_expr.cons.visit_mut_children_with(self);

                        // replace consequence to the paren for increase expr + expr itself
                        self.replace_expr_with_branch_counter(&mut *cond_expr.cons, branch);
                    }

                    if a_hint.as_deref() != Some("next") {
                        // TODO: do we need this?
                        // cond_expr.alt.visit_mut_children_with(self);

                        // replace consequence to the paren for increase expr + expr itself
                        self.replace_expr_with_branch_counter(&mut *cond_expr.alt, branch);
                    }
                }
            };

            cond_expr.visit_mut_children_with(self);
            self.on_exit(old);
        }

        // TaggedTemplateExpression: special handling to preserve template relationship
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_tagged_tpl(&mut self, tagged_tpl: &mut TaggedTpl) {
            let (old, ignore_current) = self.on_enter(tagged_tpl);

            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    // For tagged template expressions like styled(...)`template`,
                    // we need to instrument the entire expression rather than wrapping
                    // the tag part in a sequence expression, which would break the
                    // template relationship and cause emotion to lose label information.

                    // Instead of calling cover_statement on the tag (which would wrap it),
                    // we mark to prepend a statement counter before the entire tagged template
                    self.mark_prepend_stmt_counter(&tagged_tpl.span);

                    // Visit children normally to instrument any inner expressions
                    tagged_tpl.visit_mut_children_with(self);
                }
            }

            self.on_exit(old);
        }

        // ReturnStatement: entries(coverStatement),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_return_stmt(&mut self, return_stmt: &mut ReturnStmt) {
            let (old, ignore_current) = self.on_enter(return_stmt);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    self.mark_prepend_stmt_counter(&return_stmt.span);
                }
            }
            return_stmt.visit_mut_children_with(self);

            self.on_exit(old);
        }

        // VariableDeclaration: entries(), // ignore processing only
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_var_decl(&mut self, var_decl: &mut VarDecl) {
            let (old, _ignore_current) = self.on_enter(var_decl);
            //noop?
            var_decl.visit_mut_children_with(self);
            self.on_exit(old);
        }

        // ClassDeclaration: entries(parenthesizedExpressionProp('superClass')),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_class_decl(&mut self, class_decl: &mut ClassDecl) {
            let (old, ignore_current) = self.on_enter(class_decl);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    //self.mark_prepend_stmt_counter(&class_decl.class.span);
                    class_decl.visit_mut_children_with(self);
                }
            }

            self.on_exit(old);
        }

        // ClassProperty: entries(coverClassPropDeclarator),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_class_prop(&mut self, class_prop: &mut ClassProp) {
            let (old, ignore_current) = self.on_enter(class_prop);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    if let Some(value) = &mut class_prop.value {
                        self.cover_statement(&mut *value);
                    }
                    // Visit children to ensure arrow functions and other expressions are properly instrumented
                    class_prop.visit_mut_children_with(self);
                }
            }
            self.on_exit(old);
        }

        // ClassPrivateProperty: entries(coverClassPropDeclarator),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_private_prop(&mut self, private_prop: &mut PrivateProp) {
            // TODO: this is same as visit_mut_class_prop
            let (old, ignore_current) = self.on_enter(private_prop);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    if let Some(value) = &mut private_prop.value {
                        self.cover_statement(&mut *value);
                    }
                    // Visit children to ensure arrow functions and other expressions are properly instrumented
                    private_prop.visit_mut_children_with(self);
                }
            }
            self.on_exit(old);
        }

        // ClassMethod: entries(coverFunction),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_class_method(&mut self, class_method: &mut ClassMethod) {
            let (old, ignore_current) = self.on_enter(class_method);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    // TODO: this does not cover all of PropName enum yet
                    // TODO: duplicated logic between fn_expr
                    if let PropName::Ident(ident) = &class_method.key {
                        let should_ignore_via_options = self
                            .instrument_options
                            .ignore_class_methods
                            .iter()
                            .any(|v| v.as_str() == &*ident.sym);

                        if !should_ignore_via_options {
                            self.create_fn_instrumentation(
                                &Some(&ident.clone().into()),
                                &mut class_method.function,
                            );
                            class_method.visit_mut_children_with(self);
                        }
                    }
                }
            }
            self.on_exit(old);
        }

        // ObjectMethod: entries(coverFunction),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_method_prop(&mut self, method_prop: &mut MethodProp) {
            let (old, ignore_current) = self.on_enter(method_prop);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    // TODO: this does not cover all of PropName enum yet
                    // TODO: duplicated logic between class_method
                    if let PropName::Ident(ident) = &method_prop.key {
                        let should_ignore_via_options = self
                            .instrument_options
                            .ignore_class_methods
                            .iter()
                            .any(|v| v.as_str() == &*ident.sym);

                        if !should_ignore_via_options {
                            self.create_fn_instrumentation(
                                &Some(&ident.clone().into()),
                                &mut method_prop.function,
                            );
                            method_prop.visit_mut_children_with(self);
                        }
                    } else {
                        let ident = Ident {
                            sym: "anonymous".into(),
                            ..Ident::dummy()
                        };
                        self.create_fn_instrumentation(&Some(&ident), &mut method_prop.function);
                        method_prop.visit_mut_children_with(self);
                    }
                }
            }
            self.on_exit(old);
        }

        // ObjectMethod: entries(coverFunction),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_getter_prop(&mut self, getter_prop: &mut GetterProp) {
            let (old, ignore_current) = self.on_enter(getter_prop);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    // TODO: this does not cover all of PropName enum yet
                    // TODO: duplicated logic between class_method
                    if let PropName::Ident(ident) = &getter_prop.key {
                        let should_ignore_via_options = self
                            .instrument_options
                            .ignore_class_methods
                            .iter()
                            .any(|v| v.as_str() == &*ident.sym);

                        // TODO: there are _some_ duplication between create_fn_instrumentation
                        if !should_ignore_via_options {
                            let (span, name) = (&ident.span, Some(ident.sym.to_string()));

                            let range =
                                crate::lookup_range::get_range_from_span(&self.source_map, span);
                            if let Some(body) = &mut getter_prop.body {
                                let body_span = body.span;
                                let body_range = crate::lookup_range::get_range_from_span(
                                    &self.source_map,
                                    &body_span,
                                );
                                let index =
                                    self.cov
                                        .borrow_mut()
                                        .new_function(&name, &range, &body_range);

                                let b = crate::create_increase_counter_expr(
                                    &crate::constants::idents::IDENT_F,
                                    index,
                                    &self.cov_fn_ident,
                                    None,
                                );
                                let mut prepended_vec = vec![Stmt::Expr(ExprStmt {
                                    span: swc_core::common::DUMMY_SP,
                                    expr: Box::new(b),
                                })];
                                prepended_vec.extend(body.stmts.take());
                                body.stmts = prepended_vec;
                            }
                            getter_prop.visit_mut_children_with(self);
                        }
                    } else {
                        let span = &getter_prop.span;
                        let name: Option<String> = Some("anonymous".to_owned());

                        let range =
                            crate::lookup_range::get_range_from_span(&self.source_map, span);
                        if let Some(body) = &mut getter_prop.body {
                            let body_span = body.span;
                            let body_range = crate::lookup_range::get_range_from_span(
                                &self.source_map,
                                &body_span,
                            );
                            let index =
                                self.cov
                                    .borrow_mut()
                                    .new_function(&name, &range, &body_range);

                            let b = crate::create_increase_counter_expr(
                                &crate::constants::idents::IDENT_F,
                                index,
                                &self.cov_fn_ident,
                                None,
                            );
                            let mut prepended_vec = vec![Stmt::Expr(ExprStmt {
                                span: swc_core::common::DUMMY_SP,
                                expr: Box::new(b),
                            })];
                            prepended_vec.extend(body.stmts.take());
                            body.stmts = prepended_vec;
                        }
                        getter_prop.visit_mut_children_with(self);
                    }
                }
            }
            self.on_exit(old);
        }

        // TODO: this is same as visit_mut_getter_prop
        // ObjectMethod: entries(coverFunction),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_setter_prop(&mut self, setter_prop: &mut SetterProp) {
            let (old, ignore_current) = self.on_enter(setter_prop);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    // TODO: this does not cover all of PropName enum yet
                    // TODO: duplicated logic between class_method
                    if let PropName::Ident(ident) = &setter_prop.key {
                        let should_ignore_via_options = self
                            .instrument_options
                            .ignore_class_methods
                            .iter()
                            .any(|v| v.as_str() == &*ident.sym);

                        // TODO: there are _some_ duplication between create_fn_instrumentation
                        if !should_ignore_via_options {
                            let (span, name) = (&ident.span, Some(ident.sym.to_string()));

                            let range =
                                crate::lookup_range::get_range_from_span(&self.source_map, span);
                            if let Some(body) = &mut setter_prop.body {
                                let body_span = body.span;
                                let body_range = crate::lookup_range::get_range_from_span(
                                    &self.source_map,
                                    &body_span,
                                );
                                let index =
                                    self.cov
                                        .borrow_mut()
                                        .new_function(&name, &range, &body_range);

                                let b = crate::create_increase_counter_expr(
                                    &crate::constants::idents::IDENT_F,
                                    index,
                                    &self.cov_fn_ident,
                                    None,
                                );
                                let mut prepended_vec = vec![Stmt::Expr(ExprStmt {
                                    span: swc_core::common::DUMMY_SP,
                                    expr: Box::new(b),
                                })];
                                prepended_vec.extend(body.stmts.take());
                                body.stmts = prepended_vec;
                            }
                            setter_prop.visit_mut_children_with(self);
                        }
                    } else {
                        let span = &setter_prop.span;
                        let name: Option<String> = Some("anonymous".to_owned());

                        let range =
                            crate::lookup_range::get_range_from_span(&self.source_map, span);
                        if let Some(body) = &mut setter_prop.body {
                            let body_span = body.span;
                            let body_range = crate::lookup_range::get_range_from_span(
                                &self.source_map,
                                &body_span,
                            );
                            let index =
                                self.cov
                                    .borrow_mut()
                                    .new_function(&name, &range, &body_range);

                            let b = crate::create_increase_counter_expr(
                                &crate::constants::idents::IDENT_F,
                                index,
                                &self.cov_fn_ident,
                                None,
                            );
                            let mut prepended_vec = vec![Stmt::Expr(ExprStmt {
                                span: swc_core::common::DUMMY_SP,
                                expr: Box::new(b),
                            })];
                            prepended_vec.extend(body.stmts.take());
                            body.stmts = prepended_vec;
                        }
                        setter_prop.visit_mut_children_with(self);
                    }
                }
            }
            self.on_exit(old);
        }

        // VariableDeclarator: entries(coverVariableDeclarator),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_var_declarator(&mut self, declarator: &mut VarDeclarator) {
            let (old, ignore_current) = self.on_enter(declarator);

            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
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
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_for_stmt(&mut self, for_stmt: &mut ForStmt) {
            crate::visit_mut_for_like!(self, for_stmt);
        }

        // ForInStatement: entries(blockProp('body'), coverStatement),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_for_in_stmt(&mut self, for_in_stmt: &mut ForInStmt) {
            crate::visit_mut_for_like!(self, for_in_stmt);
        }

        // ForOfStatement: entries(blockProp('body'), coverStatement),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_for_of_stmt(&mut self, for_of_stmt: &mut ForOfStmt) {
            crate::visit_mut_for_like!(self, for_of_stmt);
        }

        // WhileStatement: entries(blockProp('body'), coverStatement),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_while_stmt(&mut self, while_stmt: &mut WhileStmt) {
            crate::visit_mut_for_like!(self, while_stmt);
        }

        // DoWhileStatement: entries(blockProp('body'), coverStatement),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_do_while_stmt(&mut self, do_while_stmt: &mut DoWhileStmt) {
            crate::visit_mut_for_like!(self, do_while_stmt);
        }

        //LabeledStatement: entries(coverStatement),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_labeled_stmt(&mut self, labeled_stmt: &mut LabeledStmt) {
            let (old, ignore_current) = self.on_enter(labeled_stmt);

            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    // cover_statement's is_stmt prepend logic for individual child stmt visitor
                    self.mark_prepend_stmt_counter(&labeled_stmt.span);
                }
            }

            labeled_stmt.visit_mut_children_with(self);

            self.on_exit(old);
        }

        // ContinueStatement: entries(coverStatement),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_continue_stmt(&mut self, continue_stmt: &mut ContinueStmt) {
            let (old, ignore_current) = self.on_enter(continue_stmt);

            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    // cover_statement's is_stmt prepend logic for individual child stmt visitor
                    self.mark_prepend_stmt_counter(&continue_stmt.span);
                }
            }

            continue_stmt.visit_mut_children_with(self);
            self.on_exit(old);
        }

        // SwitchStatement: entries(createSwitchBranch, coverStatement),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_switch_stmt(&mut self, switch_stmt: &mut SwitchStmt) {
            let (old, ignore_current) = self.on_enter(switch_stmt);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    // Insert stmt counter for `switch` itself, then create a new branch
                    self.mark_prepend_stmt_counter(&switch_stmt.span);

                    let range = crate::lookup_range::get_range_from_span(
                        &self.source_map,
                        &switch_stmt.span,
                    );
                    let branch =
                        self.cov
                            .borrow_mut()
                            .new_branch(crate::BranchType::Switch, &range, false);

                    // traverse `case` with a visitor contains branch idx, insert new
                    // branch increase counter accordingly
                    let mut visitor = crate::visitors::switch_case_visitor::SwitchCaseVisitor::new(
                        self.source_map.clone(),
                        self.comments.clone(),
                        self.cov.clone(),
                        self.instrument_options.clone(),
                        self.nodes.clone(),
                        ignore_current,
                        branch,
                    );

                    switch_stmt.visit_mut_children_with(&mut visitor);
                }
            }
            self.on_exit(old);
        }

        // IfStatement: entries(blockProp('consequent'), blockProp('alternate'), coverStatement, coverIfBranches)
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_if_stmt(&mut self, if_stmt: &mut IfStmt) {
            let (old, ignore_current) = self.on_enter(if_stmt);

            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {
                    self.on_exit(old);
                }
                _ => {
                    // cover_statement's is_stmt prepend logic for individual child stmt visitor
                    self.mark_prepend_stmt_counter(&if_stmt.span);

                    let range =
                        crate::lookup_range::get_range_from_span(&self.source_map, &if_stmt.span);
                    let branch =
                        self.cov
                            .borrow_mut()
                            .new_branch(crate::BranchType::If, &range, false);

                    let mut wrap_with_counter = |stmt: &mut Box<Stmt>| {
                        let mut stmt_body = *stmt.take();

                        // create a branch path counter
                        let idx = self.cov.borrow_mut().add_branch_path(branch, &range);
                        let expr = crate::create_increase_counter_expr(
                            &crate::constants::idents::IDENT_B,
                            branch,
                            &self.cov_fn_ident,
                            Some(idx),
                        );

                        let expr = Stmt::Expr(ExprStmt {
                            span: swc_core::common::DUMMY_SP,
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
                                self.source_map.clone(),
                                self.comments.clone(),
                                self.cov.clone(),
                                self.instrument_options.clone(),
                                self.nodes.clone(),
                                ignore_current,
                            );
                            stmt_body.visit_mut_with(&mut visitor);
                            stmts.extend(visitor.before.drain(..));

                            stmts.push(stmt_body);

                            BlockStmt {
                                span: swc_core::common::DUMMY_SP,
                                stmts,
                                ..Default::default()
                            }
                        };

                        *stmt = Box::new(Stmt::Block(body));
                    };

                    // Note: unlike upstream, we do not use setAttr-based approach as it is not easy to
                    // append arbitary dynamic metadata on the parents can be accessed in any childs.
                    if ignore_current != Some(crate::hint_comments::IgnoreScope::If) {
                        wrap_with_counter(&mut if_stmt.cons);
                    }

                    if ignore_current != Some(crate::hint_comments::IgnoreScope::Else) {
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
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_bin_expr(&mut self, bin_expr: &mut BinExpr) {
            // We don't use self.on_enter() here since Node::LogicalExpr is a dialect of BinExpr
            // which we can't pass directly via on_enter() macro
            let old = self.should_ignore;
            let ignore_current = match old {
                Some(crate::hint_comments::IgnoreScope::Next) => old,
                _ => {
                    self.should_ignore =
                        crate::hint_comments::should_ignore(&self.comments, Some(&bin_expr.span));
                    self.should_ignore
                }
            };

            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {
                    self.nodes.push(crate::Node::BinExpr);
                    bin_expr.visit_mut_children_with(self);
                    self.on_exit(old);
                }
                _ => {
                    match &bin_expr.op {
                        BinaryOp::LogicalOr
                        | BinaryOp::LogicalAnd
                        | BinaryOp::NullishCoalescing => {
                            self.nodes.push(crate::Node::LogicalExpr);

                            // Create a new branch. This id should be reused for any inner logical expr.
                            let range = crate::lookup_range::get_range_from_span(
                                &self.source_map,
                                &bin_expr.span,
                            );
                            let branch = self.cov.borrow_mut().new_branch(
                                crate::BranchType::BinaryExpr,
                                &range,
                                self.instrument_options.report_logic,
                            );

                            // Iterate over each expr, wrap it with branch counter.
                            self.wrap_bin_expr_with_branch_counter(branch, &mut *bin_expr.left);
                            self.wrap_bin_expr_with_branch_counter(branch, &mut *bin_expr.right);
                        }
                        _ => {
                            // iterate as normal for non loigical expr
                            self.nodes.push(crate::Node::BinExpr);
                            bin_expr.visit_mut_children_with(self);
                            self.on_exit(old);
                        }
                    }
                }
            }
        }

        // AssignmentPattern: entries(coverAssignmentPattern),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_assign_pat(&mut self, assign_pat: &mut AssignPat) {
            let (old, ignore_current) = self.on_enter(assign_pat);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    let range = crate::lookup_range::get_range_from_span(
                        &self.source_map,
                        &assign_pat.span,
                    );
                    let branch = self.cov.borrow_mut().new_branch(
                        crate::BranchType::DefaultArg,
                        &range,
                        false,
                    );

                    self.wrap_bin_expr_with_branch_counter(branch, &mut *assign_pat.right);
                }
            }
            self.on_exit(old);
        }

        // TryStatement: entries(coverStatement),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_try_stmt(&mut self, try_stmt: &mut TryStmt) {
            let (old, ignore_current) = self.on_enter(try_stmt);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    self.mark_prepend_stmt_counter(&try_stmt.span);
                    try_stmt.visit_mut_children_with(self);
                }
            }
            self.on_exit(old);
        }

        // ThrowStatement: entries(coverStatement),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_throw_stmt(&mut self, throw_stmt: &mut ThrowStmt) {
            let (old, ignore_current) = self.on_enter(throw_stmt);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    self.mark_prepend_stmt_counter(&throw_stmt.span);
                    throw_stmt.visit_mut_children_with(self);
                }
            }
            self.on_exit(old);
        }

        // WithStatement: entries(blockProp('body'), coverStatement),
        #[tracing::instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_with_stmt(&mut self, with_stmt: &mut WithStmt) {
            let (old, ignore_current) = self.on_enter(with_stmt);
            match ignore_current {
                Some(crate::hint_comments::IgnoreScope::Next) => {}
                _ => {
                    self.mark_prepend_stmt_counter(&with_stmt.span);

                    //TODO: duplicated codes for wrapping block
                    if let Stmt::Block(body_block) = &mut *with_stmt.body {
                        self.insert_stmts_counter(&mut body_block.stmts);
                    } else {
                        let mut visitor = crate::visitors::stmt_like_visitor::StmtVisitor::new(
                            self.source_map.clone(),
                            self.comments.clone(),
                            self.cov.clone(),
                            self.instrument_options.clone(),
                            self.nodes.clone(),
                            ignore_current,
                        );
                        with_stmt.body.visit_mut_with(&mut visitor);
                        let mut new_stmts = vec![];
                        new_stmts.extend(visitor.before.drain(..));
                        new_stmts.push(*with_stmt.body.take());

                        with_stmt.body = Box::new(Stmt::Block(BlockStmt {
                            span: swc_core::common::DUMMY_SP,
                            stmts: new_stmts,
                            ..Default::default()
                        }));
                    }
                }
            }
            self.on_exit(old);
        }
    };
}
