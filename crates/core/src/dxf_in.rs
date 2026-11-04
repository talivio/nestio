//! DXF dosyasını `model::Drawing` yapısına çevirir.
use crate::model::{Block, Drawing, Ent, Geom, V3};
use dxf::entities::EntityType;
use std::collections::HashMap;
use std::io::Cursor;

fn p(v: &dxf::Point) -> V3 {
    [v.x, v.y, v.z]
}
fn n(v: &dxf::Vector) -> V3 {
    let l = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    if l < 1e-12 { [0.0, 0.0, 1.0] } else { [v.x / l, v.y / l, v.z / l] }
}

pub fn read(bytes: &[u8]) -> Result<Drawing, String> {
    let mut cur = Cursor::new(bytes);
    let d = dxf::Drawing::load(&mut cur).map_err(|e| format!("DXF okunamadı: {e}"))?;
    let units = match d.header.default_drawing_units {
        dxf::enums::Units::Inches => 25.4,
        dxf::enums::Units::Feet => 304.8,
        dxf::enums::Units::Centimeters => 10.0,
        dxf::enums::Units::Meters => 1000.0,
        dxf::enums::Units::Decimeters => 100.0,
        _ => 1.0,
    };
    let entities = d.entities().filter_map(conv).collect();
    let mut blocks = HashMap::new();
    for b in d.blocks() {
        blocks.insert(b.name.clone(), Block { base: p(&b.base_point), entities: b.entities.iter().filter_map(conv).collect() });
    }
    Ok(Drawing { units, entities, blocks })
}

fn conv(e: &dxf::entities::Entity) -> Option<Ent> {
    let layer = e.common.layer.clone();
    if e.common.is_in_paper_space {
        return None;
    }
    let g = match &e.specific {
        EntityType::Line(l) => Geom::Line { a: p(&l.p1), b: p(&l.p2) },
        EntityType::Arc(a) => Geom::Arc { c: p(&a.center), r: a.radius, a0: a.start_angle, a1: a.end_angle, n: n(&a.normal) },
        EntityType::Circle(c) => Geom::Circle { c: p(&c.center), r: c.radius, n: n(&c.normal) },
        EntityType::LwPolyline(l) => Geom::Poly {
            v: l.vertices.iter().map(|v| [v.x, v.y, 0.0, v.bulge]).collect(),
            closed: l.flags & 1 == 1,
            n: n(&l.extrusion_direction),
        },
        EntityType::Polyline(pl) => {
            // Çokgen ağları ve yüzlü ağlar parça çizgisi değildir.
            if pl.flags & (16 | 64) != 0 {
                return None;
            }
            let three_d = pl.flags & 8 != 0;
            let elev = pl.location.z;
            let v = pl
                .vertices()
                .filter(|v| v.flags & 16 == 0) // spline denetim noktaları değil
                .map(|v| [v.location.x, v.location.y, if three_d { v.location.z } else { elev }, if three_d { 0.0 } else { v.bulge }])
                .collect();
            Geom::Poly { v, closed: pl.flags & 1 == 1, n: if three_d { [0.0, 0.0, 1.0] } else { n(&pl.normal) } }
        }
        EntityType::Spline(s) => Geom::Spline {
            deg: s.degree_of_curve.max(1) as usize,
            knots: s.knot_values.clone(),
            ctrl: s.control_points.iter().map(p).collect(),
            w: s.weight_values.clone(),
            fit: s.fit_points.iter().map(p).collect(),
            closed: s.flags & 1 == 1,
        },
        EntityType::Ellipse(el) => Geom::Ellipse {
            c: p(&el.center),
            major: [el.major_axis.x, el.major_axis.y, el.major_axis.z],
            ratio: el.minor_axis_ratio,
            p0: el.start_parameter,
            p1: el.end_parameter,
            n: n(&el.normal),
        },
        EntityType::Insert(i) => Geom::Insert {
            name: i.name.clone(),
            p: p(&i.location),
            s: [i.x_scale_factor, i.y_scale_factor, i.z_scale_factor],
            rot: i.rotation,
            n: n(&i.extrusion_direction),
            cols: i.column_count.max(1) as u32,
            rows: i.row_count.max(1) as u32,
            dc: i.column_spacing,
            dr: i.row_spacing,
        },
        EntityType::Text(t) => Geom::Text { p: p(&t.location), s: t.value.clone(), n: n(&t.normal) },
        EntityType::MText(t) => Geom::Text { p: p(&t.insertion_point), s: format!("{}{}", t.extended_text.join(""), t.text), n: n(&t.extrusion_direction) },
        _ => return None,
    };
    Some(Ent { layer, g })
}
