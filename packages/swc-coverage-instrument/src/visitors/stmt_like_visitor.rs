use crate::{
    create_instrumentation_visitor, instrumentation_counter_helper,
    instrumentation_stmt_counter_helper, instrumentation_visitor,
};

use swc_plugin::{ast::*, syntax_pos::DUMMY_SP, utils::take::Take};

create_instrumentation_visitor!(StmtVisitor {});

impl<C: Clone + Comments, S: SourceMapper> StmtVisitor<C, S> {
    instrumentation_counter_helper!();
    instrumentation_stmt_counter_helper!();
}

impl<C: Clone + Comments, S: SourceMapper> VisitMut for StmtVisitor<C, S> {
    instrumentation_visitor!();
}
