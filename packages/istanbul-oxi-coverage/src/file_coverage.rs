use std::collections::HashMap;

use crate::{
    coverage::Coverage,
    percent,
    types::{
        BranchCoverageMap, BranchHitMap, BranchMap, BranchMapping, FunctionMap, FunctionMapping,
    },
    CoveragePercentage, CoverageSummary, LineHitMap, Range, StatementMap, Totals,
};

fn key_from_loc(range: &Range) -> String {
    format!(
        "{}|{}|{}|{}",
        range.start.line, range.start.column, range.end.line, range.end.column
    )
}

fn merge_properties_hits_vec(
    first_hits: &BranchHitMap,
    first_map: &BranchMap,
    second_hits: &BranchHitMap,
    second_map: &BranchMap,
    get_item_key_fn: for<'r> fn(&'r BranchMapping) -> String,
) -> (BranchHitMap, HashMap<u32, BranchMapping>) {
    let mut items: HashMap<String, (Vec<u32>, BranchMapping)> = Default::default();

    for (key, item_hits) in first_hits {
        let item = first_map
            .get(key)
            .expect("Corresponding map value should exist");
        let item_key = get_item_key_fn(item);

        items.insert(item_key, (item_hits.clone(), item.clone()));
    }

    for (key, item_hits) in second_hits {
        let item = second_map
            .get(key)
            .expect("Corresponding map value should exist");
        let item_key = get_item_key_fn(item);

        items
            .entry(item_key)
            .and_modify(|_pair| {
                /*
                item_hits.forEach((hits, h) => {
                    if (aPair[0][h] !== undefined) aPair[0][h] += hits;
                    else aPair[0][h] = hits;
                }); */
            })
            .or_insert((item_hits.clone(), item.clone()));
    }

    let mut hits: BranchHitMap = Default::default();
    let mut map: BranchMap = Default::default();

    for (idx, (_, (hit, item))) in items.iter_mut().enumerate() {
        hits.insert(idx as u32, hit.clone());
        map.insert(idx as u32, item.clone());
    }

    (hits, map)
}

fn merge_properties<T>(
    first_hits: &LineHitMap,
    first_map: &HashMap<u32, T>,
    second_hits: &LineHitMap,
    second_map: &HashMap<u32, T>,
    get_item_key_fn: for<'r> fn(&'r T) -> String,
) -> (LineHitMap, HashMap<u32, T>)
where
    T: Clone,
{
    let mut items: HashMap<String, (u32, T)> = Default::default();

    for (key, item_hits) in first_hits {
        let item = first_map
            .get(key)
            .expect("Corresponding map value should exist");
        let item_key = get_item_key_fn(item);

        items.insert(item_key, (*item_hits, item.clone()));
    }

    for (key, item_hits) in second_hits {
        let item = second_map
            .get(key)
            .expect("Corresponding map value should exist");
        let item_key = get_item_key_fn(item);

        items
            .entry(item_key)
            .and_modify(|pair| {
                pair.0 += *item_hits;
            })
            .or_insert((*item_hits, item.clone()));
    }

    let mut hits: LineHitMap = Default::default();
    let mut map: HashMap<u32, T> = Default::default();

    for (idx, (_, (hit, item))) in items.iter_mut().enumerate() {
        hits.insert(idx as u32, *hit);
        map.insert(idx as u32, item.clone());
    }

    (hits, map)
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
    pub(crate) all: bool,
    path: String,
    statement_map: StatementMap,
    fn_map: FunctionMap,
    branch_map: BranchMap,
    s: LineHitMap,
    f: LineHitMap,
    b: BranchHitMap,
    b_t: Option<BranchHitMap>,
}

impl FileCoverage {
    pub fn empty(file_path: String, report_logic: bool) -> FileCoverage {
        FileCoverage {
            all: false,
            path: file_path,
            statement_map: Default::default(),
            fn_map: Default::default(),
            branch_map: Default::default(),
            s: Default::default(),
            b: Default::default(),
            f: Default::default(),
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
    pub fn get_line_coverage(&self) -> LineHitMap {
        let statements_map = &self.statement_map;
        let statements = &self.s;

        let mut line_map: LineHitMap = Default::default();

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

    pub fn get_branch_coverage_by_line(&self) -> BranchCoverageMap {
        let branch_map = &self.branch_map;
        let branches = &self.b;

        let mut prefilter_data: BranchHitMap = Default::default();
        let mut ret: BranchCoverageMap = Default::default();

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
    /// Merges a second coverage object into this one, updating hit counts
    pub fn merge(&mut self, coverage: &FileCoverage) {
        if coverage.all {
            return;
        }

        if self.all {
            *self = coverage.clone();
            return;
        }

        let (statement_hits_merged, statement_map_merged) = merge_properties(
            &self.s,
            &self.statement_map,
            &coverage.s,
            &coverage.statement_map,
            |range: &Range| key_from_loc(range),
        );

        self.s = statement_hits_merged;
        self.statement_map = statement_map_merged;

        let (fn_hits_merged, fn_map_merged) = merge_properties(
            &self.f,
            &self.fn_map,
            &coverage.f,
            &coverage.fn_map,
            |map: &FunctionMapping| key_from_loc(&map.loc),
        );

        self.f = fn_hits_merged;
        self.fn_map = fn_map_merged;

        let (branches_hits_merged, branches_map_merged) = merge_properties_hits_vec(
            &self.b,
            &self.branch_map,
            &coverage.b,
            &coverage.branch_map,
            |branch: &BranchMapping| key_from_loc(&branch.locations[0]),
        );
        self.b = branches_hits_merged;
        self.branch_map = branches_map_merged;

        // Tracking additional information about branch truthiness
        // can be optionally enabled:
        if let Some(branches_true) = &self.b_t {
            if let Some(coverage_branches_true) = &coverage.b_t {
                let (branches_true_hits_merged, _) = merge_properties_hits_vec(
                    branches_true,
                    &self.branch_map,
                    coverage_branches_true,
                    &coverage.branch_map,
                    |branch: &BranchMapping| key_from_loc(&branch.locations[0]),
                );

                self.b_t = Some(branches_true_hits_merged);
            }
        }
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

    fn compute_branch_totals(branch_map: &BranchHitMap) -> Totals {
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

    pub fn to_summary(&self) -> CoverageSummary {
        let line_coverage = self.get_line_coverage();

        let line = FileCoverage::compute_simple_totals(&line_coverage);
        let function = FileCoverage::compute_simple_totals(&self.f);
        let statement = FileCoverage::compute_simple_totals(&self.s);
        let branches = FileCoverage::compute_branch_totals(&self.b);

        let branches_true = if let Some(branches_true) = &self.b_t {
            Some(FileCoverage::compute_branch_totals(&branches_true))
        } else {
            None
        };

        CoverageSummary::new(line, function, statement, branches, branches_true)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{FileCoverage, Range};

    #[test]
    fn should_able_to_merge() {
        let base_statement_map = HashMap::from([
            ("0".to_string(), Range::new(1, 1, 1, 100)),
            ("1".to_string(), Range::new(2, 1, 2, 50)),
            ("2".to_string(), Range::new(2, 51, 2, 100)),
            ("3".to_string(), Range::new(2, 101, 3, 100)),
        ]);

        let mut base = FileCoverage::from_file_path("/path/to/file".to_string(), false);
    }
}
