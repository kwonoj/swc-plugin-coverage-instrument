use indexmap::IndexMap;

use crate::{coverage::Coverage, Range};

#[derive(Clone, Debug)]
pub struct Function {
    pub(crate) name: String,
    pub(crate) decl: Range,
    pub(crate) loc: Range,
    pub(crate) line: u32,
}

#[derive(Clone, Debug)]
pub struct Branch {
    pub(crate) loc: Range,
    pub(crate) branch_type: String,
    pub(crate) locations: Vec<Range>,
    pub(crate) line: u32,
}

/// Map to line number to hit count.
pub type LineHitMap = IndexMap<u32, u32>;
pub type StatementMap = IndexMap<u32, Range>;
pub type FunctionMap = IndexMap<u32, Function>;
pub type BranchMap = IndexMap<u32, Branch>;
pub type BranchHitMap = IndexMap<u32, Vec<u32>>;
pub type BranchCoverageMap = IndexMap<u32, Coverage>;
