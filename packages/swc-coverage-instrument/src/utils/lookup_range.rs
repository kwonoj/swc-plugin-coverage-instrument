use std::sync::Arc;

use istanbul_oxide::Range;

use swc_core::common::{SourceMapper, Span};

pub fn get_range_from_span<S: SourceMapper>(source_map: &Arc<S>, span: &Span) -> Range {
    // https://github.com/swc-project/swc/issues/5535
    // There are some node types SWC passes transformed instead of original,
    // which are not able to locate original locations.
    // This'll makes to create less-accurate coverage for those types (i.e enums)
    // while waiting upstream decision instead of hard panic.
    if span.hi.is_dummy() || span.lo.is_dummy() {
        return Default::default();
    }

    let span_hi_loc = source_map.lookup_char_pos(span.hi);
    let span_lo_loc = source_map.lookup_char_pos(span.lo);

    Range::new(
        span_lo_loc.line as u32,
        // TODO: swc_plugin::source_map::Pos to use to_u32() instead
        span_lo_loc.col.0 as u32,
        span_hi_loc.line as u32,
        span_hi_loc.col.0 as u32,
    )
}
