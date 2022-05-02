/// Expand given struct to contain necessary common filed for the coverage visitor
/// with common utility functions.
///
/// This does not impl actual visitors (VisitMut) as each visitor may have different
/// visitor logics.
#[macro_export]
macro_rules! create_instrumentation_visitor {
    ($name:ident { $($vis: vis $field:ident: $t:ty),* $(,)? }) => {
        // Declare a struct, expand fields commonly used for any instrumentation visitor.
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
            should_ignore: Option<crate::utils::hint_comments::IgnoreScope>,
            $($vis $field: $t,)*
        }

        impl<'a> $name<'a> {
            pub fn new(
                source_map: &'a swc_plugin::source_map::PluginSourceMapProxy,
                comments: Option<&'a swc_plugin::comments::PluginCommentsProxy>,
                cov: &'a mut istanbul_oxi_instrument::SourceCoverage,
                instrument_options: &'a crate::InstrumentOptions,
                nodes: &'a Vec<Node>,
                should_ignore: Option<crate::utils::hint_comments::IgnoreScope>,
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

            fn on_enter_with_span(&mut self, span: Option<&Span>) -> (Option<crate::utils::hint_comments::IgnoreScope>, Option<crate::utils::hint_comments::IgnoreScope>) {
                let old = self.should_ignore;
                let ret = match old {
                    Some(crate::utils::hint_comments::IgnoreScope::Next) => old,
                    _ => {
                        self.should_ignore = crate::utils::hint_comments::should_ignore(&self.comments, span);
                        self.should_ignore
                    }
                };

                (old, ret)
            }

            fn on_exit(&mut self, old: Option<crate::utils::hint_comments::IgnoreScope>) {
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
            fn on_enter(&mut self, n: &mut N) -> (Option<crate::utils::hint_comments::IgnoreScope>, Option<crate::utils::hint_comments::IgnoreScope>);
        }

        // Macro generates trait impl for the type can access span directly.
        macro_rules! on_enter {
            ($N: tt) => {
                impl CoverageInstrumentationMutVisitEnter<$N> for $name<'_> {
                    #[inline]
                    fn on_enter(&mut self, n: &mut swc_plugin::ast::$N) -> (Option<crate::utils::hint_comments::IgnoreScope>, Option<crate::utils::hint_comments::IgnoreScope>) {
                        self.nodes.push(Node::$N);
                        self.on_enter_with_span(Some(&n.span))
                    }
                 }
            }
        }

        impl CoverageInstrumentationMutVisitEnter<Expr> for $name<'_> {
            fn on_enter(&mut self, n: &mut Expr) -> (Option<crate::utils::hint_comments::IgnoreScope>, Option<crate::utils::hint_comments::IgnoreScope>) {
                self.nodes.push(Node::Expr);
                let span = crate::utils::lookup_range::get_expr_span(n);
                self.on_enter_with_span(span)
            }
         }

         impl CoverageInstrumentationMutVisitEnter<Stmt> for $name<'_> {
            fn on_enter(&mut self, n: &mut Stmt) -> (Option<crate::utils::hint_comments::IgnoreScope>, Option<crate::utils::hint_comments::IgnoreScope>) {
                self.nodes.push(Node::Stmt);
                let span = crate::utils::lookup_range::get_stmt_span(n);

                self.on_enter_with_span(span)
            }
         }

         impl CoverageInstrumentationMutVisitEnter<ModuleDecl> for $name<'_> {
            fn on_enter(&mut self, n: &mut ModuleDecl) -> (Option<crate::utils::hint_comments::IgnoreScope>, Option<crate::utils::hint_comments::IgnoreScope>) {
                self.nodes.push(Node::ModuleDecl);
                let span = crate::utils::lookup_range::get_module_decl_span(n);

                self.on_enter_with_span(span)
            }
         }

         impl CoverageInstrumentationMutVisitEnter<ClassDecl> for $name<'_> {
            fn on_enter(&mut self, n: &mut ClassDecl) -> (Option<crate::utils::hint_comments::IgnoreScope>, Option<crate::utils::hint_comments::IgnoreScope>) {
                self.nodes.push(Node::ClassDecl);
                self.on_enter_with_span(Some(&n.class.span))
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
    }
}
