use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub line: u32,
    pub column: u32,
}
impl Location {
    pub fn default() -> Location {
        Location { line: 0, column: 0 }
    }
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Range {
    pub start: Location,
    pub end: Location,
}

impl Range {
    pub fn default() -> Range {
        Range {
            start: Default::default(),
            end: Default::default(),
        }
    }
    pub fn new(start_line: u32, start_column: u32, end_line: u32, end_column: u32) -> Range {
        Range {
            start: Location {
                line: start_line,
                column: start_column,
            },
            end: Location {
                line: end_line,
                column: end_column,
            },
        }
    }
}
