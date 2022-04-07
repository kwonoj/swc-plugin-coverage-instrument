use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use istanbul_oxi_instrument::{FileCoverage, COVERAGE_MAGIC_KEY, COVERAGE_MAGIC_VALUE};
use swc_plugin::{
    ast::*,
    syntax_pos::DUMMY_SP,
    utils::{quote, take::Take},
};

/// Returns `var $var_decl_name = $value;`
fn get_assignment_stmt(var_decl_name: &str, value: Expr) -> (Ident, Stmt) {
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

pub fn create_global_stmt_template(coverage_global_scope: &str) -> (Ident, Stmt) {
    // var global = new Function("return $global_coverage_scope")();
    let expr = Expr::New(NewExpr {
        callee: Box::new(Expr::Ident(Ident::new("Function".into(), DUMMY_SP))),
        args: Some(vec![ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Lit(Lit::Str(Str {
                value: format!("return {}", coverage_global_scope).into(),
                ..Str::dummy()
            }))),
        }]),
        ..NewExpr::dummy()
    });

    get_assignment_stmt(
        "global",
        Expr::Call(CallExpr {
            callee: Callee::Expr(Box::new(Expr::Paren(ParenExpr {
                span: DUMMY_SP,
                expr: Box::new(expr),
            }))),
            ..CallExpr::dummy()
        }),
    )
}

fn create_coverage_data_object(coverage_data: &FileCoverage) -> (String, Expr) {
    let mut props = vec![];

    // Afaik there's no built-in way to iterate over struct properties
    if coverage_data.all {
        let all_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(Ident::new("all".into(), DUMMY_SP)),
            value: Box::new(Expr::Lit(Lit::Bool(Bool::from(true)))),
        })));
        props.push(all_prop);
    }

    let path_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new("path".into(), DUMMY_SP)),
        value: Box::new(Expr::Lit(Lit::Str(Str::from(
            coverage_data.path.to_string(),
        )))),
    })));
    props.push(path_prop);

    let statement_map_prop_values = vec![];
    let statement_map_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new("statementMap".into(), DUMMY_SP)),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: statement_map_prop_values,
        })),
    })));
    props.push(statement_map_prop);

    let fn_map_prop_values = vec![];
    let fn_map_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new("fnMap".into(), DUMMY_SP)),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: fn_map_prop_values,
        })),
    })));
    props.push(fn_map_prop);

    let branch_map_prop_values = vec![];
    let branch_map_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new("branchMap".into(), DUMMY_SP)),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: branch_map_prop_values,
        })),
    })));
    props.push(branch_map_prop);

    let s_prop_values = vec![];
    let s_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new("s".into(), DUMMY_SP)),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: s_prop_values,
        })),
    })));
    props.push(s_prop);

    let f_prop_values = vec![];
    let f_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new("f".into(), DUMMY_SP)),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: f_prop_values,
        })),
    })));
    props.push(f_prop);

    let b_prop_values = vec![];
    let b_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new("b".into(), DUMMY_SP)),
        value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: b_prop_values,
        })),
    })));
    props.push(b_prop);

    if let Some(b_t) = &coverage_data.b_t {
        let b_t_prop_values = vec![];
        let b_t_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(Ident::new("bT".into(), DUMMY_SP)),
            value: Box::new(Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: b_t_prop_values,
            })),
        })));
        props.push(b_t_prop);
    }

    // fill in _coverageSchema, and hash
    let coverage_schema_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new(COVERAGE_MAGIC_KEY.clone().into(), DUMMY_SP)),
        value: Box::new(Expr::Lit(Lit::Str(Str::from(COVERAGE_MAGIC_VALUE.clone())))),
    })));
    props.push(coverage_schema_prop);

    // Original code creates hash against raw coverage object, but we use props ast instead.
    let mut hasher = DefaultHasher::new();
    props.hash(&mut hasher);
    let hash = hasher.finish().to_string();

    let hash_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(Ident::new("hash".into(), DUMMY_SP)),
        value: Box::new(Expr::Lit(Lit::Str(Str::from(hash.clone())))),
    })));
    props.push(hash_prop);

    (
        hash,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props,
        }),
    )
}

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
    let (path_ident, path_stmt) = get_assignment_stmt(
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
        get_assignment_stmt("hash", Expr::Lit(Lit::Str(Str::from(hash.clone()))));
    stmts.push(hash_stmt);

    // var global = new Function("return $global_coverage_scope")();
    stmts.push(coverage_template);

    // var gcv = ${coverage_variable};
    let (gcv_ident, gcv_stmt) = get_assignment_stmt(
        "gcv",
        Expr::Lit(Lit::Str(Str {
            value: coverage_variable.into(),
            ..Str::dummy()
        })),
    );
    stmts.push(gcv_stmt);

    // var coverageData = INITIAL;
    let (coverage_data_ident, coverage_data_stmt) =
        get_assignment_stmt("coverageData", coverage_data_object);
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
