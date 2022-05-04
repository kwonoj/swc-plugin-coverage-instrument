use crate::{
    create_instrumentation_visitor, instrumentation_counter_helper,
    instrumentation_stmt_counter_helper, instrumentation_visitor,
};

#[cfg(not(feature = "plugin"))]
use swc_common::{util::take::Take, Span, DUMMY_SP};
#[cfg(not(feature = "plugin"))]
use swc_ecma_ast::*;
#[cfg(not(feature = "plugin"))]
use swc_ecma_visit::*;

#[cfg(feature = "plugin")]
use swc_plugin::{
    ast::*,
    syntax_pos::{Span, DUMMY_SP},
    utils::take::Take,
};

create_instrumentation_visitor!(StmtVisitor {});

impl<C: Clone + Comments> StmtVisitor<C> {
    instrumentation_counter_helper!();
    instrumentation_stmt_counter_helper!();
}

impl<C: Clone + Comments> VisitMut for StmtVisitor<C> {
    instrumentation_visitor!();
}
