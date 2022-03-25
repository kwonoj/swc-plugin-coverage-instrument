use std::collections::HashMap;

use crate::{coverage::Coverage, Range};

#[derive(Clone)]
pub struct FunctionMapping {
    pub(crate) name: String,
    pub(crate) decl: Range,
    pub(crate) loc: Range,
    pub(crate) line: u32,
}

#[derive(Clone)]
pub struct BranchMapping {
    pub(crate) loc: Range,
    pub(crate) branch_type: String,
    pub(crate) locations: Vec<Range>,
    pub(crate) line: u32,
}

/// Map to line number to hit count.
pub type LineHitMap = HashMap<u32, u32>;
pub type StatementMap = HashMap<u32, Range>;
pub type FunctionMap = HashMap<u32, FunctionMapping>;
pub type BranchMap = HashMap<u32, BranchMapping>;
pub type BranchHitMap = HashMap<u32, Vec<u32>>;
pub type BranchCoverageMap = HashMap<u32, Coverage>;
