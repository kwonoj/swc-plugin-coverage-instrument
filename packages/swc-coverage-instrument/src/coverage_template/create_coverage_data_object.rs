use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use istanbul_oxide::{Branch, FileCoverage, Range};
use swc_core::{
    common::{util::take::Take, DUMMY_SP},
    ecma::ast::*,
};

use crate::constants::idents::*;
use crate::COVERAGE_MAGIC_VALUE;

//TODO: macro, or remove create_* util
fn create_str(value: &str) -> Str {
    Str {
        value: value.into(),
        raw: Some(format!(r#""{}""#, value).into()),
        span: DUMMY_SP,
    }
}

pub fn create_str_lit_expr(value: &str) -> Expr {
    Expr::Lit(Lit::Str(create_str(value)))
}

pub fn create_num_lit_expr(value: u32) -> Expr {
    Expr::Lit(Lit::Num(Number {
        value: value.into(),
        raw: Some(value.to_string().into()),
        span: DUMMY_SP,
    }))
}

pub fn create_ident_key_value_prop(key: &Ident, value: Expr) -> PropOrSpread {
    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(key.clone().into()),
        value: Box::new(value),
    })))
}

pub fn create_str_key_value_prop(key: &str, value: Expr) -> PropOrSpread {
    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Str(create_str(&key)),
        value: Box::new(value),
    })))
}

fn create_range_object_prop(value: &Range) -> Vec<PropOrSpread> {
    vec![
        create_ident_key_value_prop(
            &IDENT_START,
            Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: vec![
                    create_ident_key_value_prop(&IDENT_LINE, create_num_lit_expr(value.start.line)),
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
                    create_ident_key_value_prop(&IDENT_LINE, create_num_lit_expr(value.end.line)),
                    create_ident_key_value_prop(
                        &IDENT_COLUMN,
                        create_num_lit_expr(value.end.column),
                    ),
                ],
            }),
        ),
    ]
}

fn create_range_object_lit(value: &Range) -> Expr {
    Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: create_range_object_prop(value),
    })
}

fn create_fn_prop(key: &str, value: &istanbul_oxide::types::Function) -> PropOrSpread {
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

fn create_branch_vec_prop(value: &Vec<u32>) -> Expr {
    Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems: value
            .iter()
            .map(|v| {
                Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(create_num_lit_expr(*v)),
                })
            })
            .collect(),
    })
}

fn create_branch_prop(key: &str, value: &Branch) -> PropOrSpread {
    let mut props = vec![];

    if let Some(loc) = value.loc {
        props.push(create_ident_key_value_prop(
            &IDENT_LOC,
            create_range_object_lit(&loc),
        ));
    }

    props.push(create_ident_key_value_prop(
        &IDENT_TYPE,
        create_str_lit_expr(&value.branch_type.to_string()),
    ));

    if value.locations.len() > 0 {
        props.push(create_ident_key_value_prop(
            &IDENT_LOCATIONS,
            Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: value
                    .locations
                    .iter()
                    .map(|value| {
                        Some(ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Object(ObjectLit {
                                span: DUMMY_SP,
                                props: create_range_object_prop(value),
                            })),
                        })
                    })
                    .collect(),
            }),
        ))
    }

    if let Some(line) = value.line {
        props.push(create_ident_key_value_prop(
            &IDENT_LINE,
            create_num_lit_expr(line),
        ));
    }

    create_str_key_value_prop(
        key,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props,
        }),
    )
}

pub fn create_coverage_data_object(coverage_data: &FileCoverage) -> (String, Expr) {
    // Afaik there's no built-in way to iterate over struct properties via keys.
    let mut props = vec![];

    // assign coverage['all']
    props.push(create_ident_key_value_prop(
        &IDENT_ALL,
        Expr::Lit(Lit::Bool(coverage_data.all.into())),
    ));

    // assign coverage['path']
    props.push(create_ident_key_value_prop(
        &IDENT_PATH,
        Expr::Lit(Lit::Str(Str {
            value: coverage_data.path.clone().into(),
            ..Str::dummy()
        })),
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

    // assign coverage['branchMap']
    let branch_map_prop_values = coverage_data
        .branch_map
        .iter()
        .map(|(key, value)| create_branch_prop(&key.to_string(), value))
        .collect();
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

    let b_prop_values = coverage_data
        .b
        .iter()
        .map(|(key, value)| {
            create_str_key_value_prop(&key.to_string(), create_branch_vec_prop(value))
        })
        .collect();
    let b_prop = create_ident_key_value_prop(
        &IDENT_B,
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: b_prop_values,
        }),
    );
    props.push(b_prop);

    if let Some(b_t) = &coverage_data.b_t {
        let b_t_prop_values = b_t
            .iter()
            .map(|(key, value)| {
                create_str_key_value_prop(&key.to_string(), create_branch_vec_prop(value))
            })
            .collect();
        let b_t_prop = create_ident_key_value_prop(
            &IDENT_BT,
            Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: b_t_prop_values,
            }),
        );
        props.push(b_t_prop);
    }

    // assign coverage['inputSourceMap']
    if let Some(input_source_map) = &coverage_data.input_source_map {
        let mut source_map_props = vec![];

        source_map_props.push(create_ident_key_value_prop(
            &IDENT_VERSION,
            Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                value: input_source_map.version as f64,
                raw: None,
            })),
        ));

        if let Some(file) = &input_source_map.file {
            source_map_props.push(create_ident_key_value_prop(
                &IDENT_FILE,
                Expr::Lit(Lit::Str(Str {
                    value: file.clone().into(),
                    ..Str::dummy()
                })),
            ));
        }

        if let Some(source_root) = &input_source_map.source_root {
            source_map_props.push(create_ident_key_value_prop(
                &IDENT_SOURCE_ROOT,
                Expr::Lit(Lit::Str(Str {
                    value: source_root.clone().into(),
                    ..Str::dummy()
                })),
            ));
        }

        source_map_props.push(create_ident_key_value_prop(
            &IDENT_SOURCES,
            Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: input_source_map
                    .sources
                    .iter()
                    .map(|v| {
                        Some(ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Lit(Lit::Str(Str {
                                value: v.clone().into(),
                                ..Str::dummy()
                            }))),
                        })
                    })
                    .collect(),
            }),
        ));

        if let Some(sources_content) = &input_source_map.sources_content {
            source_map_props.push(create_ident_key_value_prop(
                &IDENT_SOURCES_CONTENT,
                Expr::Array(ArrayLit {
                    span: DUMMY_SP,
                    elems: sources_content
                        .iter()
                        .map(|v| {
                            Some(ExprOrSpread {
                                spread: None,
                                expr: Box::new(Expr::Lit(if let Some(v) = v {
                                    Lit::Str(Str::from(v.as_ref()))
                                } else {
                                    Lit::Null(Null::dummy())
                                })),
                            })
                        })
                        .collect(),
                }),
            ));
        }

        source_map_props.push(create_ident_key_value_prop(
            &IDENT_NAMES,
            Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: input_source_map
                    .names
                    .iter()
                    .map(|v| {
                        Some(ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Lit(Lit::Str(Str {
                                value: v.clone().into(),
                                ..Str::dummy()
                            }))),
                        })
                    })
                    .collect(),
            }),
        ));

        source_map_props.push(create_ident_key_value_prop(
            &IDENT_MAPPINGS,
            Expr::Lit(Lit::Str(Str {
                value: input_source_map.mappings.clone().into(),
                ..Str::dummy()
            })),
        ));

        let input_source_map_prop = create_ident_key_value_prop(
            &IDENT_INPUT_SOURCE_MAP,
            Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: source_map_props,
            }),
        );

        props.push(input_source_map_prop);
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
    use istanbul_oxide::BranchType;
    use swc_core::quote;

    use crate::source_coverage::SourceCoverage;

    use pretty_assertions::assert_eq;

    use super::*;

    fn adjust_expected_ast_path_raw(e: &mut Expr, idx: usize, value: &str) {
        if let Expr::Object(lit) = e {
            let _ = std::mem::replace(
                &mut lit.props[idx],
                create_ident_key_value_prop(
                    &IDENT_PATH,
                    Expr::Lit(Lit::Str(Str {
                        value: value.into(),
                        ..Str::dummy()
                    })),
                ),
            );
        }
    }

    #[test]
    fn should_create_empty() {
        let file_path = "anon";
        let coverage_data = FileCoverage::empty(file_path.to_string(), false);
        let (_hash, coverage_data_expr) = create_coverage_data_object(&coverage_data);

        let mut expected = quote!(
            r#"
        {
            all: false,
            path: "anon",
            statementMap: {},
            fnMap: {},
            branchMap: {},
            s: {},
            f: {},
            b: {},
            _coverageSchema: "11020577277169172593",
            hash: "2749072808032864045"
        }
        "# as Expr
        );
        adjust_expected_ast_path_raw(&mut expected, 1, file_path);

        assert_eq!(expected, coverage_data_expr);
    }

    #[test]
    fn should_create_empty_all() {
        let file_path = "anon";
        let mut coverage_data = FileCoverage::empty(file_path.to_string(), false);
        coverage_data.all = true;
        let (_hash, coverage_data_expr) = create_coverage_data_object(&coverage_data);

        let mut expected = quote!(
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
            _coverageSchema: "11020577277169172593",
            hash: "9996448737459597674"
        }
        "# as Expr
        );
        adjust_expected_ast_path_raw(&mut expected, 1, file_path);

        assert_eq!(expected, coverage_data_expr);
    }

    #[test]
    fn should_create_empty_report_logic() {
        let file_path = "/test/src/file.js";
        let coverage_data = FileCoverage::empty(file_path.to_string(), true);
        let (_hash, coverage_data_expr) = create_coverage_data_object(&coverage_data);

        let mut expected = quote!(
            r#"
        {
            all: false,
            path: "/test/src/file.js",
            statementMap: {},
            fnMap: {},
            branchMap: {},
            s: {},
            f: {},
            b: {},
            bT: {},
            _coverageSchema: "11020577277169172593",
            hash: "5324777076056671972"
        }
        "# as Expr
        );
        adjust_expected_ast_path_raw(&mut expected, 1, file_path);

        assert_eq!(expected, coverage_data_expr);
    }

    #[test]
    fn should_create_statement_map() {
        let file_path = "/test/src/statement.js";
        let mut coverage_data = SourceCoverage::new(file_path.to_string(), false);

        let dummy_range = Range::new(2, 3, 5, 2);
        coverage_data.new_statement(&dummy_range);

        let (_hash, coverage_data_expr) = create_coverage_data_object(coverage_data.as_ref());

        let mut expected = quote!(
            r#"
        {
            all: false,
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
            _coverageSchema: "11020577277169172593",
            hash: "14358638674647738158"
        }
        "# as Expr
        );
        adjust_expected_ast_path_raw(&mut expected, 1, file_path);

        assert_eq!(expected, coverage_data_expr);

        let dummy_range = Range::new(4, 9, 3, 6);
        coverage_data.new_statement(&dummy_range);

        let mut expected = quote!(
            r#"
        {
            all: false,
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
            _coverageSchema: "11020577277169172593",
            hash: "8495704048820686839"
        }
        "# as Expr
        );
        adjust_expected_ast_path_raw(&mut expected, 1, file_path);

        let (_hash, coverage_data_expr) = create_coverage_data_object(coverage_data.as_ref());
        assert_eq!(expected, coverage_data_expr);
    }

    #[test]
    fn should_create_fn_map() {
        let file_path = "/test/src/fn.js";
        let mut coverage_data = SourceCoverage::new(file_path.to_string(), false);

        let dummy_range = Range::new(2, 3, 5, 2);
        let decl_range = Range::new(2, 3, 2, 10);
        coverage_data.new_function(
            &Some("named_function".to_string()),
            &decl_range,
            &dummy_range,
        );

        let (_hash, coverage_data_expr) = create_coverage_data_object(coverage_data.as_ref());

        let mut expected = quote!(
            r#"
        {
            all: false,
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
            _coverageSchema: "11020577277169172593",
            hash: "12684468276621003816"
        }
        "# as Expr
        );
        adjust_expected_ast_path_raw(&mut expected, 1, file_path);

        assert_eq!(expected, coverage_data_expr);

        let dummy_range = Range::new(4, 9, 3, 6);
        let decl_range = Range::new(4, 9, 4, 25);
        coverage_data.new_function(&None, &decl_range, &dummy_range);

        let mut expected = quote!(
            r#"
        {
            all: false,
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
            _coverageSchema: "11020577277169172593",
            hash: "8413193639409683826"
        }
        "# as Expr
        );
        adjust_expected_ast_path_raw(&mut expected, 1, file_path);

        let (_hash, coverage_data_expr) = create_coverage_data_object(coverage_data.as_ref());
        assert_eq!(expected, coverage_data_expr);
    }

    #[test]
    fn should_create_new_branch() {
        let file_path = "/test/src/branch.js";
        let mut coverage_data = SourceCoverage::new(file_path.to_string(), false);

        let dummy_range = Range::new(2, 3, 5, 2);
        coverage_data.new_branch(BranchType::Switch, &dummy_range, false);

        let (_hash, coverage_data_expr) = create_coverage_data_object(coverage_data.as_ref());

        let mut expected = quote!(
            r#"
        {
          all: false,
          path: "/test/src/branch.js",
          statementMap: {},
          fnMap: {},
          branchMap: {
            "0": {
              loc: { start: { line: 2, column: 3 }, end: { line: 5, column: 2 } },
              type: "switch",
              line: 2
            }
          },
          s: {},
          f: {},
          b: { "0": [] },
          _coverageSchema: "11020577277169172593",
          hash: "16290170317654300968"
        }
        "# as Expr
        );
        adjust_expected_ast_path_raw(&mut expected, 1, file_path);

        assert_eq!(expected, coverage_data_expr);

        let dummy_range = Range::new(6, 4, 2, 8);
        coverage_data.new_branch(BranchType::BinaryExpr, &dummy_range, true);

        let (_hash, coverage_data_expr) = create_coverage_data_object(coverage_data.as_ref());

        let mut expected = quote!(
            r#"
        {
          all: false,
          path: "/test/src/branch.js",
          statementMap: {},
          fnMap: {},
          branchMap: {
            "0": {
              loc: { start: { line: 2, column: 3 }, end: { line: 5, column: 2 } },
              type: "switch",
              line: 2
            },
            "1": {
              loc: { start: { line: 6, column: 4 }, end: { line: 2, column: 8 } },
              type: "binary-expr",
              line: 6
            }
          },
          s: {},
          f: {},
          b: { "0": [], "1": [] },
          bT: { "1": [] },
          _coverageSchema: "11020577277169172593",
          hash: "394046461779423801"
        }
        "# as Expr
        );
        adjust_expected_ast_path_raw(&mut expected, 1, file_path);

        assert_eq!(expected, coverage_data_expr);
    }

    #[test]
    fn should_add_branch_path() {
        let file_path = "/test/src/branch_path.js";
        let mut coverage_data = SourceCoverage::new(file_path.to_string(), false);

        let dummy_range = Range::new(2, 3, 5, 2);
        let location_range = Range::new(3, 4, 5, 4);
        let name = coverage_data.new_branch(BranchType::Switch, &dummy_range, false);
        coverage_data.add_branch_path(name, &location_range);

        let (_hash, coverage_data_expr) = create_coverage_data_object(coverage_data.as_ref());

        let mut expected = quote!(
            r#"
        {
          all: false,
          path: "/test/src/branch_path.js",
          statementMap: {},
          fnMap: {},
          branchMap: {
            "0": {
              loc: { start: { line: 2, column: 3 }, end: { line: 5, column: 2 } },
              type: "switch",
              locations: [{ start: { line: 3, column: 4 }, end: { line: 5, column: 4 } }],
              line: 2
            }
          },
          s: {},
          f: {},
          b: { "0": [0] },
          _coverageSchema: "11020577277169172593",
          hash: "1206056395566328244"
        }
        "# as Expr
        );
        adjust_expected_ast_path_raw(&mut expected, 1, file_path);

        assert_eq!(expected, coverage_data_expr);

        let dummy_range = Range::new(6, 4, 2, 8);
        let name = coverage_data.new_branch(BranchType::BinaryExpr, &dummy_range, true);
        coverage_data.add_branch_path(name, &location_range);

        let (_hash, coverage_data_expr) = create_coverage_data_object(coverage_data.as_ref());

        let mut expected = quote!(
            r#"
        {
          all: false,
          path: "/test/src/branch_path.js",
          statementMap: {},
          fnMap: {},
          branchMap: {
            "0": {
              loc: { start: { line: 2, column: 3 }, end: { line: 5, column: 2 } },
              type: "switch",
              locations: [{ start: { line: 3, column: 4 }, end: { line: 5, column: 4 } }],
              line: 2
            },
            "1": {
              loc: { start: { line: 6, column: 4 }, end: { line: 2, column: 8 } },
              type: "binary-expr",
              locations: [{ start: { line: 3, column: 4 }, end: { line: 5, column: 4 } }],
              line: 6
            }
          },
          s: {},
          f: {},
          b: { "0": [0], "1": [0] },
          bT: { "1": [0] },
          _coverageSchema: "11020577277169172593",
          hash: "5849348874565150566"
        }
        "# as Expr
        );
        adjust_expected_ast_path_raw(&mut expected, 1, file_path);

        assert_eq!(expected, coverage_data_expr);
    }
}
