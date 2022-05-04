/// Expand given struct to contain necessary common filed for the coverage visitor
/// with common utility functions.
///
/// This does not impl actual visitors (VisitMut) as each visitor may have different
/// visitor logics.
#[macro_export]
macro_rules! create_instrumentation_visitor {
    ($name:ident { $($vis: vis $field:ident: $t:ty),* $(,)? }) => {
        #[cfg(not(feature = "plugin"))]
        use swc_common::{Span as SyntaxPosSpan,
            comments::Comments,
            SourceMap as SourceMapImpl
        };
        #[cfg(not(feature = "plugin"))]
        use swc_ecma_ast::{Stmt, Ident};

        #[cfg(feature = "plugin")]
        use swc_plugin::{syntax_pos::Span as SyntaxPosSpan,
            source_map::PluginSourceMapProxy as SourceMapImpl,
            comments::Comments
        };
        #[cfg(feature = "plugin")]
        use swc_plugin::ast::{Stmt, Ident};

        // Declare a struct, expand fields commonly used for any instrumentation visitor.
        pub struct $name<C: Clone + Comments> {
            // We may not need Arc in the plugin context - this is only to preserve isomorphic interface
            // between plugin & custom transform pass.
            source_map: std::sync::Arc<SourceMapImpl>,
            comments: C,
            cov: std::rc::Rc<std::cell::RefCell<crate::SourceCoverage>>,
            cov_fn_ident: Ident,
            cov_fn_temp_ident: Ident,
            instrument_options: crate::InstrumentOptions,
            // Current visitor state to hold stmts to be prepended by parent node.
            pub before: Vec<Stmt>,
            nodes: Vec<crate::Node>,
            should_ignore: Option<crate::hint_comments::IgnoreScope>,
            $($vis $field: $t,)*
        }

        impl<C: Clone + Comments> $name<C> {
            pub fn new(
                source_map: &std::sync::Arc<SourceMapImpl>,
                comments: C,
                cov: &std::rc::Rc<std::cell::RefCell<crate::SourceCoverage>>,
                instrument_options: &crate::InstrumentOptions,
                nodes: &Vec<crate::Node>,
                should_ignore: Option<crate::hint_comments::IgnoreScope>,
                $($field: $t,)*
            ) -> $name<C> {
                $name {
                    source_map: source_map.clone(),
                    comments: comments.clone(),
                    cov: cov.clone(),
                    cov_fn_ident: crate::COVERAGE_FN_IDENT.get().expect("Coverage fn Ident should be initialized already").clone(),
                    cov_fn_temp_ident: crate::COVERAGE_FN_TRUE_TEMP_IDENT.get().expect("Coverage fn Ident should be initialized already").clone(),
                    instrument_options: instrument_options.clone(),
                    before: vec![],
                    nodes: nodes.clone(),
                    should_ignore,
                    $($field,)*
                }
            }

            // Display current nodes.
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
                    "".to_string()
                }
            }

            fn on_enter_with_span(&mut self, span: Option<&SyntaxPosSpan>) -> (Option<crate::hint_comments::IgnoreScope>, Option<crate::hint_comments::IgnoreScope>) {
                let old = self.should_ignore;
                let ret = match old {
                    Some(crate::hint_comments::IgnoreScope::Next) => old,
                    _ => {
                        self.should_ignore = crate::hint_comments::should_ignore(&self.comments, span);
                        self.should_ignore
                    }
                };

                (old, ret)
            }

            fn on_exit(&mut self, old: Option<crate::hint_comments::IgnoreScope>) {
                self.should_ignore = old;
                self.nodes.pop();
            }
        }


        /// A trait expands to the ast types we want to use to determine if we need to ignore
        /// certain section of the code for the instrumentation.
        /// TODO: Can a macro like `on_visit_mut_expr` expands on_enter / exit automatically?
        /// `on_visit_mut_expr!(|expr| {self.xxx})` doesn't seem to work.
        trait CoverageInstrumentationMutVisitEnter<N> {
            fn on_enter(&mut self, n: &mut N) -> (Option<crate::hint_comments::IgnoreScope>, Option<crate::hint_comments::IgnoreScope>);
        }

        // Macro generates trait impl for the type can access span directly.
        macro_rules! on_enter {
            ($N: tt) => {
                impl<C: Clone + Comments> CoverageInstrumentationMutVisitEnter<$N> for $name<C> {
                    #[cfg(feature = "plugin")]
                    #[inline]
                    fn on_enter(&mut self, n: &mut swc_plugin::ast::$N) -> (Option<crate::hint_comments::IgnoreScope>, Option<crate::hint_comments::IgnoreScope>) {
                        self.nodes.push(crate::Node::$N);
                        self.on_enter_with_span(Some(&n.span))
                    }

                    #[cfg(not(feature = "plugin"))]
                    #[inline]
                    fn on_enter(&mut self, n: &mut swc_ecma_ast::$N) -> (Option<crate::hint_comments::IgnoreScope>, Option<crate::hint_comments::IgnoreScope>) {
                        self.nodes.push(crate::Node::$N);
                        self.on_enter_with_span(Some(&n.span))
                    }
                 }
            }
        }

        impl<C: Clone + Comments> CoverageInstrumentationMutVisitEnter<Expr> for $name<C> {
            fn on_enter(&mut self, n: &mut Expr) -> (Option<crate::hint_comments::IgnoreScope>, Option<crate::hint_comments::IgnoreScope>) {
                self.nodes.push(crate::Node::Expr);
                let span = crate::lookup_range::get_expr_span(n);
                self.on_enter_with_span(span)
            }
         }

         impl<C: Clone + Comments> CoverageInstrumentationMutVisitEnter<Stmt> for $name<C> {
            fn on_enter(&mut self, n: &mut Stmt) -> (Option<crate::hint_comments::IgnoreScope>, Option<crate::hint_comments::IgnoreScope>) {
                self.nodes.push(crate::Node::Stmt);
                let span = crate::lookup_range::get_stmt_span(n);

                self.on_enter_with_span(span)
            }
         }

         impl<C: Clone + Comments> CoverageInstrumentationMutVisitEnter<ModuleDecl> for $name<C> {
            fn on_enter(&mut self, n: &mut ModuleDecl) -> (Option<crate::hint_comments::IgnoreScope>, Option<crate::hint_comments::IgnoreScope>) {
                self.nodes.push(crate::Node::ModuleDecl);
                let span = crate::lookup_range::get_module_decl_span(n);

                self.on_enter_with_span(span)
            }
         }

         impl<C: Clone + Comments> CoverageInstrumentationMutVisitEnter<ClassDecl> for $name<C> {
            fn on_enter(&mut self, n: &mut ClassDecl) -> (Option<crate::hint_comments::IgnoreScope>, Option<crate::hint_comments::IgnoreScope>) {
                self.nodes.push(crate::Node::ClassDecl);
                self.on_enter_with_span(Some(&n.class.span))
            }
         }

         impl<C: Clone + Comments> CoverageInstrumentationMutVisitEnter<FnExpr> for $name<C> {
            fn on_enter(&mut self, n: &mut FnExpr) -> (Option<crate::hint_comments::IgnoreScope>, Option<crate::hint_comments::IgnoreScope>) {
                self.nodes.push(crate::Node::FnExpr);
                self.on_enter_with_span(Some(&n.function.span))
            }
         }

         impl<C: Clone + Comments> CoverageInstrumentationMutVisitEnter<MethodProp> for $name<C> {
            fn on_enter(&mut self, n: &mut MethodProp) -> (Option<crate::hint_comments::IgnoreScope>, Option<crate::hint_comments::IgnoreScope>) {
                self.nodes.push(crate::Node::MethodProp);
                self.on_enter_with_span(Some(&n.function.span))
            }
         }

         impl<C: Clone + Comments> CoverageInstrumentationMutVisitEnter<FnDecl> for $name<C> {
            fn on_enter(&mut self, n: &mut FnDecl) -> (Option<crate::hint_comments::IgnoreScope>, Option<crate::hint_comments::IgnoreScope>) {
                self.nodes.push(crate::Node::FnDecl);
                self.on_enter_with_span(Some(&n.function.span))
            }
         }

         on_enter!(BinExpr);
         on_enter!(VarDeclarator);
         on_enter!(VarDecl);
         on_enter!(CondExpr);
         on_enter!(ExprStmt);
         on_enter!(IfStmt);
         on_enter!(LabeledStmt);
         on_enter!(ContinueStmt);
         on_enter!(ClassProp);
         on_enter!(PrivateProp);
         on_enter!(ClassMethod);
         on_enter!(ArrowExpr);
         on_enter!(ForStmt);
         on_enter!(ForOfStmt);
         on_enter!(ForInStmt);
         on_enter!(WhileStmt);
         on_enter!(DoWhileStmt);
         on_enter!(SwitchStmt);
         on_enter!(SwitchCase);
         on_enter!(BreakStmt);
         on_enter!(ReturnStmt);
         on_enter!(BlockStmt);
         on_enter!(WithStmt);
         on_enter!(TryStmt);
         on_enter!(ThrowStmt);
         on_enter!(ExportDecl);
         on_enter!(ExportDefaultDecl);
         on_enter!(DebuggerStmt);
         on_enter!(AssignPat);
         on_enter!(GetterProp);
         on_enter!(SetterProp);
    }
}
