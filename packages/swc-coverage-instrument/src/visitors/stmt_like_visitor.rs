use swc_core::{
    common::{comments::Comments, util::take::Take, SourceMapper},
    ecma::visit::{noop_visit_mut_type, VisitMut, VisitMutWith, VisitWith},
};

use crate::{
    create_instrumentation_visitor, instrumentation_counter_helper,
    instrumentation_stmt_counter_helper, instrumentation_visitor,
};

create_instrumentation_visitor!(StmtVisitor {});

impl<C: Clone + Comments, S: SourceMapper> StmtVisitor<C, S> {
    instrumentation_counter_helper!();
    instrumentation_stmt_counter_helper!();
}

impl<C: Clone + Comments, S: SourceMapper> VisitMut for StmtVisitor<C, S> {
    instrumentation_visitor!();
}
