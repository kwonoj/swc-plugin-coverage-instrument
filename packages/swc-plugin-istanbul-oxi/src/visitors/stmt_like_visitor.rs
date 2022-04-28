use istanbul_oxi_instrument::SourceCoverage;
use swc_plugin::{
    ast::*,
    comments::PluginCommentsProxy,
    source_map::PluginSourceMapProxy,
    syntax_pos::{Span, DUMMY_SP},
    utils::take::Take,
};
use tracing::instrument;

use crate::{
    constants::idents::*,
    insert_counter_helper,
    instrument::create_increase_expression_expr,
    utils::{
        lookup_range::{get_expr_span, get_range_from_span},
        node::Node,
    },
    visit_mut_coverage, InstrumentOptions,
};

pub struct StmtVisitor<'a> {
    pub source_map: &'a PluginSourceMapProxy,
    pub comments: Option<&'a PluginCommentsProxy>,
    pub cov: &'a mut SourceCoverage,
    pub var_name_ident: Ident,
    pub instrument_options: InstrumentOptions,
    pub before: Vec<Stmt>,
    pub nodes: Vec<Node>,
}

// TODO: duplicated path between CoverageVisitor
impl<'a> StmtVisitor<'a> {
    pub fn new(
        source_map: &'a PluginSourceMapProxy,
        comments: Option<&'a PluginCommentsProxy>,
        cov: &'a mut SourceCoverage,
        var_name_ident: &'a Ident,
        instrument_options: &InstrumentOptions,
        current_node: &[Node],
    ) -> StmtVisitor<'a> {
        StmtVisitor {
            source_map,
            comments,
            cov,
            var_name_ident: var_name_ident.clone(),
            instrument_options: instrument_options.clone(),
            before: vec![],
            nodes: current_node.to_vec(),
        }
    }

    insert_counter_helper!();

    /// Visit individual statements with stmt_visitor and update.
    fn insert_stmts_counter(&mut self, stmts: &mut Vec<Stmt>) {
        /*for stmt in stmts {
            if !self.is_injected_counter_stmt(&stmt) {
                let span = crate::utils::lookup_range::get_stmt_span(&stmt);
                if let Some(span) = span {
                    let increment_expr = self.create_stmt_increase_counter_expr(span, None);

                    self.before.push(Stmt::Expr(ExprStmt {
                        span: DUMMY_SP,
                        expr: Box::new(increment_expr),
                    }));
                }
            }
        }*/
        let mut new_stmts = vec![];

        for mut stmt in stmts.drain(..) {
            if !self.is_injected_counter_stmt(&stmt) {
                let _span = crate::utils::lookup_range::get_stmt_span(&stmt);
                let mut visitor = StmtVisitor::new(
                    self.source_map,
                    self.comments,
                    &mut self.cov,
                    &self.var_name_ident,
                    &self.instrument_options,
                    &self.nodes,
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
}

impl VisitMut for StmtVisitor<'_> {
    visit_mut_coverage!();
}
