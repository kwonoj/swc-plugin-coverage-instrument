use istanbul_oxi_instrument::program_visitor;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "programVisitorSpec")]
pub fn program_visitor_spec() {
    program_visitor();
}
