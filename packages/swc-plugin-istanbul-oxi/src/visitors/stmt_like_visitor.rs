use crate::{
    create_instrumentation_visitor, instrumentation_counter_helper,
    instrumentation_stmt_counter_helper, instrumentation_visitor,
};

create_instrumentation_visitor!(StmtVisitor {});

impl<'a> StmtVisitor<'a> {
    instrumentation_counter_helper!();
    instrumentation_stmt_counter_helper!();
}

impl VisitMut for StmtVisitor<'_> {
    instrumentation_visitor!();
}
