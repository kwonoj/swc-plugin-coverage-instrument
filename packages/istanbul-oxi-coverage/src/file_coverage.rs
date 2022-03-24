use std::collections::HashMap;

use crate::{percent, CoveragePercentage, CoverageSummary, Totals};

type LineMap = HashMap<u32, u32>;

#[derive(Copy, Clone)]
pub struct Coverage {
    covered: u32,
    total: u32,
    coverage: f32,
}

impl Coverage {
    pub fn new(covered: u32, total: u32, coverage: f32) -> Coverage {
        Coverage {
            covered,
            total,
            coverage,
        }
    }
}

type BranchMap = HashMap<u32, Coverage>;

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

impl Range {
    pub fn key_from_loc(range: &Range) -> String {
        format!(
            "{}|{}|{}|{}",
            range.start.line, range.start.column, range.end.line, range.end.column
        )
    }
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
    b_t: Option<HashMap<String, Vec<u32>>>,
}

fn merge_properties() {
    unimplemented!()
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
    pub fn get_line_coverage(&self) -> LineMap {
        let statements_map = &self.statement_map;
        let statements = &self.s;

        let mut line_map: LineMap = Default::default();

        for (st, count) in statements {
            let line = statements_map
                .get(st)
                .expect("statement not found")
                .start
                .line;
            let pre_val = line_map.get(&line);

            match pre_val {
                Some(pre_val) if pre_val < count => {
                    line_map.insert(line, *count);
                }
                None => {
                    line_map.insert(line, *count);
                }
                _ => {
                    //noop
                }
            }
        }

        line_map
    }

    /// Returns an array of uncovered line numbers.
    pub fn get_uncovered_lines(&self) -> Vec<u32> {
        let lc = self.get_line_coverage();
        let mut ret: Vec<u32> = Default::default();

        for (l, hits) in lc {
            if hits == 0 {
                ret.push(l);
            }
        }

        ret
    }

    pub fn get_branch_coverage_by_line(&self) -> BranchMap {
        let branch_map = &self.branch_map;
        let branches = &self.b;

        let mut prefilter_data: HashMap<u32, Vec<u32>> = Default::default();
        let mut ret: BranchMap = Default::default();

        for (k, map) in branch_map {
            let line = if map.line > 0 {
                map.line
            } else {
                map.loc.start.line
            };
            let branch_data = branches.get(k).expect("branch data not found");

            if let Some(line_data) = prefilter_data.get_mut(&line) {
                line_data.append(&mut branch_data.clone());
            } else {
                prefilter_data.insert(line, branch_data.clone());
            }
        }

        for (k, data_array) in prefilter_data {
            let covered: Vec<&u32> = data_array.iter().filter(|&x| *x > 0).collect();
            let coverage = covered.len() as f32 / data_array.len() as f32 * 100 as f32;

            ret.insert(
                k,
                Coverage::new(covered.len() as u32, data_array.len() as u32, coverage),
            );
        }

        ret
    }

    pub fn to_json() {
        unimplemented!()
    }

    pub fn merge() {
        unimplemented!()
    }

    pub fn compute_simple_totals<T>(line_map: &HashMap<T, u32>) -> Totals {
        let mut ret: Totals = Totals {
            total: line_map.len() as u32,
            covered: line_map.values().filter(|&x| *x > 0).count() as u32,
            skipped: 0,
            pct: CoveragePercentage::Unknown,
        };

        ret.pct = CoveragePercentage::Value(percent(ret.covered, ret.total));
        ret
    }

    fn compute_branch_totals(branch_map: &HashMap<String, Vec<u32>>) -> Totals {
        let mut ret: Totals = Default::default();

        branch_map.values().for_each(|branches| {
            ret.covered += branches.iter().filter(|hits| **hits > 0).count() as u32;
            ret.total += branches.len() as u32;
        });

        ret.pct = CoveragePercentage::Value(percent(ret.covered, ret.total));
        ret
    }

    pub fn reset_hits(&mut self) {
        for val in self.s.values_mut() {
            *val = 0;
        }

        for val in self.f.values_mut() {
            *val = 0;
        }

        for val in self.b.values_mut() {
            val.iter_mut().for_each(|x| *x = 0);
        }

        if let Some(branches_true) = &mut self.b_t {
            for val in branches_true.values_mut() {
                val.iter_mut().for_each(|x| *x = 0);
            }
        }
    }

    pub fn to_summary() -> CoverageSummary {
        unimplemented!()
    }
}
