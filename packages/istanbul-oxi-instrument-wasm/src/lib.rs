use istanbul_oxi_instrument::FileCoverage;
use wasm_bindgen::prelude::*;

/// Wraps FileCoverage for the wasm-bindgen to allow to use oxi-coverage in JS context
/// without oxi-coverage requires wasm-bindgen directly.
#[wasm_bindgen]
pub struct FileCoverageInterop {
    inner: FileCoverage,
}

#[wasm_bindgen]
impl FileCoverageInterop {
    #[wasm_bindgen(constructor)]
    pub fn new(val: &JsValue) -> FileCoverageInterop {
        let inner: FileCoverage = val.into_serde().unwrap();

        FileCoverageInterop { inner }
    }

    #[wasm_bindgen(js_name = "getLineCoverage")]
    pub fn get_line_coverage(&self) -> JsValue {
        let ret_index_map = self.inner.get_line_coverage();
        serde_wasm_bindgen::to_value(&ret_index_map).unwrap()
    }
}
