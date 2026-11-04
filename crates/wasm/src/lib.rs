//! Tarayıcı bağlaması. Hesap kullanıcının kendi işlemcisinde, Web Worker'larda yapılır;
//! çizim hiçbir sunucuya gönderilmez.
use js_sys::{Function, Promise};
use nestio_core::nest::{self, Event, NestInput, NestPart, PlateOut, Progress, Solver};
use nestio_core::parts::{self, AnalyzeOptions};
use nestio_core::strip::{self, StripJob, StripResult};
use nestio_core::{dxf_in, dxf_out, model::Drawing};
use std::future::Future;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, future_to_promise};

fn err(e: impl ToString) -> JsError {
    JsError::new(&e.to_string())
}

fn opts(s: &str) -> Result<AnalyzeOptions, JsError> {
    if s.is_empty() { Ok(AnalyzeOptions::default()) } else { serde_json::from_str(s).map_err(err) }
}

/// DXF baytlarından biçimden bağımsız çizim modeli (JSON).
#[wasm_bindgen]
pub fn dxf_to_drawing(bytes: &[u8]) -> Result<String, JsError> {
    let d = dxf_in::read(bytes).map_err(err)?;
    serde_json::to_string(&d).map_err(err)
}

/// Çizim modelinden parça ve plaka adayları (JSON).
#[wasm_bindgen]
pub fn analyze(drawing_json: &str, options_json: &str) -> Result<String, JsError> {
    let d: Drawing = serde_json::from_str(drawing_json).map_err(err)?;
    let a = parts::analyze(&d, &opts(options_json)?);
    serde_json::to_string(&a).map_err(err)
}

/// Tek şerit doldurma işi (havuzdaki işçiler çağırır).
#[wasm_bindgen]
pub fn solve_strip(job_json: &str) -> Result<String, JsError> {
    let job: StripJob = serde_json::from_str(job_json).map_err(err)?;
    serde_json::to_string(&strip::solve(&job)).map_err(err)
}

struct JsSolver {
    f: Function,
    width: usize,
}

impl Solver for JsSolver {
    fn width(&self) -> usize {
        self.width
    }
    fn solve_many(&self, jobs: Vec<StripJob>) -> impl Future<Output = Vec<StripResult>> {
        let n = jobs.len();
        let call = serde_json::to_string(&jobs).ok().and_then(|s| self.f.call1(&JsValue::NULL, &JsValue::from_str(&s)).ok());
        async move {
            let fail = |m: &str| vec![StripResult { error: Some(m.to_string()), ..Default::default() }; n];
            let Some(p) = call else { return fail("çözücü çağrılamadı") };
            match JsFuture::from(Promise::from(p)).await {
                Ok(v) => v.as_string().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_else(|| fail("çözücü yanıtı okunamadı")),
                Err(_) => fail("çözücü hata verdi"),
            }
        }
    }
}

struct JsProgress {
    f: Function,
    cancel: Function,
}

impl Progress for JsProgress {
    fn report(&self, ev: Event) {
        if let Ok(s) = serde_json::to_string(&ev) {
            let _ = self.f.call1(&JsValue::NULL, &JsValue::from_str(&s));
        }
    }
    fn cancelled(&self) -> bool {
        self.cancel.call0(&JsValue::NULL).map(|v| v.is_truthy()).unwrap_or(false)
    }
}

/// Bütün dizme işi. `solver(jobsJson) → Promise<resultsJson>`, `progress(eventJson)`,
/// `cancelled() → bool`. Sonuç: NestOutput JSON.
#[wasm_bindgen]
pub fn nest(input_json: String, solver: Function, width: usize, progress: Function, cancelled: Function) -> Promise {
    future_to_promise(async move {
        let input: NestInput = serde_json::from_str(&input_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let out = nest::nest(&input, &JsSolver { f: solver, width }, &JsProgress { f: progress, cancel: cancelled }).await;
        serde_json::to_string(&out).map(|s| JsValue::from_str(&s)).map_err(|e| JsValue::from_str(&e.to_string()))
    })
}

/// Dizilim DXF'i.
#[wasm_bindgen]
pub fn write_dxf(parts_json: &str, plates_json: &str, gap: f64) -> Result<Vec<u8>, JsError> {
    let parts: Vec<NestPart> = serde_json::from_str(parts_json).map_err(err)?;
    let plates: Vec<PlateOut> = serde_json::from_str(plates_json).map_err(err)?;
    dxf_out::write(&parts, &plates, gap).map_err(err)
}
