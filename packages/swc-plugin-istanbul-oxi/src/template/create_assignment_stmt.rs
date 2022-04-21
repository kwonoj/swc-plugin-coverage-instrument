use swc_plugin::{ast::*, syntax_pos::DUMMY_SP, utils::take::Take};

/// Create an assignment stmt AST for `var $var_decl_name = $value;`
pub fn create_assignment_stmt(var_decl_name: &str, value: Expr) -> (Ident, Stmt) {
    let ident = Ident::new(var_decl_name.into(), DUMMY_SP);

    let stmt = Stmt::Decl(Decl::Var(VarDecl {
        kind: VarDeclKind::Var,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Assign(AssignPat {
                span: DUMMY_SP,
                left: Box::new(Pat::Ident(BindingIdent::from(ident.clone()))),
                right: Box::new(value),
                type_ann: None,
            }),
            init: None,
            definite: false,
        }],
        ..VarDecl::dummy()
    }));

    (ident, stmt)
}
