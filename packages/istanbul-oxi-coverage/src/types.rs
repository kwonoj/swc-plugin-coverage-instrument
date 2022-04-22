use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{coverage::Coverage, Range};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub decl: Range,
    pub loc: Range,
    pub line: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BranchType {
    BinaryExpr,
    DefaultArg,
    If,
    Switch,
    CondExpr,
}

impl ToString for BranchType {
    fn to_string(&self) -> String {
        match self {
            BranchType::BinaryExpr => "binary-expr".to_string(),
            BranchType::DefaultArg => "default-arg".to_string(),
            BranchType::If => "if".to_string(),
            BranchType::Switch => "switch".to_string(),
            BranchType::CondExpr => "cond-expr".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Branch {
    pub loc: Option<Range>,
    #[serde(rename = "type")]
    pub branch_type: BranchType,
    pub locations: Vec<Range>,
    pub line: Option<u32>,
}

impl Branch {
    pub fn from_line(branch_type: BranchType, line: u32, locations: Vec<Range>) -> Branch {
        Branch {
            loc: None,
            branch_type,
            locations,
            line: Some(line),
        }
    }
    pub fn from_loc(branch_type: BranchType, loc: Range, locations: Vec<Range>) -> Branch {
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

#[cfg(test)]
mod tests {
    use crate::BranchType;

    #[test]
    fn branch_type_should_return_kebab_string() {
        assert_eq!(&BranchType::BinaryExpr.to_string(), "binary-expr");
        assert_eq!(&BranchType::DefaultArg.to_string(), "default-arg");
        assert_eq!(&BranchType::If.to_string(), "if");
        assert_eq!(&BranchType::Switch.to_string(), "switch");
        assert_eq!(&BranchType::CondExpr.to_string(), "cond-expr");
    }
}
