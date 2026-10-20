use js_sys::Function;
use wasm_bindgen::prelude::*;

use pptx_fix_core as core;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "__emf2svgConvert")]
    fn emf2svg_hook(emf: &[u8]) -> String;
}

struct JsEmf;

impl core::EmfConverter for JsEmf {
    fn emf_to_svg(&mut self, emf: &[u8]) -> Result<String, String> {
        Ok(emf2svg_hook(emf))
    }
}

#[wasm_bindgen]
pub struct Callbacks {
    on_stage: Function,
    on_media: Function,
}

#[wasm_bindgen]
impl Callbacks {
    #[wasm_bindgen(constructor)]
    pub fn new(on_stage: Function, on_media: Function) -> Callbacks {
        Callbacks { on_stage, on_media }
    }
}

struct JsProgress(Callbacks);

impl core::Progress for JsProgress {
    fn on_stage(&self, stage: core::Stage) {
        let _ = self.0.on_stage.call1(&JsValue::NULL, &JsValue::from(stage.label()));
    }
    fn on_media(&self, cur: usize, total: usize, name: &str) {
        let _ = self.0.on_media.call3(
            &JsValue::NULL,
            &JsValue::from(cur),
            &JsValue::from(total),
            &JsValue::from(name),
        );
    }
}

#[wasm_bindgen]
pub fn fix_pptx(
    pptx: &[u8],
    fonts: Vec<js_sys::Uint8Array>,
    cb: &Callbacks,
    font_map: &js_sys::Object,
) -> Result<Vec<u8>, JsValue> {
    let font_data: Vec<Vec<u8>> = fonts.iter().map(|f| f.to_vec()).collect();
    let fdb = core::media::Fonts::from_bytes(&font_data);
    let mut map = std::collections::HashMap::new();
    let entries = js_sys::Object::entries(font_map);
    for i in 0..entries.length() {
        let pair: js_sys::Array = entries.get(i).unchecked_into();
        if let (Some(k), Some(v)) = (pair.get(0).as_string(), pair.get(1).as_string()) {
            map.insert(k, v);
        }
    }
    let mut emf = JsEmf;
    core::fix_pptx(pptx, &fdb, &mut emf, &JsProgress(Callbacks { on_stage: cb.on_stage.clone(), on_media: cb.on_media.clone() }), &core::media::Options::default(), &map)
        .map(|out| out.pptx)
        .map_err(JsValue::from)
}
