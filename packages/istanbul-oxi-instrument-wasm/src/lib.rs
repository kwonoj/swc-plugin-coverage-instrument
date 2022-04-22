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
        serde_wasm_bindgen::to_value(&self.inner.get_line_coverage()).unwrap()
    }

    #[wasm_bindgen(js_name = "f")]
    pub fn get_f(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.inner.f).unwrap()
    }

    #[wasm_bindgen(js_name = "b")]
    pub fn get_b(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.inner.b).unwrap()
    }

    #[wasm_bindgen(js_name = "bT")]
    pub fn get_b_t(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.inner.b_t).unwrap()
    }

    #[wasm_bindgen(js_name = "s")]
    pub fn get_s(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.inner.s).unwrap()
    }
}
