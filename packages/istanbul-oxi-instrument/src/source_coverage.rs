use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use istanbul_oxi_coverage::{
    BranchHitMap, BranchMap, FileCoverage, FunctionMap, LineHitMap, Range, StatementMap,
};
use once_cell::sync::Lazy;

/// SourceCoverage provides mutation methods to manipulate the structure of
/// a file coverage object. Used by the instrumenter to create a full coverage
/// object for a file incrementally.
pub struct SourceCoverage {
    inner: FileCoverage,
}

static COVERAGE_MAGIC_VALUE: Lazy<String> = Lazy::new(|| {
    let mut s = DefaultHasher::new();
    let name = "istanbul-oxi-instrument";
    format!("{}@{}", name, 4).hash(&mut s);
    return format!("cov_{}", s.finish());
});

#[allow(non_snake_case)]
pub struct SealedFileCoverage {
    pub _coverageSchema: String,
    pub hash: String,

    pub all: bool,
    pub path: String,
    pub statement_map: StatementMap,
    pub fn_map: FunctionMap,
    pub branch_map: BranchMap,
    pub s: LineHitMap,
    pub f: LineHitMap,
    pub b: BranchHitMap,
    pub b_t: Option<BranchHitMap>,
}

impl SealedFileCoverage {
    fn from(value: &FileCoverage) -> SealedFileCoverage {
        let mut ret = SealedFileCoverage {
            _coverageSchema: COVERAGE_MAGIC_VALUE.clone(),
            hash: Default::default(),
            all: value.all,
            path: value.path.clone(),
            statement_map: value.statement_map.clone(),
            fn_map: value.fn_map.clone(),
            branch_map: value.branch_map.clone(),
            s: value.s.clone(),
            f: value.f.clone(),
            b: value.b.clone(),
            b_t: value.b_t.clone(),
        };

        // TODO: proper hash
        let mut s = DefaultHasher::new();
        "todo".hash(&mut s);
        /*
        const hash = createHash(SHA)
            .update(JSON.stringify(coverageData))
            .digest('hex');
        */
        ret.hash = format!("{}", s.finish());

        ret
    }
}

pub struct UnknownReserved;

impl SourceCoverage {
    pub fn new_statement(&mut self, loc: Range) {}
    pub fn new_function(&mut self, decl: Range, loc: Range) {}
    pub fn new_branch(&mut self, loc: Range, is_report_logic: bool) {}
    pub fn maybe_new_branch_true(&mut self, name: u32, is_report_logic: bool) {}
    pub fn add_branch_path(&mut self, name: u32, location: Range) {}
    pub fn maybe_add_branch_true(&mut self, name: u32) {}
    pub fn set_input_source_map(&mut self, source_map: UnknownReserved) {}
    pub fn freeze(&mut self) {}

    /// Returns inner file coverage contains necessary hashes to be attached into ast
    /// Original code does it with to_json() with duck-typing to add necessary properties.
    pub fn get_sealed_inner(&self) -> SealedFileCoverage {
        SealedFileCoverage::from(&self.inner)
    }

    pub fn from_file_path(file_path: String, report_logic: bool) -> SourceCoverage {
        SourceCoverage {
            inner: FileCoverage::from_file_path(file_path, report_logic),
        }
    }
}
