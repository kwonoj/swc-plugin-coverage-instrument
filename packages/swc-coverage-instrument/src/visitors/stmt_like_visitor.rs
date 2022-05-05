use crate::{
    create_instrumentation_visitor, instrumentation_counter_helper,
    instrumentation_stmt_counter_helper, instrumentation_visitor,
};

use swc_plugin::{ast::*, syntax_pos::DUMMY_SP, utils::take::Take};

create_instrumentation_visitor!(StmtVisitor {});

impl<C: Clone + Comments> StmtVisitor<C> {
    instrumentation_counter_helper!();
    instrumentation_stmt_counter_helper!();
}

impl<C: Clone + Comments> VisitMut for StmtVisitor<C> {
    instrumentation_visitor!();
}
