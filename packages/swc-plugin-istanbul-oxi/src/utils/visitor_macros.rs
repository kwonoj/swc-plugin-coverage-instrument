#[derive(Debug)]
pub struct UnknownReserved;
impl Default for UnknownReserved {
    fn default() -> UnknownReserved {
        UnknownReserved
    }
}

/// Expand given struct to contain necessary common filed for the coverage visitor
/// with common utility functions.
///
/// This does not impl actual visitors (VisitMut) as each visitor may have different
/// visitor logics.
#[macro_export]
macro_rules! create_coverage_visitor {
    ($name:ident { $($field:ident: $t:ty),* $(,)? }) => {
        #[allow(unused)]
        #[derive(Debug)]
        pub struct $name<'a> {
            source_map: &'a swc_plugin::source_map::PluginSourceMapProxy,
            comments: Option<&'a swc_plugin::comments::PluginCommentsProxy>,
            cov: &'a mut istanbul_oxi_instrument::SourceCoverage,
            cov_fn_ident: swc_plugin::ast::Ident,
            instrument_options: crate::InstrumentOptions,
            pub before: Vec<swc_plugin::ast::Stmt>,
            nodes: Vec<Node>,
            should_ignore: bool,
            $(pub $field: $t,)*
        }

        impl<'a> $name<'a> {
            pub fn new(
                source_map: &'a swc_plugin::source_map::PluginSourceMapProxy,
                comments: Option<&'a swc_plugin::comments::PluginCommentsProxy>,
                cov: &'a mut istanbul_oxi_instrument::SourceCoverage,
                instrument_options: &'a crate::InstrumentOptions,
                nodes: &'a Vec<Node>,
                should_ignore: bool,
                $($field: $t,)*
            ) -> $name<'a> {
                $name {
                    source_map,
                    comments,
                    cov,
                    cov_fn_ident: crate::COVERAGE_FN_IDENT.get().expect("Coverage fn Ident should be initialized already").clone(),
                    instrument_options: instrument_options.clone(),
                    before: vec![],
                    nodes: nodes.clone(),
                    should_ignore,
                    $($field,)*
                }
            }

            fn on_exit(&mut self, old: bool) {
                self.should_ignore = old;
                self.nodes.pop();
            }
        }

        #[allow(unused)]
        use swc_plugin::ast::*;
        #[allow(unused)]
        use crate::utils::node::*;

        /// A trait expands to the ast types we want to use to determine if we need to ignore
        /// certain section of the code for the instrumentation.
        /// TODO: Can a macro like `on_visit_mut_expr` expands on_enter / exit automatically?
        /// `on_visit_mut_expr!(|expr| {self.xxx})` doesn't seem to work.
        trait CoverageInstrumentationMutVisitEnter<N> {
            fn on_enter(&mut self, n: &mut N) -> (bool, bool);
        }


        // Macro generates trait impl for the type can access span directly.
        macro_rules! on_enter_span {
            ($N: tt) => {
                impl CoverageInstrumentationMutVisitEnter<$N> for $name<'_> {
                    #[inline]
                    fn on_enter(&mut self, n: &mut swc_plugin::ast::$N) -> (bool, bool) {
                        self.nodes.push(Node::$N);

                        let old = self.should_ignore;
                        let ret = if old {
                            old
                        } else {
                            self.should_ignore = crate::utils::hint_comments::should_ignore(&self.comments, Some(&n.span));
                            self.should_ignore
                        };

                        (old, ret)
                    }
                 }
            }
        }

        impl CoverageInstrumentationMutVisitEnter<Expr> for $name<'_> {
            fn on_enter(&mut self, n: &mut Expr) -> (bool, bool) {
                self.nodes.push(Node::Expr);

                let old = self.should_ignore;
                let ret = if old {
                    old
                } else {
                    let span = get_expr_span(n);
                    self.should_ignore  = crate::utils::hint_comments::should_ignore(&self.comments, span);
                    self.should_ignore
                };

                (old, ret)
            }
         }

         on_enter_span!(BinExpr);
         on_enter_span!(VarDeclarator);
         on_enter_span!(VarDecl);
         on_enter_span!(CondExpr);
         on_enter_span!(ExprStmt);
    }
}

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
macro_rules! insert_logical_expr_helper {
    () => {
        /// Attempt to wrap expression with branch increase counter. Given Expr may be left, or right of the logical expression.
        fn wrap_bin_expr_with_branch_counter(&mut self, branch: u32, expr: &mut Expr) {
            // Logical expression can have inner logical expression as non-direct child
            // (i.e `args[0] > 0 && (args[0] < 5 || args[0] > 10)`, logical || expr is child of ParenExpr.
            // Try to look up if current expr is the `leaf` of whole logical expr tree.
            let mut has_inner_logical_expr = crate::visitors::finders::LogicalExprLeafFinder(false);
            expr.visit_with(&mut has_inner_logical_expr);

            // If current expr have inner logical expr, traverse until reaches to the leaf
            if has_inner_logical_expr.0 {
                let mut visitor = crate::visitors::logical_expr_visitor::LogicalExprVisitor::new(
                    self.source_map,
                    self.comments,
                    &mut self.cov,
                    &self.instrument_options,
                    &self.nodes,
                    self.should_ignore,
                    branch,
                );

                expr.visit_mut_children_with(&mut visitor);
            } else {
                // Now we believe this expr is the leaf of the logical expr tree.
                // Wrap it with branch counter.
                if self.instrument_options.report_logic {
                    /*
                    // TODO
                    const increment = this.getBranchLogicIncrement(
                        leaf,
                        b,
                        leaf.node.loc
                    );
                    if (!increment[0]) {
                        continue;
                    }
                    leaf.parent[leaf.property] = T.sequenceExpression([
                        increment[0],
                        increment[1]
                    ]);
                    */
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

        // if (path.isExpression()) {
        //    path.replaceWith(T.sequenceExpression([increment, path.node]));
        //}
        #[tracing::instrument(skip_all)]
        fn replace_expr_with_stmt_counter(&mut self, expr: &mut Expr) {
            self.replace_expr_with_counter(expr, |cov, cov_fn_ident, range| {
                let idx = cov.new_statement(&range);
                create_increase_expression_expr(&IDENT_S, idx, cov_fn_ident, None)
            });
        }

        #[tracing::instrument(skip_all)]
        fn replace_expr_with_branch_counter(&mut self, expr: &mut Expr, branch: u32) {
            self.replace_expr_with_counter(expr, |cov, cov_fn_ident, range| {
                let idx = cov.add_branch_path(branch, &range);

                create_increase_expression_expr(&IDENT_B, branch, cov_fn_ident, Some(idx))
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
                let prepend_expr = get_counter(&mut self.cov, &self.cov_fn_ident, &init_range);

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
                    let b =
                        create_increase_expression_expr(&IDENT_F, index, &self.cov_fn_ident, None);
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

        fn is_injected_counter_expr(&self, expr: &swc_plugin::ast::Expr) -> bool {
            use swc_plugin::ast::*;

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

                let mut expr_finder = crate::visitors::finders::ExprFinder::new();
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
        noop_visit_mut_type!();

        /*
        fn visit_mut_array_lit(&mut self, n: &mut ArrayLit) {}
        fn visit_mut_array_pat(&mut self, n: &mut ArrayPat) {}
        fn visit_mut_arrow_expr(&mut self, n: &mut ArrowExpr) {}
        fn visit_mut_assign_expr(&mut self, n: &mut AssignExpr) {}
        fn visit_mut_assign_op(&mut self, n: &mut AssignOp) {}
        fn visit_mut_assign_pat(&mut self, n: &mut AssignPat) {}
        fn visit_mut_assign_pat_prop(&mut self, n: &mut AssignPatProp) {}
        fn visit_mut_assign_prop(&mut self, n: &mut AssignProp) {}
        fn visit_mut_await_expr(&mut self, n: &mut AwaitExpr) {}
        fn visit_mut_big_int(&mut self, n: &mut BigInt) {}
        fn visit_mut_binary_op(&mut self, n: &mut BinaryOp) {}
        fn visit_mut_binding_ident(&mut self, n: &mut BindingIdent) {}
        fn visit_mut_block_stmt_or_expr(&mut self, n: &mut BlockStmtOrExpr) {}
        fn visit_mut_bool(&mut self, n: &mut Bool) {}
        fn visit_mut_break_stmt(&mut self, n: &mut BreakStmt) {}
        fn visit_mut_call_expr(&mut self, n: &mut CallExpr) {}
        fn visit_mut_callee(&mut self, n: &mut Callee) {}
        fn visit_mut_catch_clause(&mut self, n: &mut CatchClause) {}
        fn visit_mut_class(&mut self, n: &mut Class) {}
        fn visit_mut_class_decl(&mut self, n: &mut ClassDecl) {}
        fn visit_mut_class_expr(&mut self, n: &mut ClassExpr) {}
        fn visit_mut_class_member(&mut self, n: &mut ClassMember) {}
        fn visit_mut_class_members(&mut self, n: &mut Vec<ClassMember>) {}
        fn visit_mut_class_method(&mut self, n: &mut ClassMethod) {}
        fn visit_mut_class_prop(&mut self, n: &mut ClassProp) {}
        fn visit_mut_computed_prop_name(&mut self, n: &mut ComputedPropName) {}
        fn visit_mut_cond_expr(&mut self, n: &mut CondExpr) {}
        fn visit_mut_constructor(&mut self, n: &mut Constructor) {}
        fn visit_mut_continue_stmt(&mut self, n: &mut ContinueStmt) {}
        fn visit_mut_debugger_stmt(&mut self, n: &mut DebuggerStmt) {}
        fn visit_mut_decl(&mut self, n: &mut Decl) {}
        fn visit_mut_decorator(&mut self, n: &mut Decorator) {}
        fn visit_mut_decorators(&mut self, n: &mut Vec<Decorator>) {}
        fn visit_mut_default_decl(&mut self, n: &mut DefaultDecl) {}
        fn visit_mut_do_while_stmt(&mut self, n: &mut DoWhileStmt) {}
        fn visit_mut_empty_stmt(&mut self, n: &mut EmptyStmt) {}
        fn visit_mut_export_all(&mut self, n: &mut ExportAll) {}
        fn visit_mut_export_decl(&mut self, n: &mut ExportDecl) {}
        fn visit_mut_export_default_decl(&mut self, n: &mut ExportDefaultDecl) {}
        fn visit_mut_export_default_expr(&mut self, n: &mut ExportDefaultExpr) {}
        fn visit_mut_export_default_specifier(&mut self, n: &mut ExportDefaultSpecifier) {}
        fn visit_mut_export_named_specifier(&mut self, n: &mut ExportNamedSpecifier) {}
        fn visit_mut_export_namespace_specifier(&mut self, n: &mut ExportNamespaceSpecifier) {}
        fn visit_mut_export_specifier(&mut self, n: &mut ExportSpecifier) {}
        fn visit_mut_export_specifiers(&mut self, n: &mut Vec<ExportSpecifier>) {}
        fn visit_mut_expr(&mut self, n: &mut Expr) {}
        fn visit_mut_expr_or_spread(&mut self, n: &mut ExprOrSpread) {}
        fn visit_mut_expr_or_spreads(&mut self, n: &mut Vec<ExprOrSpread>) {}
        fn visit_mut_exprs(&mut self, n: &mut Vec<Box<Expr>>) {}
        fn visit_mut_f_64(&mut self, n: &mut f64) {}
        fn visit_mut_fn_expr(&mut self, n: &mut FnExpr) {}
        fn visit_mut_for_in_stmt(&mut self, n: &mut ForInStmt) {}
        fn visit_mut_for_of_stmt(&mut self, n: &mut ForOfStmt) {}
        fn visit_mut_function(&mut self, n: &mut Function) {}
        fn visit_mut_getter_prop(&mut self, n: &mut GetterProp) {}
        fn visit_mut_ident(&mut self, n: &mut Ident) {}
        fn visit_mut_import(&mut self, n: &mut Import) {}
        fn visit_mut_import_decl(&mut self, n: &mut ImportDecl) {}
        fn visit_mut_import_default_specifier(&mut self, n: &mut ImportDefaultSpecifier) {}
        fn visit_mut_import_named_specifier(&mut self, n: &mut ImportNamedSpecifier) {}
        fn visit_mut_import_specifier(&mut self, n: &mut ImportSpecifier) {}
        fn visit_mut_import_specifiers(&mut self, n: &mut Vec<ImportSpecifier>) {}
        fn visit_mut_import_star_as_specifier(&mut self, n: &mut ImportStarAsSpecifier) {}
        fn visit_mut_invalid(&mut self, n: &mut Invalid) {}
        fn visit_mut_js_word(&mut self, n: &mut JsWord) {}
        fn visit_mut_jsx_attr(&mut self, n: &mut JSXAttr) {}
        fn visit_mut_jsx_attr_name(&mut self, n: &mut JSXAttrName) {}
        fn visit_mut_jsx_attr_or_spread(&mut self, n: &mut JSXAttrOrSpread) {}
        fn visit_mut_jsx_attr_or_spreads(&mut self, n: &mut Vec<JSXAttrOrSpread>) {}
        fn visit_mut_jsx_attr_value(&mut self, n: &mut JSXAttrValue) {}
        fn visit_mut_jsx_closing_element(&mut self, n: &mut JSXClosingElement) {}
        fn visit_mut_jsx_closing_fragment(&mut self, n: &mut JSXClosingFragment) {}
        fn visit_mut_jsx_element(&mut self, n: &mut JSXElement) {}
        fn visit_mut_jsx_element_child(&mut self, n: &mut JSXElementChild) {}
        fn visit_mut_jsx_element_children(&mut self, n: &mut Vec<JSXElementChild>) {}
        fn visit_mut_jsx_element_name(&mut self, n: &mut JSXElementName) {}
        fn visit_mut_jsx_empty_expr(&mut self, n: &mut JSXEmptyExpr) {}
        fn visit_mut_jsx_expr(&mut self, n: &mut JSXExpr) {}
        fn visit_mut_jsx_expr_container(&mut self, n: &mut JSXExprContainer) {}
        fn visit_mut_jsx_fragment(&mut self, n: &mut JSXFragment) {}
        fn visit_mut_jsx_member_expr(&mut self, n: &mut JSXMemberExpr) {}
        fn visit_mut_jsx_namespaced_name(&mut self, n: &mut JSXNamespacedName) {}
        fn visit_mut_jsx_object(&mut self, n: &mut JSXObject) {}
        fn visit_mut_jsx_opening_element(&mut self, n: &mut JSXOpeningElement) {}
        fn visit_mut_jsx_opening_fragment(&mut self, n: &mut JSXOpeningFragment) {}
        fn visit_mut_jsx_spread_child(&mut self, n: &mut JSXSpreadChild) {}
        fn visit_mut_jsx_text(&mut self, n: &mut JSXText) {}
        fn visit_mut_key_value_pat_prop(&mut self, n: &mut KeyValuePatProp) {}
        fn visit_mut_key_value_prop(&mut self, n: &mut KeyValueProp) {}
        fn visit_mut_lit(&mut self, n: &mut Lit) {}
        fn visit_mut_member_expr(&mut self, n: &mut MemberExpr) {}
        fn visit_mut_member_prop(&mut self, n: &mut MemberProp) {}
        fn visit_mut_meta_prop_expr(&mut self, n: &mut MetaPropExpr) {}
        fn visit_mut_meta_prop_kind(&mut self, n: &mut MetaPropKind) {}
        fn visit_mut_method_kind(&mut self, n: &mut MethodKind) {}
        fn visit_mut_method_prop(&mut self, n: &mut MethodProp) {}
        fn visit_mut_module(&mut self, n: &mut Module) {}
        fn visit_mut_module_decl(&mut self, n: &mut ModuleDecl) {}
        fn visit_mut_module_export_name(&mut self, n: &mut ModuleExportName) {}
        fn visit_mut_module_item(&mut self, n: &mut ModuleItem) {}
        fn visit_mut_module_items(&mut self, n: &mut Vec<ModuleItem>) {}
        fn visit_mut_named_export(&mut self, n: &mut NamedExport) {}
        fn visit_mut_new_expr(&mut self, n: &mut NewExpr) {}
        fn visit_mut_null(&mut self, n: &mut Null) {}
        fn visit_mut_number(&mut self, n: &mut Number) {}
        fn visit_mut_object_lit(&mut self, n: &mut ObjectLit) {}
        fn visit_mut_object_pat(&mut self, n: &mut ObjectPat) {}
        fn visit_mut_object_pat_prop(&mut self, n: &mut ObjectPatProp) {}
        fn visit_mut_object_pat_props(&mut self, n: &mut Vec<ObjectPatProp>) {}
        fn visit_mut_opt_accessibility(&mut self, n: &mut Option<Accessibility>) {}
        fn visit_mut_opt_block_stmt(&mut self, n: &mut Option<BlockStmt>) {}
        fn visit_mut_opt_call(&mut self, n: &mut OptCall) {}
        fn visit_mut_opt_catch_clause(&mut self, n: &mut Option<CatchClause>) {}
        fn visit_mut_opt_chain_base(&mut self, n: &mut OptChainBase) {}
        fn visit_mut_opt_chain_expr(&mut self, n: &mut OptChainExpr) {}
        fn visit_mut_opt_expr(&mut self, n: &mut Option<Box<Expr>>) {}
        fn visit_mut_opt_expr_or_spread(&mut self, n: &mut Option<ExprOrSpread>) {}
        fn visit_mut_opt_expr_or_spreads(&mut self, n: &mut Option<Vec<ExprOrSpread>>) {}
        fn visit_mut_opt_ident(&mut self, n: &mut Option<Ident>) {}
        fn visit_mut_opt_js_word(&mut self, n: &mut Option<JsWord>) {}
        fn visit_mut_opt_jsx_attr_value(&mut self, n: &mut Option<JSXAttrValue>) {}
        fn visit_mut_opt_jsx_closing_element(&mut self, n: &mut Option<JSXClosingElement>) {}
        fn visit_mut_opt_module_export_name(&mut self, n: &mut Option<ModuleExportName>) {}
        fn visit_mut_opt_object_lit(&mut self, n: &mut Option<ObjectLit>) {}
        fn visit_mut_opt_pat(&mut self, n: &mut Option<Pat>) {}
        fn visit_mut_opt_span(&mut self, n: &mut Option<Span>) {}
        fn visit_mut_opt_stmt(&mut self, n: &mut Option<Box<Stmt>>) {}
        fn visit_mut_opt_str(&mut self, n: &mut Option<Str>) {}
        fn visit_mut_opt_true_plus_minus(&mut self, n: &mut Option<TruePlusMinus>) {}
        fn visit_mut_opt_ts_entity_name(&mut self, n: &mut Option<TsEntityName>) {}
        fn visit_mut_opt_ts_namespace_body(&mut self, n: &mut Option<TsNamespaceBody>) {}
        fn visit_mut_opt_ts_type(&mut self, n: &mut Option<Box<TsType>>) {}
        fn visit_mut_opt_ts_type_ann(&mut self, n: &mut Option<TsTypeAnn>) {}
        fn visit_mut_opt_ts_type_param_decl(&mut self, n: &mut Option<TsTypeParamDecl>) {}
        fn visit_mut_opt_ts_type_param_instantiation(
            &mut self,
            n: &mut Option<TsTypeParamInstantiation>,
        ) {
        }
        fn visit_mut_opt_var_decl_or_expr(&mut self, n: &mut Option<VarDeclOrExpr>) {}
        fn visit_mut_opt_vec_expr_or_spreads(&mut self, n: &mut Vec<Option<ExprOrSpread>>) {}
        fn visit_mut_opt_vec_pats(&mut self, n: &mut Vec<Option<Pat>>) {}
        fn visit_mut_param(&mut self, n: &mut Param) {}
        fn visit_mut_param_or_ts_param_prop(&mut self, n: &mut ParamOrTsParamProp) {}
        fn visit_mut_param_or_ts_param_props(&mut self, n: &mut Vec<ParamOrTsParamProp>) {}
        fn visit_mut_params(&mut self, n: &mut Vec<Param>) {}
        fn visit_mut_paren_expr(&mut self, n: &mut ParenExpr) {}
        fn visit_mut_pat(&mut self, n: &mut Pat) {}
        fn visit_mut_pat_or_expr(&mut self, n: &mut PatOrExpr) {}
        fn visit_mut_pats(&mut self, n: &mut Vec<Pat>) {}
        fn visit_mut_private_method(&mut self, n: &mut PrivateMethod) {}
        fn visit_mut_private_name(&mut self, n: &mut PrivateName) {}
        fn visit_mut_private_prop(&mut self, n: &mut PrivateProp) {}
        fn visit_mut_program(&mut self, n: &mut Program) {}
        fn visit_mut_prop(&mut self, n: &mut Prop) {}
        fn visit_mut_prop_name(&mut self, n: &mut PropName) {}
        fn visit_mut_prop_or_spread(&mut self, n: &mut PropOrSpread) {}
        fn visit_mut_prop_or_spreads(&mut self, n: &mut Vec<PropOrSpread>) {}
        fn visit_mut_regex(&mut self, n: &mut Regex) {}
        fn visit_mut_rest_pat(&mut self, n: &mut RestPat) {}
        fn visit_mut_script(&mut self, n: &mut Script) {}
        fn visit_mut_seq_expr(&mut self, n: &mut SeqExpr) {}
        fn visit_mut_setter_prop(&mut self, n: &mut SetterProp) {}
        fn visit_mut_span(&mut self, n: &mut Span) {}
        fn visit_mut_spread_element(&mut self, n: &mut SpreadElement) {}
        fn visit_mut_static_block(&mut self, n: &mut StaticBlock) {}
        fn visit_mut_stmt(&mut self, n: &mut Stmt) {}
        fn visit_mut_stmts(&mut self, n: &mut Vec<Stmt>) {}
        fn visit_mut_str(&mut self, n: &mut Str) {}
        fn visit_mut_super(&mut self, n: &mut Super) {}
        fn visit_mut_super_prop(&mut self, n: &mut SuperProp) {}
        fn visit_mut_super_prop_expr(&mut self, n: &mut SuperPropExpr) {}
        fn visit_mut_switch_case(&mut self, n: &mut SwitchCase) {}
        fn visit_mut_switch_cases(&mut self, n: &mut Vec<SwitchCase>) {}
        fn visit_mut_switch_stmt(&mut self, n: &mut SwitchStmt) {}
        fn visit_mut_tagged_tpl(&mut self, n: &mut TaggedTpl) {}
        fn visit_mut_this_expr(&mut self, n: &mut ThisExpr) {}
        fn visit_mut_throw_stmt(&mut self, n: &mut ThrowStmt) {}
        fn visit_mut_tpl(&mut self, n: &mut Tpl) {}
        fn visit_mut_tpl_element(&mut self, n: &mut TplElement) {}
        fn visit_mut_tpl_elements(&mut self, n: &mut Vec<TplElement>) {}
        fn visit_mut_true_plus_minus(&mut self, n: &mut TruePlusMinus) {}
        fn visit_mut_try_stmt(&mut self, n: &mut TryStmt) {}
        fn visit_mut_unary_expr(&mut self, n: &mut UnaryExpr) {}
        fn visit_mut_unary_op(&mut self, n: &mut UnaryOp) {}
        fn visit_mut_update_expr(&mut self, n: &mut UpdateExpr) {}
        fn visit_mut_update_op(&mut self, n: &mut UpdateOp) {}
        fn visit_mut_var_decl(&mut self, n: &mut VarDecl) {}
        fn visit_mut_var_decl_kind(&mut self, n: &mut VarDeclKind) {}
        fn visit_mut_var_decl_or_expr(&mut self, n: &mut VarDeclOrExpr) {}
        fn visit_mut_var_decl_or_pat(&mut self, n: &mut VarDeclOrPat) {}
        fn visit_mut_var_declarators(&mut self, n: &mut Vec<VarDeclarator>) {}
        fn visit_mut_while_stmt(&mut self, n: &mut WhileStmt) {}
        fn visit_mut_with_stmt(&mut self, n: &mut WithStmt) {}
        fn visit_mut_yield_expr(&mut self, n: &mut YieldExpr) {}
         */

        // BlockStatement: entries(), // ignore processing only
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_block_stmt(&mut self, block_stmt: &mut BlockStmt) {
            self.nodes.push(Node::BlockStmt);

            // Recursively visit inner for the blockstmt
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
            let (old, ignore_current) = self.on_enter(expr_stmt);

            if !ignore_current && !self.is_injected_counter_expr(&*expr_stmt.expr) {
                self.mark_prepend_stmt_counter(&expr_stmt.span);
            }
            expr_stmt.visit_mut_children_with(self);

            self.on_exit(old);
        }

        // ReturnStatement: entries(coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_return_stmt(&mut self, return_stmt: &mut ReturnStmt) {
            self.nodes.push(Node::ReturnStmt);
            self.mark_prepend_stmt_counter(&return_stmt.span);
            return_stmt.visit_mut_children_with(self);
            self.nodes.pop();
        }

        // VariableDeclaration: entries(), // ignore processing only
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_var_decl(&mut self, var_decl: &mut VarDecl) {
            let (old, ignore_current) = self.on_enter(var_decl);
            //noop?
            var_decl.visit_mut_children_with(self);
            self.on_exit(old);
        }

        // VariableDeclarator: entries(coverVariableDeclarator),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_var_declarator(&mut self, declarator: &mut VarDeclarator) {
            let (old, ignore_current) = self.on_enter(declarator);

            if !ignore_current {
                if let Some(init) = &mut declarator.init {
                    let init = &mut **init;
                    self.cover_statement(init);
                }
            }

            declarator.visit_mut_children_with(self);

            self.on_exit(old);
        }

        // ForStatement: entries(blockProp('body'), coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_for_stmt(&mut self, for_stmt: &mut ForStmt) {
            self.nodes.push(Node::ForStmt);

            // cover_statement's is_stmt prepend logic for individual child stmt visitor
            self.mark_prepend_stmt_counter(&for_stmt.span);

            let body = *for_stmt.body.take();
            // if for stmt body is not block, wrap it before insert statement counter
            let body = if let Stmt::Block(body) = body {
                //self.insert_stmts_counter(&mut body.stmts);
                body
            } else {
                let stmts = vec![body];
                //self.insert_stmts_counter(&mut stmts);

                BlockStmt {
                    span: DUMMY_SP,
                    stmts,
                }
            };

            //self.insert_stmts_counter(&mut body.stmts);

            for_stmt.body = Box::new(Stmt::Block(body));
            for_stmt.visit_mut_children_with(self);

            self.nodes.pop();
        }

        // FunctionExpression: entries(coverStatement),
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_labeled_stmt(&mut self, labeled_stmt: &mut LabeledStmt) {
            self.nodes.push(Node::LabeledStmt);

            // cover_statement's is_stmt prepend logic for individual child stmt visitor
            self.mark_prepend_stmt_counter(&labeled_stmt.span);
            labeled_stmt.visit_mut_children_with(self);
            self.nodes.pop();
        }

        // IfStatement: entries(blockProp('consequent'), blockProp('alternate'), coverStatement, coverIfBranches)
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_if_stmt(&mut self, if_stmt: &mut IfStmt) {
            self.nodes.push(Node::IfStmt);

            // cover_statement's is_stmt prepend logic for individual child stmt visitor
            self.mark_prepend_stmt_counter(&if_stmt.span);

            let hint = crate::utils::hint_comments::lookup_hint_comments(
                &self.comments,
                Some(if_stmt.span).as_ref(),
            );
            let (ignore_if, ignore_else) = if let Some(hint) = hint {
                (&hint == "if", &hint == "else")
            } else {
                (false, false)
            };

            let range = get_range_from_span(self.source_map, &if_stmt.span);
            let branch =
                self.cov
                    .new_branch(istanbul_oxi_instrument::BranchType::If, &range, false);

            let mut wrap_with_counter = |stmt: &mut Box<Stmt>| {
                let stmt_body = *stmt.take();

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
                    // if cons / alt is not a blockstmt, manually create stmt increase counter
                    // for the stmt then wrap it with blockstmt
                    let span = crate::utils::lookup_range::get_stmt_span(&stmt_body);
                    let stmts = if let Some(span) = span {
                        let increment_expr = self.create_stmt_increase_counter_expr(span, None);
                        vec![
                            expr,
                            Stmt::Expr(ExprStmt {
                                span: DUMMY_SP,
                                expr: Box::new(increment_expr),
                            }),
                            stmt_body,
                        ]
                    } else {
                        vec![expr, stmt_body]
                    };

                    BlockStmt {
                        span: DUMMY_SP,
                        stmts,
                    }
                };

                *stmt = Box::new(Stmt::Block(body));
            };

            if ignore_if {
                //setAttr(if_stmt.cons, 'skip-all', true);
            } else {
                wrap_with_counter(&mut if_stmt.cons);
            }

            if ignore_else {
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
                    self.nodes.pop();
                    return;
                }
            }

            // We visit individual cons / alt depends on its state, need to run visitor for the `test` as well
            if_stmt.test.visit_mut_with(self);

            self.nodes.pop();
        }

        // LogicalExpression: entries(coverLogicalExpression)
        #[instrument(skip_all, fields(node = %self.print_node()))]
        fn visit_mut_bin_expr(&mut self, bin_expr: &mut BinExpr) {
            match &bin_expr.op {
                BinaryOp::LogicalOr | BinaryOp::LogicalAnd | BinaryOp::NullishCoalescing => {
                    self.nodes.push(Node::LogicalExpr);

                    // escape if there's ignore comments
                    let hint = crate::utils::hint_comments::lookup_hint_comments(
                        &self.comments,
                        Some(bin_expr.span).as_ref(),
                    );
                    if hint.as_deref() == Some("next") {
                        bin_expr.visit_mut_children_with(self);
                        self.nodes.pop();
                        return;
                    }

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
                }
            }
            self.nodes.pop();
        }
    };
}

/// Create a fn inserts stmt counter for each stmt
#[macro_export]
macro_rules! insert_stmt_counter {
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
