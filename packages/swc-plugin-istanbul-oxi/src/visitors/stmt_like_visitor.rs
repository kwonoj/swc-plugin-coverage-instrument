use istanbul_oxi_instrument::SourceCoverage;
use swc_plugin::{
    ast::*,
    syntax_pos::{Span, DUMMY_SP},
    utils::take::Take,
};
use tracing::instrument;

use crate::{
    constants::idents::*,
    create_coverage_visitor, insert_counter_helper, insert_logical_expr_helper,
    instrument::create_increase_expression_expr,
    utils::{
        lookup_range::{get_expr_span, get_range_from_span},
        node::Node,
    },
    visit_mut_coverage,
};

create_coverage_visitor!(StmtVisitor {});

impl<'a> StmtVisitor<'a> {
    insert_logical_expr_helper!();
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
                    &self.instrument_options,
                    &self.nodes,
                    false,
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
