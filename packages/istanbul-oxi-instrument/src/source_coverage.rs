use istanbul_oxi_coverage::{Branch, BranchType, FileCoverage, Function, Range};

pub struct SourceCoverageMetaHitCount {
    pub s: u32,
    pub f: u32,
    pub b: u32,
}

impl Default for SourceCoverageMetaHitCount {
    fn default() -> Self {
        SourceCoverageMetaHitCount { s: 0, f: 0, b: 0 }
    }
}

pub struct SourceCoverageMeta {
    last: SourceCoverageMetaHitCount,
}

impl Default for SourceCoverageMeta {
    fn default() -> Self {
        SourceCoverageMeta {
            last: Default::default(),
        }
    }
}

/// SourceCoverage provides mutation methods to manipulate the structure of
/// a file coverage object. Used by the instrumenter to create a full coverage
/// object for a file incrementally.
pub struct SourceCoverage {
    inner: FileCoverage,
    meta: SourceCoverageMeta,
}

pub struct UnknownReserved;

impl SourceCoverage {
    pub fn new(file_path: String, report_logic: bool) -> Self {
        SourceCoverage {
            inner: FileCoverage::from_file_path(file_path, report_logic),
            meta: Default::default(),
        }
    }

    pub fn as_ref(&self) -> &FileCoverage {
        &self.inner
    }
}

impl SourceCoverage {
    pub fn new_statement(&mut self, loc: &Range) -> u32 {
        let s = self.meta.last.s;
        self.inner.statement_map.insert(s, loc.clone());
        self.inner.s.insert(s, 0);
        self.meta.last.s += 1;
        s
    }

    pub fn new_function(&mut self, name: &Option<String>, decl: &Range, loc: &Range) -> u32 {
        let f = self.meta.last.f;
        let name = if let Some(name) = name {
            name.clone()
        } else {
            format!("(anonymous_{})", f)
        };

        self.inner.fn_map.insert(
            f,
            Function {
                name,
                decl: decl.clone(),
                loc: loc.clone(),
                // DEPRECATED: some legacy reports require this info.
                line: loc.start.line,
            },
        );

        self.inner.f.insert(f, 0);
        self.meta.last.f += 1;
        f
    }

    pub fn new_branch(
        &mut self,
        branch_type: &BranchType,
        loc: &Range,
        is_report_logic: bool,
    ) -> u32 {
        let b = self.meta.last.b;
        self.inner.b.insert(b, vec![]);
        self.inner.branch_map.insert(
            b,
            Branch {
                loc: Some(loc.clone()),
                branch_type: branch_type.clone(),
                locations: vec![],
                // DEPRECATED: some legacy reports require this info.
                line: Some(loc.start.line),
            },
        );

        self.meta.last.b += 1;
        self.maybe_new_branch_true(branch_type, b, is_report_logic);
        b
    }

    pub fn maybe_new_branch_true(
        &mut self,
        branch_type: &BranchType,
        name: u32,
        is_report_logic: bool,
    ) {
        if !is_report_logic {
            return;
        }

        if let BranchType::BinaryExpr = branch_type {
            if self.inner.b_t.is_none() {
                self.inner.b_t = Some(Default::default());
            }

            self.inner
                .b_t
                .as_mut()
                .expect("b_t should be available")
                .insert(name, vec![]);
        }
    }

    pub fn add_branch_path(&mut self, name: u32, location: &Range) -> u32 {
        let b_meta = self
            .inner
            .branch_map
            .get_mut(&name)
            .expect(&format!("Invalid branch {}", name));
        let counts = self
            .inner
            .b
            .get_mut(&name)
            .expect("Counts should be available");

        b_meta.locations.push(location.clone());
        counts.push(0);

        self.maybe_add_branch_true(name);

        (self
            .inner
            .b
            .get(&name)
            .expect("Counts should be available")
            .len()
            - 1) as u32
    }

    pub fn maybe_add_branch_true(&mut self, name: u32) {
        if let Some(b_t) = &mut self.inner.b_t {
            let counts_true = b_t.get_mut(&name);
            if let Some(counts_true) = counts_true {
                counts_true.push(0);
            }
        }
    }

    pub fn set_input_source_map(&mut self, source_map: UnknownReserved) {
        todo!("Not implemented");
    }

    pub fn freeze(&mut self) {
        // prune empty branches
        let map = &mut self.inner.branch_map;
        let branches = &mut self.inner.b;
        let branches_t = &mut self.inner.b_t;

        map.retain(|key, branch| {
            if branch.locations.len() == 0 {
                branches.remove_entry(key);
                if let Some(branches_t) = branches_t {
                    branches_t.remove_entry(key);
                }
                false
            } else {
                true
            }
        });
    }
}
