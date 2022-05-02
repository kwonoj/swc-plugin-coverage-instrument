use std::fmt::{Display, Formatter};

#[derive(Copy, Debug, Clone, PartialEq)]
pub enum Node {
    Program,
    ModuleDecl,
    Stmt,
    Stmts,
    Expr,
    VarDeclarator,
    ExprStmt,
    ModuleItems,
    ArrowExpr,
    SetterProp,
    GetterProp,
    MethodProp,
    BinExpr,
    LogicalExpr,
    CondExpr,
    LabeledStmt,
    FnExpr,
    FnDecl,
    WithStmt,
    SwitchCase,
    SwitchStmt,
    DoWhileStmt,
    WhileStmt,
    ForOfStmt,
    ForInStmt,
    ForStmt,
    IfStmt,
    VarDecl,
    TryStmt,
    ThrowStmt,
    ReturnStmt,
    DebuggerStmt,
    ContinueStmt,
    BreakStmt,
    PrivateProp,
    ClassProp,
    ClassDecl,
    ClassMethod,
    ExportDecl,
    ExportDefaultDecl,
    BlockStmt,
    AssignPat,
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}
