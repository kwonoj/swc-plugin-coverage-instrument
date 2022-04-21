use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use istanbul_oxi_instrument::{FileCoverage, Range, COVERAGE_MAGIC_VALUE};
use swc_plugin::{ast::*, syntax_pos::DUMMY_SP};

use crate::{
    constants::idents::*,
    utils::ast_builder::{
        create_ident_key_value_prop, create_num_lit_expr, create_str_key_value_prop,
        create_str_lit_expr,
    },
};

fn create_range_object_lit(value: &Range) -> Expr {
    Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: vec![
            create_ident_key_value_prop(
                &IDENT_START,
                Expr::Object(ObjectLit {
                    span: DUMMY_SP,
                    props: vec![
                        create_ident_key_value_prop(
                            &IDENT_LINE,
                            create_num_lit_expr(value.start.line),
                        ),
                        create_ident_key_value_prop(
                            &IDENT_COLUMN,
                            create_num_lit_expr(value.start.column),
                        ),
                    ],
                }),
            ),
            create_ident_key_value_prop(
                &IDENT_END,
                Expr::Object(ObjectLit {
                    span: DUMMY_SP,
                    props: vec![
                        create_ident_key_value_prop(
                            &IDENT_LINE,
                            create_num_lit_expr(value.end.line),
                        ),
                        create_ident_key_value_prop(
                            &IDENT_COLUMN,
                            create_num_lit_expr(value.end.column),
                        ),
                    ],
                }),
            ),
        ],
    })
}

fn create_fn_prop(key: &str, value: &istanbul_oxi_instrument::Function) -> PropOrSpread {
    create_str_key_value_prop(
        key,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![
                create_ident_key_value_prop(&IDENT_NAME, create_str_lit_expr(&value.name)),
                create_ident_key_value_prop(&IDENT_DECL, create_range_object_lit(&value.decl)),
                create_ident_key_value_prop(&IDENT_LOC, create_range_object_lit(&value.loc)),
                create_ident_key_value_prop(&IDENT_LINE, create_num_lit_expr(value.line)),
            ],
        }),
    )
}

pub fn create_coverage_data_object(coverage_data: &FileCoverage) -> (String, Expr) {
    // Afaik there's no built-in way to iterate over struct properties via keys.

    let mut props = vec![];

    // assign coverage['all']
    if coverage_data.all {
        props.push(create_ident_key_value_prop(
            &IDENT_ALL,
            Expr::Lit(Lit::Bool(true.into())),
        ));
    }

    // assign coverage['path']
    props.push(create_ident_key_value_prop(
        &IDENT_PATH,
        create_str_lit_expr(&coverage_data.path),
    ));

    // assign coverage['statementMap']
    let statement_map_prop_values = coverage_data
        .statement_map
        .iter()
        .map(|(key, value)| {
            create_str_key_value_prop(&key.to_string(), create_range_object_lit(value))
        })
        .collect();

    let statement_map_prop = create_ident_key_value_prop(
        &IDENT_STATEMENT_MAP,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: statement_map_prop_values,
        }),
    );
    props.push(statement_map_prop);

    // assign coverage['fnMap']
    let fn_map_prop_values = coverage_data
        .fn_map
        .iter()
        .map(|(key, value)| create_fn_prop(&key.to_string(), value))
        .collect();
    let fn_map_prop = create_ident_key_value_prop(
        &IDENT_FN_MAP,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: fn_map_prop_values,
        }),
    );
    props.push(fn_map_prop);

    let branch_map_prop_values = vec![];
    let branch_map_prop = create_ident_key_value_prop(
        &IDENT_BRANCH_MAP,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: branch_map_prop_values,
        }),
    );

    props.push(branch_map_prop);

    let s_prop_values = coverage_data
        .s
        .iter()
        .map(|(key, value)| {
            create_str_key_value_prop(&key.to_string(), create_num_lit_expr(*value))
        })
        .collect();

    let s_prop = create_ident_key_value_prop(
        &IDENT_S,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: s_prop_values,
        }),
    );
    props.push(s_prop);

    let f_prop_values = coverage_data
        .f
        .iter()
        .map(|(key, value)| {
            create_str_key_value_prop(&key.to_string(), create_num_lit_expr(*value))
        })
        .collect();
    let f_prop = create_ident_key_value_prop(
        &IDENT_F,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: f_prop_values,
        }),
    );
    props.push(f_prop);

    let b_prop_values = vec![];
    let b_prop = create_ident_key_value_prop(
        &IDENT_B,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: b_prop_values,
        }),
    );
    props.push(b_prop);

    if let Some(b_t) = &coverage_data.b_t {
        let b_t_prop_values = vec![];
        let b_t_prop = create_ident_key_value_prop(
            &IDENT_BT,
            Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: b_t_prop_values,
            }),
        );
        props.push(b_t_prop);
    }

    // assign coverage['_coverageSchema']
    let coverage_schema_prop = create_ident_key_value_prop(
        &IDENT_COVERAGE_MAGIC_KEY,
        create_str_lit_expr(&COVERAGE_MAGIC_VALUE),
    );
    props.push(coverage_schema_prop);

    // Original code creates hash against raw coverage object. In here uses str-serialized object instead.
    let coverage_str =
        serde_json::to_string(coverage_data).expect("Should able to serialize coverage data");
    let mut hasher = DefaultHasher::new();
    coverage_str.hash(&mut hasher);
    let hash = hasher.finish().to_string();

    // assign coverage['hash']
    props.push(create_ident_key_value_prop(
        &IDENT_HASH,
        create_str_lit_expr(&hash),
    ));

    (
        hash,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props,
        }),
    )
}

#[cfg(test)]
mod tests {
    use istanbul_oxi_instrument::{BranchType, FileCoverage, Range, SourceCoverage};
    use swc_ecma_quote::quote;
    use swc_plugin::ast::*;

    use crate::template::create_coverage_data_object::create_coverage_data_object;

    use pretty_assertions::assert_eq;

    #[test]
    fn should_create_empty() {
        let coverage_data = FileCoverage::empty("anon".to_string(), false);
        let (_hash, coverage_data_expr) = create_coverage_data_object(&coverage_data);

        let expected = quote!(
            r#"
        {
            path: "anon",
            statementMap: {},
            fnMap: {},
            branchMap: {},
            s: {},
            f: {},
            b: {},
            _coverageSchema: "7101652470475984838",
            hash: "2749072808032864045"
        }
        "# as Expr
        );

        assert_eq!(expected, coverage_data_expr);
    }

    #[test]
    fn should_create_empty_all() {
        let mut coverage_data = FileCoverage::empty("anon".to_string(), false);
        coverage_data.all = true;
        let (_hash, coverage_data_expr) = create_coverage_data_object(&coverage_data);

        let expected = quote!(
            r#"
        {
            all: true,
            path: "anon",
            statementMap: {},
            fnMap: {},
            branchMap: {},
            s: {},
            f: {},
            b: {},
            _coverageSchema: "7101652470475984838",
            hash: "9996448737459597674"
        }
        "# as Expr
        );

        assert_eq!(expected, coverage_data_expr);
    }

    #[test]
    fn should_create_empty_report_logic() {
        let coverage_data = FileCoverage::empty("/test/src/file.js".to_string(), true);
        let (_hash, coverage_data_expr) = create_coverage_data_object(&coverage_data);

        let expected = quote!(
            r#"
        {
            path: "/test/src/file.js",
            statementMap: {},
            fnMap: {},
            branchMap: {},
            s: {},
            f: {},
            b: {},
            bT: {},
            _coverageSchema: "7101652470475984838",
            hash: "5324777076056671972"
        }
        "# as Expr
        );

        assert_eq!(expected, coverage_data_expr);
    }

    #[test]
    fn should_create_statement_map() {
        let mut coverage_data = SourceCoverage::new("/test/src/statement.js".to_string(), false);

        let dummy_range = Range::new(2, 3, 5, 2);
        coverage_data.new_statement(&dummy_range);

        let (_hash, coverage_data_expr) = create_coverage_data_object(coverage_data.as_ref());

        let expected = quote!(
            r#"
        {
            path: "/test/src/statement.js",
            statementMap: {
                "0": {
                    start: {
                        line: 2,
                        column: 3
                    },
                    end: {
                        line: 5,
                        column: 2
                    }
                }
            },
            fnMap: {},
            branchMap: {},
            s: {
                "0": 0
            },
            f: {},
            b: {},
            _coverageSchema: "7101652470475984838",
            hash: "14358638674647738158"
        }
        "# as Expr
        );

        assert_eq!(expected, coverage_data_expr);

        let dummy_range = Range::new(4, 9, 3, 6);
        coverage_data.new_statement(&dummy_range);

        let expected = quote!(
            r#"
        {
            path: "/test/src/statement.js",
            statementMap: {
                "0": {
                    start: {
                        line: 2,
                        column: 3
                    },
                    end: {
                        line: 5,
                        column: 2
                    }
                },
                "1": {
                    start: {
                        line: 4,
                        column: 9
                    },
                    end: {
                        line: 3,
                        column: 6
                    }
                }
            },
            fnMap: {},
            branchMap: {},
            s: {
                "0": 0,
                "1": 0,
            },
            f: {},
            b: {},
            _coverageSchema: "7101652470475984838",
            hash: "8495704048820686839"
        }
        "# as Expr
        );

        let (_hash, coverage_data_expr) = create_coverage_data_object(coverage_data.as_ref());
        assert_eq!(expected, coverage_data_expr);
    }

    #[test]
    fn should_create_fn_map() {
        let mut coverage_data = SourceCoverage::new("/test/src/fn.js".to_string(), false);

        let dummy_range = Range::new(2, 3, 5, 2);
        let decl_range = Range::new(2, 3, 2, 10);
        coverage_data.new_function(
            &Some("named_function".to_string()),
            &decl_range,
            &dummy_range,
        );

        let (_hash, coverage_data_expr) = create_coverage_data_object(coverage_data.as_ref());

        let expected = quote!(
            r#"
        {
            path: "/test/src/fn.js",
            statementMap: {},
            fnMap: {
                "0": {
                    name: "named_function",
                    decl: {
                        start: {
                            line: 2,
                            column: 3
                        },
                        end: {
                            line: 2,
                            column: 10
                        }
                    },
                    loc: {
                        start: {
                            line: 2,
                            column: 3
                        },
                        end: {
                            line: 5,
                            column: 2
                        }
                    },
                    line: 2
                }
            },
            branchMap: {},
            s: {},
            f: {
                "0": 0,
            },
            b: {},
            _coverageSchema: "7101652470475984838",
            hash: "12684468276621003816"
        }
        "# as Expr
        );

        assert_eq!(expected, coverage_data_expr);

        let dummy_range = Range::new(4, 9, 3, 6);
        let decl_range = Range::new(4, 9, 4, 25);
        coverage_data.new_function(&None, &decl_range, &dummy_range);

        let expected = quote!(
            r#"
        {
            path: "/test/src/fn.js",
            statementMap: {},
            fnMap: {
                "0": {
                    name: "named_function",
                    decl: {
                        start: {
                            line: 2,
                            column: 3
                        },
                        end: {
                            line: 2,
                            column: 10
                        }
                    },
                    loc: {
                        start: {
                            line: 2,
                            column: 3
                        },
                        end: {
                            line: 5,
                            column: 2
                        }
                    },
                    line: 2
                },
                "1": {
                    name: "(anonymous_1)",
                    decl: {
                        start: {
                            line: 4,
                            column: 9
                        },
                        end: {
                            line: 4,
                            column: 25
                        }
                    },
                    loc: {
                        start: {
                            line: 4,
                            column: 9
                        },
                        end: {
                            line: 3,
                            column: 6
                        }
                    },
                    line: 4
                }
            },
            branchMap: {},
            s: {},
            f: {
                "0": 0,
                "1": 0
            },
            b: {},
            _coverageSchema: "7101652470475984838",
            hash: "8413193639409683826"
        }
        "# as Expr
        );

        let (_hash, coverage_data_expr) = create_coverage_data_object(coverage_data.as_ref());
        assert_eq!(expected, coverage_data_expr);
    }
}
