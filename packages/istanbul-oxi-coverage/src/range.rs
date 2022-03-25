#[derive(Copy, Clone)]
pub struct Location {
    pub(crate) line: u32,
    pub(crate) column: u32,
}

#[derive(Copy, Clone)]
pub struct Range {
    pub(crate) start: Location,
    pub(crate) end: Location,
}

impl Range {
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
