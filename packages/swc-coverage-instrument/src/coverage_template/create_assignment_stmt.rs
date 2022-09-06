use swc_core::{
    common::{util::take::Take, DUMMY_SP},
    ecma::ast::*,
};

/// Create an assignment stmt AST for `var $var_decl_ident = $value;`
pub fn create_assignment_stmt(var_decl_ident: &Ident, value: Expr) -> Stmt {
    Stmt::Decl(Decl::Var(VarDecl {
        kind: VarDeclKind::Var,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Assign(AssignPat {
                span: DUMMY_SP,
                left: Box::new(Pat::Ident(BindingIdent::from(var_decl_ident.clone()))),
                right: Box::new(value),
                type_ann: None,
            }),
            init: None,
            definite: false,
        }],
        ..VarDecl::dummy()
    }))
}
