use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{coverage::Coverage, Range};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Function {
    pub(crate) name: String,
    pub(crate) decl: Range,
    pub(crate) loc: Range,
    pub(crate) line: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Branch {
    pub(crate) loc: Option<Range>,
    pub(crate) branch_type: String,
    pub(crate) locations: Vec<Range>,
    pub(crate) line: Option<u32>,
}

impl Branch {
    pub fn from_line(branch_type: String, line: u32, locations: Vec<Range>) -> Branch {
        Branch {
            loc: None,
            branch_type,
            locations,
            line: Some(line),
        }
    }
    pub fn from_loc(branch_type: String, loc: Range, locations: Vec<Range>) -> Branch {
        Branch {
            loc: Some(loc),
            branch_type,
            locations,
            line: None,
        }
    }
}

/// Map to line number to hit count.
pub type LineHitMap = IndexMap<u32, u32>;
pub type StatementMap = IndexMap<u32, Range>;
pub type FunctionMap = IndexMap<u32, Function>;
pub type BranchMap = IndexMap<u32, Branch>;
pub type BranchHitMap = IndexMap<u32, Vec<u32>>;
pub type BranchCoverageMap = IndexMap<u32, Coverage>;
