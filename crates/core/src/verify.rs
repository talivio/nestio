//! Dizilimi çözücüden bağımsız denetler: her parça plakanın kenar payı içinde mi,
//! iki parça arası boşluk en az istenen kadar mı, çakışma var mı.
use crate::nest::{NestPart, PlateOut};
use geo::{BoundingRect, Contains, Distance, Euclidean, Polygon, Rect, Rotate, Translate, coord};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PlateCheck {
    pub parts: usize,
    pub fill: f64,
    pub used: f64,
    pub min_dist: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Check {
    pub ok: bool,
    pub errors: Vec<String>,
    pub plates: Vec<PlateCheck>,
    pub placed: usize,
}

pub fn placed_poly(parts: &[NestPart], it: &crate::nest::Placed) -> Polygon<f64> {
    parts[it.part].shape.poly().rotate_around_point(it.rot, geo::Point::new(0.0, 0.0)).translate(it.x, it.y)
}

pub fn check(parts: &[NestPart], plates: &[PlateOut], gap: f64, margin: f64) -> Check {
    let mut c = Check { ok: true, ..Default::default() };
    for (k, pl) in plates.iter().enumerate() {
        let inner = Rect::new(coord! { x: margin - 0.01, y: margin - 0.01 }, coord! { x: pl.w - margin + 0.01, y: pl.h - margin + 0.01 }).to_polygon();
        let gs: Vec<(String, Polygon<f64>)> = pl.items.iter().map(|it| (parts[it.part].name.clone(), placed_poly(parts, it))).collect();
        let bbs: Vec<Rect<f64>> = gs.iter().map(|g| g.1.bounding_rect().unwrap()).collect();
        for (n, g) in &gs {
            if !inner.contains(g) {
                c.errors.push(format!("Plaka {}: {} kenar payının dışına taşıyor", k + 1, n));
            }
        }
        let mut min_d: Option<f64> = None;
        for i in 0..gs.len() {
            for j in i + 1..gs.len() {
                let (a, b) = (bbs[i], bbs[j]);
                if a.min().x > b.max().x + gap || b.min().x > a.max().x + gap || a.min().y > b.max().y + gap || b.min().y > a.max().y + gap {
                    continue;
                }
                let d = Euclidean.distance(&gs[i].1, &gs[j].1);
                min_d = Some(min_d.map_or(d, |m: f64| m.min(d)));
                if d < gap - 0.05 {
                    c.errors.push(format!("Plaka {}: {} ile {} arası {:.2} mm (istenen {:.1})", k + 1, gs[i].0, gs[j].0, d, gap));
                }
            }
        }
        let used = bbs.iter().map(|b| b.max().x).fold(0.0, f64::max);
        let area: f64 = gs.iter().map(|g| geo::Area::unsigned_area(&g.1)).sum();
        c.placed += gs.len();
        c.plates.push(PlateCheck { parts: gs.len(), fill: area / (pl.w * pl.h), used, min_dist: min_d });
    }
    c.ok = c.errors.is_empty();
    c
}
