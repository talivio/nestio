//! Biçimden bağımsız çizim modeli. DXF bunu Rust tarafında (`dxf_in`), DWG ise
//! tarayıcıda LibreDWG ile okunup JavaScript tarafında bu yapıya çevrilir.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type V3 = [f64; 3];

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Drawing {
    /// Çizim birimini milimetreye çeviren çarpan (mm = 1, inç = 25.4).
    #[serde(default = "one")]
    pub units: f64,
    pub entities: Vec<Ent>,
    #[serde(default)]
    pub blocks: HashMap<String, Block>,
}

fn one() -> f64 {
    1.0
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Block {
    #[serde(default)]
    pub base: V3,
    pub entities: Vec<Ent>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Ent {
    #[serde(default)]
    pub layer: String,
    #[serde(flatten)]
    pub g: Geom,
}

fn zn() -> V3 {
    [0.0, 0.0, 1.0]
}

/// Açılar derece. `n` nesne koordinat sisteminin (OCS) normali; OCS'de verilen
/// noktalar (yay merkezi, hafif çoklu çizgi köşeleri, ekleme noktası) buna göredir.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Geom {
    Line { a: V3, b: V3 },
    Arc { c: V3, r: f64, a0: f64, a1: f64, #[serde(default = "zn")] n: V3 },
    Circle { c: V3, r: f64, #[serde(default = "zn")] n: V3 },
    /// Köşeler (x, y, z, bulge). 3B çoklu çizgide noktalar dünya koordinatındadır ve n = (0, 0, 1).
    Poly { v: Vec<[f64; 4]>, #[serde(default)] closed: bool, #[serde(default = "zn")] n: V3 },
    Spline {
        #[serde(default)]
        deg: usize,
        #[serde(default)]
        knots: Vec<f64>,
        #[serde(default)]
        ctrl: Vec<V3>,
        #[serde(default)]
        w: Vec<f64>,
        #[serde(default)]
        fit: Vec<V3>,
        #[serde(default)]
        closed: bool,
    },
    /// `p0`, `p1` parametrik açı (radyan), `major` merkeze göre büyük yarı eksen (dünya).
    Ellipse { c: V3, major: V3, ratio: f64, p0: f64, p1: f64, #[serde(default = "zn")] n: V3 },
    Insert {
        name: String,
        p: V3,
        #[serde(default = "ones")]
        s: V3,
        #[serde(default)]
        rot: f64,
        #[serde(default = "zn")]
        n: V3,
        #[serde(default = "one_i")]
        cols: u32,
        #[serde(default = "one_i")]
        rows: u32,
        #[serde(default)]
        dc: f64,
        #[serde(default)]
        dr: f64,
    },
    Text { p: V3, s: String, #[serde(default = "zn")] n: V3 },
}

fn ones() -> V3 {
    [1.0, 1.0, 1.0]
}
fn one_i() -> u32 {
    1
}
