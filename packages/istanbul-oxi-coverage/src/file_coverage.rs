use std::collections::HashMap;

#[derive(Copy, Clone)]
pub struct UnknownReserved;

#[derive(Copy, Clone)]
pub struct Location {
    line: u32,
    column: u32,
}

#[derive(Copy, Clone)]
pub struct Range {
    start: Location,
    end: Location,
}

#[derive(Clone)]
pub struct FunctionMapping {
    name: String,
    decl: Range,
    loc: Range,
    line: u32,
}

#[derive(Clone)]
pub struct BranchMapping {
    loc: Range,
    branch_type: String,
    locations: Vec<Range>,
    line: u32,
}

/// provides a read-only view of coverage for a single file.
/// The deep structure of this object is documented elsewhere. It has the following
/// properties:
/// `path` - the file path for which coverage is being tracked
/// `statementMap` - map of statement locations keyed by statement index
/// `fnMap` - map of function metadata keyed by function index
/// `branchMap` - map of branch metadata keyed by branch index
/// `s` - hit counts for statements
/// `f` - hit count for functions
/// `b` - hit count for branches

#[derive(Clone)]
pub struct FileCoverage {
    path: String,
    statement_map: HashMap<String, Range>,
    fn_map: HashMap<String, FunctionMapping>,
    branch_map: HashMap<String, BranchMapping>,
    s: HashMap<String, u32>,
    f: HashMap<String, u32>,
    b: HashMap<String, Vec<u32>>,
    b_t: Option<HashMap<String, UnknownReserved>>,
}

impl FileCoverage {
    pub fn empty(file_path: String, report_logic: bool) -> FileCoverage {
        FileCoverage {
            path: file_path,
            statement_map: Default::default(),
            fn_map: Default::default(),
            branch_map: Default::default(),
            s: Default::default(),
            f: Default::default(),
            b: Default::default(),
            b_t: if report_logic {
                Some(Default::default())
            } else {
                None
            },
        }
    }

    pub fn from_file_path(file_path: String, report_logic: bool) -> FileCoverage {
        FileCoverage::empty(file_path, report_logic)
    }

    pub fn from_file_coverage(coverage: &FileCoverage) -> FileCoverage {
        coverage.clone()
    }

    /// Returns computed line coverage from statement coverage.
    /// This is a map of hits keyed by line number in the source.
    pub fn get_line_coverage() {
        unimplemented!()
    }

    /// Returns an array of uncovered line numbers.
    pub fn get_uncovered_lines() {
        unimplemented!()
    }

    pub fn get_branch_coverage_by_line() {
        unimplemented!()
    }

    pub fn to_json() {
        unimplemented!()
    }

    pub fn merge() {
        unimplemented!()
    }

    pub fn compute_simple_totals() {
        unimplemented!()
    }

    pub fn compute_branch_totals() {
        unimplemented!()
    }

    pub fn reset_hits() {
        unimplemented!()
    }

    pub fn to_summary() {
        unimplemented!()
    }
}
