use swc_core::{
    ast::*,
    visit::{Visit, VisitWith},
};

/// A visitor to check if counter need to be `hoisted` for certain types of nodes.
#[derive(Debug)]
pub struct HoistingFinder(pub bool);

impl HoistingFinder {
    pub fn new() -> HoistingFinder {
        HoistingFinder(false)
    }
}

impl Visit for HoistingFinder {
    fn visit_fn_expr(&mut self, _fn_expr: &FnExpr) {
        self.0 = true;
    }

    fn visit_arrow_expr(&mut self, _arrow_expr: &ArrowExpr) {
        self.0 = true;
    }

    fn visit_class_expr(&mut self, _class_expr: &ClassExpr) {
        self.0 = true;
    }
}

/// Check if nodes have block statements.
#[derive(Debug)]
pub struct BlockStmtFinder(pub bool);

impl BlockStmtFinder {
    pub fn new() -> BlockStmtFinder {
        BlockStmtFinder(false)
    }
}

impl Visit for BlockStmtFinder {
    fn visit_block_stmt(&mut self, _block: &BlockStmt) {
        self.0 = true;
    }
}

#[derive(Debug)]
pub struct StmtFinder(pub bool);

impl StmtFinder {
    pub fn new() -> StmtFinder {
        StmtFinder(false)
    }
}

impl Visit for StmtFinder {
    fn visit_stmt(&mut self, _block: &Stmt) {
        self.0 = true;
    }
}

// Check a node have expressions.
#[derive(Debug)]
pub struct ExprFinder(pub bool);

impl ExprFinder {
    pub fn new() -> ExprFinder {
        ExprFinder(false)
    }
}

impl Visit for ExprFinder {
    fn visit_expr(&mut self, _block: &Expr) {
        self.0 = true;
    }
}

/// Traverse down given nodes to check if it's leaf of the logical expr,
/// or have inner logical expr to recurse.
#[derive(Debug)]
pub struct LogicalExprLeafFinder(pub bool);

impl Visit for LogicalExprLeafFinder {
    fn visit_bin_expr(&mut self, bin_expr: &BinExpr) {
        match &bin_expr.op {
            BinaryOp::LogicalOr | BinaryOp::LogicalAnd | BinaryOp::NullishCoalescing => {
                self.0 = true;
                // short curcuit, we know it's not leaf
                return;
            }
            _ => {}
        }

        bin_expr.visit_children_with(self);
    }
}
