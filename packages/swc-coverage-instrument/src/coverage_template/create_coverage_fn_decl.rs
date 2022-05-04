use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use istanbul_oxi_coverage::FileCoverage;
#[cfg(not(feature = "plugin"))]
use swc_common::{util::take::Take, DUMMY_SP};
#[cfg(not(feature = "plugin"))]
use swc_ecma_ast::*;
#[cfg(not(feature = "plugin"))]
use swc_ecma_quote::quote;

#[cfg(feature = "plugin")]
use swc_plugin::{
    ast::*,
    syntax_pos::DUMMY_SP,
    utils::{quote, take::Take},
};

use once_cell::sync::OnceCell;

use crate::constants::idents::*;

use crate::{create_assignment_stmt, create_coverage_data_object};

pub static COVERAGE_FN_IDENT: OnceCell<Ident> = OnceCell::new();
/// temporal ident being used for b_t true counter
pub static COVERAGE_FN_TRUE_TEMP_IDENT: OnceCell<Ident> = OnceCell::new();

/// Create a unique ident for the injected coverage counter fn,
/// Stores it into a global scope.
///
/// Do not use static value directly - create_instrumentation_visitor macro
/// should inject this into a struct accordingly.
pub fn create_coverage_fn_ident(value: &str) {
    let mut s = DefaultHasher::new();
    value.hash(&mut s);
    let var_name_hash = format!("cov_{}", s.finish());

    COVERAGE_FN_IDENT.get_or_init(|| Ident::new(var_name_hash.clone().into(), DUMMY_SP));
    COVERAGE_FN_TRUE_TEMP_IDENT
        .get_or_init(|| Ident::new(format!("{}_temp", var_name_hash).into(), DUMMY_SP));
}

/// Creates a function declaration for actual coverage collection.
pub fn create_coverage_fn_decl(
    coverage_variable: &str,
    coverage_template: Stmt,
    cov_fn_ident: &Ident,
    file_path: &str,
    coverage_data: &FileCoverage,
) -> Stmt {
    // Actual fn body statements will be injected
    let mut stmts = vec![];

    // var path = $file_path;
    let path_stmt = create_assignment_stmt(
        &IDENT_PATH,
        Expr::Lit(Lit::Str(Str {
            value: file_path.into(),
            ..Str::dummy()
        })),
    );
    stmts.push(path_stmt);

    let (hash, coverage_data_object) = create_coverage_data_object(coverage_data);

    // var hash = $HASH;
    let hash_stmt =
        create_assignment_stmt(&IDENT_HASH, Expr::Lit(Lit::Str(Str::from(hash.clone()))));
    stmts.push(hash_stmt);

    // var global = new Function("return $global_coverage_scope")();
    stmts.push(coverage_template);

    // var gcv = ${coverage_variable};
    let gcv_stmt = create_assignment_stmt(
        &IDENT_GCV,
        Expr::Lit(Lit::Str(Str {
            value: coverage_variable.into(),
            ..Str::dummy()
        })),
    );
    stmts.push(gcv_stmt);

    // var coverageData = INITIAL;
    let coverage_data_stmt = create_assignment_stmt(&IDENT_COVERAGE_DATA, coverage_data_object);
    stmts.push(coverage_data_stmt);

    let coverage_ident = Ident::new("coverage".into(), DUMMY_SP);
    stmts.push(quote!(
        "var $coverage = $global[$gcv] || ($global[$gcv] = {})" as Stmt,
        coverage = coverage_ident.clone(),
        gcv = IDENT_GCV.clone(),
        global = IDENT_GLOBAL.clone()
    ));

    stmts.push(quote!(
        r#"
if (!$coverage[$path] || $coverage[$path].$hash !== $hash) {
  $coverage[$path] = $coverage_data;
}
"# as Stmt,
        coverage = coverage_ident.clone(),
        path = IDENT_PATH.clone(),
        hash = IDENT_HASH.clone(),
        coverage_data = IDENT_COVERAGE_DATA.clone()
    ));

    // var actualCoverage = coverage[path];
    let actual_coverage_ident = Ident::new("actualCoverage".into(), DUMMY_SP);
    stmts.push(quote!(
        "var $actual_coverage = $coverage[$path];" as Stmt,
        actual_coverage = actual_coverage_ident.clone(),
        coverage = coverage_ident.clone(),
        path = IDENT_PATH.clone()
    ));

    //
    //COVERAGE_FUNCTION = function () {
    //   return actualCoverage;
    //}
    // TODO: need to add @ts-ignore leading comment
    let coverage_fn_assign_expr = Expr::Assign(AssignExpr {
        left: PatOrExpr::Pat(Box::new(Pat::Ident(BindingIdent::from(
            cov_fn_ident.clone(),
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
    Stmt::Decl(Decl::Fn(FnDecl {
        ident: cov_fn_ident.clone(),
        declare: false,
        function: Function {
            body: Some(BlockStmt {
                span: DUMMY_SP,
                stmts,
            }),
            ..Function::dummy()
        },
    }))
}
