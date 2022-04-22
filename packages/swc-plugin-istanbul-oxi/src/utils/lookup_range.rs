use istanbul_oxi_instrument::Range;
//TODO : swc_plugin need to import Pos
use swc_ecma_quote::swc_common::source_map::Pos;
use swc_plugin::source_map::PluginSourceMapProxy;
use swc_plugin::syntax_pos::Span;

pub fn get_range_from_span(source_map: &PluginSourceMapProxy, span: &Span) -> Range {
    let span_hi_loc = source_map.lookup_char_pos(span.hi);
    let span_lo_loc = source_map.lookup_char_pos(span.lo);

    Range::new(
        span_lo_loc.line as u32,
        span_lo_loc.col.to_u32(),
        span_hi_loc.line as u32,
        span_hi_loc.col.to_u32(),
    )
}
