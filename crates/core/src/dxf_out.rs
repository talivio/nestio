//! Dizilimi DXF olarak yazar. Plakalar yan yana dizilir; katmanlar:
//! PLAKA (plaka sınırı), KESIM (parça konturları), ETIKET (parça adı), ARTIK (kalan dikdörtgen).
use crate::nest::{NestPart, PlateOut};
use crate::verify::placed_poly;
use dxf::entities::{Entity, EntityType, LwPolyline, Text};
use dxf::enums::{AcadVersion, HorizontalTextJustification, Units, VerticalTextJustification};
use dxf::tables::Layer;
use dxf::{Color, Drawing, LwPolylineVertex, Point};
use geo::InteriorPoint;

fn poly(d: &mut Drawing, pts: &[(f64, f64)], layer: &str) {
    let mut p = LwPolyline::default();
    p.set_is_closed(true);
    p.vertices = pts.iter().map(|&(x, y)| LwPolylineVertex { x, y, ..Default::default() }).collect();
    let mut e = Entity::new(EntityType::LwPolyline(p));
    e.common.layer = layer.into();
    d.add_entity(e);
}

/// Plakaların çizimdeki x konumları.
pub fn offsets(plates: &[PlateOut]) -> Vec<f64> {
    let mut x = 0.0;
    plates.iter().map(|p| {
        let o = x;
        x += p.w + (p.w * 0.1).max(200.0);
        o
    }).collect()
}

pub fn write(parts: &[NestPart], plates: &[PlateOut], gap: f64) -> Result<Vec<u8>, String> {
    let mut d = Drawing::new();
    d.header.version = AcadVersion::R2010;
    d.header.default_drawing_units = Units::Millimeters;
    for (name, col) in [("PLAKA", 4u8), ("KESIM", 7), ("ETIKET", 3), ("ARTIK", 8)] {
        let mut l = Layer::default();
        l.name = name.into();
        l.color = Color::from_index(col);
        d.add_layer(l);
    }
    let offs = offsets(plates);
    for (pl, &ox) in plates.iter().zip(&offs) {
        poly(&mut d, &[(ox, 0.0), (ox + pl.w, 0.0), (ox + pl.w, pl.h), (ox, pl.h)], "PLAKA");
        let cut = (pl.used + gap).min(pl.w);
        if pl.w - cut > 1.0 {
            poly(&mut d, &[(ox + cut, 0.0), (ox + pl.w, 0.0), (ox + pl.w, pl.h), (ox + cut, pl.h)], "ARTIK");
        }
        for it in &pl.items {
            let g = placed_poly(parts, it);
            for r in std::iter::once(g.exterior()).chain(g.interiors()) {
                let mut v: Vec<(f64, f64)> = r.0.iter().map(|c| (c.x + ox, c.y)).collect();
                if v.len() > 1 && v.first() == v.last() {
                    v.pop();
                }
                poly(&mut d, &v, "KESIM");
            }
            if let Some(ip) = g.interior_point() {
                let mut t = Text::default();
                t.value = parts[it.part].name.clone();
                t.text_height = 12.0;
                t.location = Point::new(ip.x() + ox, ip.y(), 0.0);
                t.second_alignment_point = t.location.clone();
                t.horizontal_text_justification = HorizontalTextJustification::Middle;
                t.vertical_text_justification = VerticalTextJustification::Middle;
                let mut e = Entity::new(EntityType::Text(t));
                e.common.layer = "ETIKET".into();
                d.add_entity(e);
            }
        }
    }
    let mut buf = vec![];
    d.save(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}
