use istanbul_oxi_instrument::FileCoverage;
use swc_plugin::{
    ast::*,
    syntax_pos::DUMMY_SP,
    utils::{quote, take::Take},
};

use super::{
    create_assignment_stmt::create_assignment_stmt,
    create_coverage_data_object::create_coverage_data_object,
};

/// Creates a function declaration for actual coverage collection.
pub fn create_coverage_fn_decl(
    coverage_variable: &str,
    global_ident: Ident,
    coverage_template: Stmt,
    var_name: &str,
    file_path: &str,
    coverage_data: &FileCoverage,
) -> (Ident, ModuleItem) {
    // Actual fn body statements will be injected
    let mut stmts = vec![];

    // var path = $file_path;
    let (path_ident, path_stmt) = create_assignment_stmt(
        "path",
        Expr::Lit(Lit::Str(Str {
            value: file_path.into(),
            ..Str::dummy()
        })),
    );
    stmts.push(path_stmt);

    let (hash, coverage_data_object) = create_coverage_data_object(coverage_data);

    // var hash = $HASH;
    let (hash_ident, hash_stmt) =
        create_assignment_stmt("hash", Expr::Lit(Lit::Str(Str::from(hash.clone()))));
    stmts.push(hash_stmt);

    // var global = new Function("return $global_coverage_scope")();
    stmts.push(coverage_template);

    // var gcv = ${coverage_variable};
    let (gcv_ident, gcv_stmt) = create_assignment_stmt(
        "gcv",
        Expr::Lit(Lit::Str(Str {
            value: coverage_variable.into(),
            ..Str::dummy()
        })),
    );
    stmts.push(gcv_stmt);

    // var coverageData = INITIAL;
    let (coverage_data_ident, coverage_data_stmt) =
        create_assignment_stmt("coverageData", coverage_data_object);
    stmts.push(coverage_data_stmt);

    let coverage_ident = Ident::new("coverage".into(), DUMMY_SP);
    stmts.push(quote!(
        "var $coverage = $global[$gcv] || ($global[$gcv] = {})" as Stmt,
        coverage = coverage_ident.clone(),
        gcv = gcv_ident.clone(),
        global = global_ident.clone()
    ));

    stmts.push(quote!(
        r#"
if (!$coverage[$path] || $coverage[$path].$hash !== $hash) {
  $coverage[$path] = $coverage_data;
}
"# as Stmt,
        coverage = coverage_ident.clone(),
        path = path_ident.clone(),
        hash = hash_ident.clone(),
        coverage_data = coverage_data_ident.clone()
    ));

    // var actualCoverage = coverage[path];
    let actual_coverage_ident = Ident::new("actualCoverage".into(), DUMMY_SP);
    stmts.push(quote!(
        "var $actual_coverage = $coverage[$path];" as Stmt,
        actual_coverage = actual_coverage_ident.clone(),
        coverage = coverage_ident.clone(),
        path = path_ident.clone()
    ));

    let coverage_fn_ident = Ident::new(var_name.into(), DUMMY_SP);
    //
    //COVERAGE_FUNCTION = function () {
    //   return actualCoverage;
    //}
    // TODO: need to add @ts-ignore leading comment
    let coverage_fn_assign_expr = Expr::Assign(AssignExpr {
        left: PatOrExpr::Pat(Box::new(Pat::Ident(BindingIdent::from(
            coverage_fn_ident.clone(),
        )))),
        right: Box::new(Expr::Fn(FnExpr {
            ident: None,
            function: Function {
                body: Some(BlockStmt {
                    span: DUMMY_SP,
                    stmts: vec![Stmt::Return(ReturnStmt {
                        span: DUMMY_SP,
                        arg: Some(Box::new(Expr::Ident(actual_coverage_ident.clone()))),
                    })],
                }),
                ..Function::dummy()
            },
        })),
        ..AssignExpr::dummy()
    });

    stmts.push(Stmt::Block(BlockStmt {
        stmts: vec![Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(coverage_fn_assign_expr),
        })],
        ..BlockStmt::dummy()
    }));

    stmts.push(Stmt::Return(ReturnStmt {
        span: DUMMY_SP,
        arg: Some(Box::new(Expr::Ident(actual_coverage_ident.clone()))),
    }));

    // moduleitem for fn decl includes body defined above
    let module_item = ModuleItem::Stmt(Stmt::Decl(Decl::Fn(FnDecl {
        ident: coverage_fn_ident.clone(),
        declare: false,
        function: Function {
            body: Some(BlockStmt {
                span: DUMMY_SP,
                stmts,
            }),
            ..Function::dummy()
        },
    })));

    (coverage_fn_ident, module_item)
}
