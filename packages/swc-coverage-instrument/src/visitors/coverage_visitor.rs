use swc_core::{
    ast::*,
    common::{comments::Comments, util::take::Take, SourceMapper, DUMMY_SP},
    visit::{noop_visit_mut_type, VisitMut, VisitMutWith, VisitWith},
};
use tracing::instrument;

use crate::{
    create_instrumentation_visitor, instrumentation_counter_helper,
    instrumentation_stmt_counter_helper, instrumentation_visitor, InstrumentOptions,
};

create_instrumentation_visitor!(CoverageVisitor { file_path: String });

/// Public interface to create a visitor performs transform to inject
/// coverage instrumentation counter.
pub fn create_coverage_instrumentation_visitor<C: Clone + Comments, S: SourceMapper>(
    source_map: std::sync::Arc<S>,
    comments: C,
    instrument_options: InstrumentOptions,
    filename: String,
) -> CoverageVisitor<C, S> {
    // create a function name ident for the injected coverage instrumentation counters.
    crate::create_coverage_fn_ident(&filename);

    let mut cov = crate::SourceCoverage::new(filename.to_string(), instrument_options.report_logic);
    cov.set_input_source_map(&instrument_options.input_source_map);

    CoverageVisitor::new(
        source_map,
        comments.clone(),
        std::rc::Rc::new(std::cell::RefCell::new(cov)),
        instrument_options,
        vec![],
        None,
        filename,
    )
}

impl<C: Clone + Comments, S: SourceMapper> CoverageVisitor<C, S> {
    instrumentation_counter_helper!();
    instrumentation_stmt_counter_helper!();

    /// Not implemented.
    /// TODO: is this required?
    fn is_instrumented_already(&self) -> bool {
        return false;
    }

    /// Create coverage instrumentation template exprs to be injected into the top of the transformed output.
    fn get_coverage_templates(&mut self) -> (Stmt, Stmt) {
        self.cov.borrow_mut().freeze();

        //TODO: option: global coverage variable scope. (optional, default `this`)
        let coverage_global_scope = "this";
        //TODO: option: use an evaluated function to find coverageGlobalScope.
        let coverage_global_scope_func = true;

        let gv_template = if coverage_global_scope_func {
            // TODO: path.scope.getBinding('Function')
            let is_function_binding_scope = false;

            if is_function_binding_scope {
                /*
                gvTemplate = globalTemplateAlteredFunction({
                    GLOBAL_COVERAGE_SCOPE: T.stringLiteral(
                        'return ' + opts.coverageGlobalScope
                    )
                });
                 */
                unimplemented!("");
            } else {
                crate::create_global_stmt_template(coverage_global_scope)
            }
        } else {
            unimplemented!("");
            /*
            gvTemplate = globalTemplateVariable({
                GLOBAL_COVERAGE_SCOPE: opts.coverageGlobalScope
            });
            */
        };

        let coverage_template = crate::create_coverage_fn_decl(
            &self.instrument_options.coverage_variable,
            gv_template,
            &self.cov_fn_ident,
            &self.file_path,
            self.cov.borrow().as_ref(),
            &self.comments,
            self.instrument_options.debug_initial_coverage_comment,
        );

        // explicitly call this.varName to ensure coverage is always initialized
        let call_coverage_template_stmt = Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Call(CallExpr {
                callee: Callee::Expr(Box::new(Expr::Ident(self.cov_fn_ident.clone()))),
                ..CallExpr::dummy()
            })),
        });

        (coverage_template, call_coverage_template_stmt)
    }
}

impl<C: Clone + Comments, S: SourceMapper> VisitMut for CoverageVisitor<C, S> {
    instrumentation_visitor!();

    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_program(&mut self, program: &mut Program) {
        self.nodes.push(crate::Node::Program);
        if crate::hint_comments::should_ignore_file(&self.comments, program) {
            return;
        }

        if self.is_instrumented_already() {
            return;
        }

        program.visit_mut_children_with(self);
        self.nodes.pop();
    }

    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
        if self.is_instrumented_already() {
            return;
        }

        let root_exists = match self.nodes.get(0) {
            Some(node) => node == &crate::Node::Program,
            _ => false,
        };

        // Articulate root by injecting Program node if visut_mut_program is not called.
        // TODO: Need to figure out why custom_js_pass doesn't hit visit_mut_program
        // instead of manually injecting node here
        if !root_exists {
            let mut new_nodes = vec![crate::Node::Program];
            new_nodes.extend(self.nodes.drain(..));
            self.nodes = new_nodes;
        }

        // TODO: Should module_items need to be added in self.nodes?
        let mut new_items = vec![];
        for mut item in items.drain(..) {
            let (old, _ignore_current) = match &mut item {
                ModuleItem::ModuleDecl(decl) => self.on_enter(decl),
                ModuleItem::Stmt(stmt) => self.on_enter(stmt),
            };
            item.visit_mut_children_with(self);

            new_items.extend(self.before.drain(..).map(|v| ModuleItem::Stmt(v)));
            new_items.push(item);
            self.on_exit(old);
        }
        *items = new_items;

        let (coverage_template, call_coverage_template_stmt) = self.get_coverage_templates();

        // prepend template to the top of the code
        items.insert(0, ModuleItem::Stmt(coverage_template));
        items.insert(1, ModuleItem::Stmt(call_coverage_template_stmt));

        if !root_exists {
            self.nodes.pop();
        }
    }

    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_script(&mut self, items: &mut Script) {
        if self.is_instrumented_already() {
            return;
        }

        let mut new_items = vec![];
        for mut item in items.body.drain(..) {
            item.visit_mut_children_with(self);
            new_items.extend(self.before.drain(..));
            new_items.push(item);
        }
        items.body = new_items;

        let (coverage_template, call_coverage_template_stmt) = self.get_coverage_templates();

        // prepend template to the top of the code
        items.body.insert(0, coverage_template);
        items.body.insert(1, call_coverage_template_stmt);
    }

    // ExportDefaultDeclaration: entries(), // ignore processing only
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_export_default_decl(&mut self, export_default_decl: &mut ExportDefaultDecl) {
        let (old, ignore_current) = self.on_enter(export_default_decl);
        match ignore_current {
            Some(crate::hint_comments::IgnoreScope::Next) => {}
            _ => {
                // noop
                export_default_decl.visit_mut_children_with(self);
            }
        }
        self.on_exit(old);
    }

    // ExportNamedDeclaration: entries(), // ignore processing only
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_export_decl(&mut self, export_named_decl: &mut ExportDecl) {
        let (old, ignore_current) = self.on_enter(export_named_decl);
        match ignore_current {
            Some(crate::hint_comments::IgnoreScope::Next) => {}
            _ => {
                // noop
                export_named_decl.visit_mut_children_with(self);
            }
        }
        self.on_exit(old);
    }

    // DebuggerStatement: entries(coverStatement),
    #[instrument(skip_all, fields(node = %self.print_node()))]
    fn visit_mut_debugger_stmt(&mut self, debugger_stmt: &mut DebuggerStmt) {
        let (old, ignore_current) = self.on_enter(debugger_stmt);
        match ignore_current {
            Some(crate::hint_comments::IgnoreScope::Next) => {}
            _ => {
                debugger_stmt.visit_mut_children_with(self);
            }
        }
        self.on_exit(old);
    }

    // ConditionalExpression: entries(coverTernary),
    #[instrument(skip_all, fields(node = %self.print_node()))]
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
                    crate::lookup_range::get_expr_span(&*cond_expr.cons),
                );
                let a_hint = crate::hint_comments::lookup_hint_comments(
                    &self.comments,
                    crate::lookup_range::get_expr_span(&*cond_expr.alt),
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
}
