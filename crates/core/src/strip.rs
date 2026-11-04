//! Tek bir şerit doldurma işini sparrow ile çözer. Tarayıcıda her Web Worker,
//! komut satırında her iş parçacığı bu fonksiyonu ayrı tohumla çağırır.
use jagua_rs::io::ext_repr::{ExtItem as BaseItem, ExtOrientation, ExtRotation, ExtSPolygon, ExtShape};
use jagua_rs::io::import::Importer;
use jagua_rs::probs::spp::io::ext_repr::{ExtItem, ExtSPInstance};
use rand::SeedableRng;
use rand::rngs::Xoshiro256PlusPlus;
use serde::{Deserialize, Serialize};
use sparrow::config::DEFAULT_SPARROW_CONFIG;
use sparrow::optimizer::optimize;
use sparrow::util::listener::DummySolListener;
use sparrow::util::terminator::BasicTerminator;
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JobItem {
    pub id: u64,
    pub demand: u64,
    pub shape: Vec<(f32, f32)>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StripJob {
    pub items: Vec<JobItem>,
    pub height: f32,
    pub gap: f32,
    pub secs: f64,
    pub seed: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StripResult {
    pub width: f32,
    /// (öğe kimliği, döndürme derecesi, x, y)
    pub placed: Vec<(u64, f32, f32, f32)>,
    #[serde(default)]
    pub error: Option<String>,
}

pub fn solve(job: &StripJob) -> StripResult {
    match solve_inner(job) {
        Ok(r) => r,
        Err(e) => StripResult { error: Some(e), ..Default::default() },
    }
}

fn solve_inner(job: &StripJob) -> Result<StripResult, String> {
    if job.items.is_empty() {
        return Ok(StripResult::default());
    }
    let ext = ExtSPInstance {
        name: "nestio".into(),
        min_item_separation: job.gap,
        strip_height: job.height,
        items: job.items.iter().map(|it| ExtItem {
            base: BaseItem {
                id: it.id,
                orientation: ExtOrientation { rotation: ExtRotation::Continuous {}, reflection_axes: vec![] },
                shape: ExtShape::SimplePolygon(ExtSPolygon(it.shape.clone())),
                min_quality: None,
            },
            demand: it.demand,
        }).collect(),
    };
    let mut cfg = DEFAULT_SPARROW_CONFIG;
    cfg.expl_cfg.time_limit = Duration::from_secs_f64(job.secs * 0.8);
    cfg.cmpr_cfg.time_limit = Duration::from_secs_f64(job.secs * 0.2);
    let importer = Importer::new(cfg.cde_config, cfg.poly_simpl_tolerance, cfg.narrow_concavity_cutoff_ratio);
    let inst = jagua_rs::probs::spp::io::import_instance(&importer, &ext).map_err(|e| format!("{e:#}"))?;
    let sol = optimize(
        inst,
        Xoshiro256PlusPlus::seed_from_u64(job.seed),
        &mut DummySolListener,
        &mut BasicTerminator::new(),
        &cfg.expl_cfg,
        &cfg.cmpr_cfg,
        None,
    )
    .map_err(|e| format!("{e:#}"))?;
    let out = jagua_rs::probs::spp::io::export(&sol, *sparrow::EPOCH);
    Ok(StripResult {
        width: out.strip_width,
        placed: out.layout.placed_items.iter().map(|p| (p.item_id, p.transformation.rotation, p.transformation.translation.0, p.transformation.translation.1)).collect(),
        error: None,
    })
}
