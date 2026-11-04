//! Çizim varlıklarını dünya koordinatında (mm) çoklu çizgilere indirir.
//! Her blok eklemesi (INSERT) ayrı bir kaynak olur; serbest varlıklar tek kaynakta toplanır.
use crate::model::{Drawing, Ent, Geom, V3};
use std::f64::consts::PI;

#[derive(Clone, Debug)]
pub struct Path {
    pub pts: Vec<V3>,
    pub layer: String,
}

#[derive(Clone, Debug, Default)]
pub struct Src {
    pub name: String,
    pub insert: bool,
    pub layer: String,
    pub paths: Vec<Path>,
    pub texts: Vec<(V3, String)>,
}

/// Eğri yaklaştırma hatası (mm).
const TOL: f64 = 0.05;

#[derive(Clone, Copy)]
struct M {
    r: [[f64; 3]; 3],
    t: V3,
}

impl M {
    fn id() -> M {
        M { r: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], t: [0.0; 3] }
    }
    fn scale(s: f64) -> M {
        M { r: [[s, 0.0, 0.0], [0.0, s, 0.0], [0.0, 0.0, s]], t: [0.0; 3] }
    }
    fn ap(&self, p: V3) -> V3 {
        let r = &self.r;
        [
            r[0][0] * p[0] + r[0][1] * p[1] + r[0][2] * p[2] + self.t[0],
            r[1][0] * p[0] + r[1][1] * p[1] + r[1][2] * p[2] + self.t[1],
            r[2][0] * p[0] + r[2][1] * p[1] + r[2][2] * p[2] + self.t[2],
        ]
    }
    /// self ∘ o  (önce o, sonra self)
    fn mul(&self, o: &M) -> M {
        let mut r = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                r[i][j] = (0..3).map(|k| self.r[i][k] * o.r[k][j]).sum();
            }
        }
        let t0 = self.ap(o.t);
        M { r, t: t0 }
    }
    /// Ortalama doğrusal ölçek (eğri yaklaştırma toleransı için).
    fn mag(&self) -> f64 {
        let c = |j: usize| (0..3).map(|i| self.r[i][j] * self.r[i][j]).sum::<f64>().sqrt();
        ((c(0) + c(1)) / 2.0).max(1e-9)
    }
}

fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: V3) -> V3 {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if l < 1e-15 { a } else { [a[0] / l, a[1] / l, a[2] / l] }
}

/// AutoCAD "keyfi eksen" algoritması: OCS → WCS dönüşümü.
fn ocs(n: V3) -> M {
    // bazı okuyucular normali (0, 0, 0) verir: varsayılan Z
    let n = if n.iter().map(|v| v * v).sum::<f64>() < 1e-18 { [0.0, 0.0, 1.0] } else { norm(n) };
    let ax = if n[0].abs() < 1.0 / 64.0 && n[1].abs() < 1.0 / 64.0 { cross([0.0, 1.0, 0.0], n) } else { cross([0.0, 0.0, 1.0], n) };
    let ax = norm(ax);
    let ay = norm(cross(n, ax));
    M { r: [[ax[0], ay[0], n[0]], [ax[1], ay[1], n[1]], [ax[2], ay[2], n[2]]], t: [0.0; 3] }
}

fn segs_for(r: f64, sweep: f64, tol: f64) -> usize {
    let r = r.abs().max(1e-9);
    let step = if tol >= r { PI / 4.0 } else { 2.0 * (1.0 - tol / r).acos() };
    ((sweep.abs() / step.max(1e-3)).ceil() as usize).clamp(4, 4096)
}

fn arc_pts(c: V3, r: f64, a0: f64, sweep: f64, tol: f64) -> Vec<V3> {
    let n = segs_for(r, sweep, tol);
    (0..=n).map(|i| {
        let a = a0 + sweep * i as f64 / n as f64;
        [c[0] + r * a.cos(), c[1] + r * a.sin(), c[2]]
    }).collect()
}

fn bulge_pts(a: V3, b: V3, bulge: f64, tol: f64, out: &mut Vec<V3>) {
    if bulge.abs() < 1e-9 {
        out.push(b);
        return;
    }
    let th = 4.0 * bulge.atan();
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let chord = (dx * dx + dy * dy).sqrt();
    if chord < 1e-12 {
        out.push(b);
        return;
    }
    let r = chord / (2.0 * (th / 2.0).sin());
    // merkez, kirişin ortasından dik yönde
    let h = r * (th / 2.0).cos();
    let (mx, my) = ((a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0);
    let (ux, uy) = (-dy / chord, dx / chord);
    let c = [mx + ux * h, my + uy * h, a[2]];
    let a0 = (a[1] - c[1]).atan2(a[0] - c[0]);
    let pts = arc_pts(c, r.abs(), a0, th, tol);
    out.extend(pts.into_iter().skip(1));
    if let Some(l) = out.last_mut() {
        *l = b;
    }
}

fn nurbs(deg: usize, knots: &[f64], ctrl: &[V3], w: &[f64], n: usize) -> Vec<V3> {
    let k = ctrl.len();
    if k < 2 || knots.len() != k + deg + 1 {
        return ctrl.to_vec();
    }
    let (u0, u1) = (knots[deg], knots[k]);
    let ww = |i: usize| if w.len() == k { w[i] } else { 1.0 };
    (0..=n).map(|s| {
        let u = (u0 + (u1 - u0) * s as f64 / n as f64).min(u1 - 1e-12 * (u1 - u0).abs().max(1.0)).max(u0);
        // açıklık bul
        let mut span = deg;
        while span < k - 1 && knots[span + 1] <= u {
            span += 1;
        }
        // de Boor, homojen koordinatlarda
        let mut d: Vec<[f64; 4]> = (0..=deg).map(|j| {
            let i = span - deg + j;
            let wi = ww(i);
            [ctrl[i][0] * wi, ctrl[i][1] * wi, ctrl[i][2] * wi, wi]
        }).collect();
        for r in 1..=deg {
            for j in (r..=deg).rev() {
                let i = span - deg + j;
                let den = knots[i + deg + 1 - r] - knots[i];
                let a = if den.abs() < 1e-15 { 0.0 } else { (u - knots[i]) / den };
                for c in 0..4 {
                    d[j][c] = (1.0 - a) * d[j - 1][c] + a * d[j][c];
                }
            }
        }
        let q = d[deg];
        [q[0] / q[3], q[1] / q[3], q[2] / q[3]]
    }).collect()
}

/// Yalnız uyum noktası olan spline: noktalardan geçen Catmull-Rom eğrisi.
fn catmull(p: &[V3], closed: bool) -> Vec<V3> {
    let n = p.len();
    if n < 3 {
        return p.to_vec();
    }
    let at = |i: isize| -> V3 {
        if closed { p[i.rem_euclid(n as isize) as usize] } else { p[i.clamp(0, n as isize - 1) as usize] }
    };
    let segs = if closed { n } else { n - 1 };
    let mut out = vec![p[0]];
    for i in 0..segs as isize {
        let (a, b, c, d) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        for k in 1..=12 {
            let t = k as f64 / 12.0;
            let (t2, t3) = (t * t, t * t * t);
            let f = |j: usize| 0.5 * (2.0 * b[j] + (c[j] - a[j]) * t + (2.0 * a[j] - 5.0 * b[j] + 4.0 * c[j] - d[j]) * t2 + (3.0 * b[j] - a[j] - 3.0 * c[j] + d[j]) * t3);
            out.push([f(0), f(1), f(2)]);
        }
    }
    out
}

struct Ctx<'a> {
    d: &'a Drawing,
    loose: Src,
    inserts: Vec<Src>,
}

/// Varlığı dünya koordinatında çoklu çizgi(ler)e çevirir. `tol` yerel birimdedir.
fn ent_paths(e: &Ent, m: &M, tol: f64) -> Vec<Vec<V3>> {
    let t = |v: Vec<V3>| -> Vec<V3> { v.into_iter().map(|p| m.ap(p)).collect() };
    match &e.g {
        Geom::Line { a, b } => vec![t(vec![*a, *b])],
        Geom::Arc { c, r, a0, a1, n } => {
            let mut sw = (a1 - a0).to_radians();
            while sw <= 0.0 {
                sw += 2.0 * PI;
            }
            let o = m.mul(&ocs(*n));
            vec![arc_pts(*c, *r, a0.to_radians(), sw, tol).into_iter().map(|p| o.ap(p)).collect()]
        }
        Geom::Circle { c, r, n } => {
            let o = m.mul(&ocs(*n));
            vec![arc_pts(*c, *r, 0.0, 2.0 * PI, tol).into_iter().map(|p| o.ap(p)).collect()]
        }
        Geom::Poly { v, closed, n } => {
            if v.is_empty() {
                return vec![];
            }
            let o = m.mul(&ocs(*n));
            let mut out = vec![[v[0][0], v[0][1], v[0][2]]];
            let cnt = if *closed { v.len() } else { v.len() - 1 };
            for i in 0..cnt {
                let a = v[i];
                let b = v[(i + 1) % v.len()];
                bulge_pts([a[0], a[1], a[2]], [b[0], b[1], b[2]], a[3], tol, &mut out);
            }
            vec![out.into_iter().map(|p| o.ap(p)).collect()]
        }
        Geom::Spline { deg, knots, ctrl, w, fit, closed } => {
            let mut pts = if ctrl.len() >= 2 {
                let n = (ctrl.len() * 12).clamp(48, 4000);
                nurbs(*deg, knots, ctrl, w, n)
            } else {
                catmull(fit, *closed)
            };
            if *closed && pts.len() > 2 {
                pts.push(pts[0]);
            }
            vec![t(pts)]
        }
        Geom::Ellipse { c, major, ratio, p0, p1, n } => {
            let nn = if n.iter().map(|v| v * v).sum::<f64>() < 1e-18 { [0.0, 0.0, 1.0] } else { norm(*n) };
            let mn = cross(nn, *major);
            let minor = [mn[0] * ratio, mn[1] * ratio, mn[2] * ratio];
            let mut sw = p1 - p0;
            while sw <= 1e-9 {
                sw += 2.0 * PI;
            }
            let rmax = (major[0] * major[0] + major[1] * major[1] + major[2] * major[2]).sqrt();
            let k = segs_for(rmax, sw, tol);
            let pts = (0..=k).map(|i| {
                let a = p0 + sw * i as f64 / k as f64;
                let (ca, sa) = (a.cos(), a.sin());
                [c[0] + major[0] * ca + minor[0] * sa, c[1] + major[1] * ca + minor[1] * sa, c[2] + major[2] * ca + minor[2] * sa]
            }).collect();
            vec![t(pts)]
        }
        Geom::Insert { .. } | Geom::Text { .. } => vec![],
    }
}

fn insert_mats(g: &Geom, base: V3) -> Vec<M> {
    let Geom::Insert { p, s, rot, n, cols, rows, dc, dr, .. } = g else { return vec![] };
    let (c, sn) = (rot.to_radians().cos(), rot.to_radians().sin());
    let o = ocs(*n);
    let mut out = vec![];
    for i in 0..(*cols).min(1000) {
        for j in 0..(*rows).min(1000) {
            let (ox, oy) = (i as f64 * dc, j as f64 * dr);
            let local = M {
                r: [[c * s[0], -sn * s[1], 0.0], [sn * s[0], c * s[1], 0.0], [0.0, 0.0, s[2]]],
                t: [p[0] + c * ox - sn * oy, p[1] + sn * ox + c * oy, p[2]],
            };
            let shift = M { r: M::id().r, t: [-base[0], -base[1], -base[2]] };
            out.push(o.mul(&local.mul(&shift)));
        }
    }
    out
}

fn walk(ctx: &mut Ctx, ents: &[Ent], m: &M, parent_layer: Option<&str>, target: Option<usize>, depth: usize) {
    if depth > 12 {
        return;
    }
    for e in ents {
        let layer = match parent_layer {
            Some(pl) if e.layer == "0" || e.layer.is_empty() => pl.to_string(),
            _ => e.layer.clone(),
        };
        match &e.g {
            Geom::Insert { name, .. } => {
                let Some(b) = ctx.d.blocks.get(name) else { continue };
                for bm in insert_mats(&e.g, b.base) {
                    let mm = m.mul(&bm);
                    let tgt = match target {
                        Some(t) => Some(t),
                        None => {
                            ctx.inserts.push(Src { name: name.clone(), insert: true, layer: layer.clone(), ..Default::default() });
                            Some(ctx.inserts.len() - 1)
                        }
                    };
                    walk(ctx, &b.entities, &mm, Some(&layer), tgt, depth + 1);
                }
            }
            Geom::Text { p, s, n } => {
                let w = m.mul(&ocs(*n)).ap(*p);
                let src = match target { Some(t) => &mut ctx.inserts[t], None => &mut ctx.loose };
                src.texts.push((w, s.clone()));
            }
            _ => {
                let tol = TOL / m.mag();
                for pts in ent_paths(e, m, tol) {
                    if pts.len() < 2 {
                        continue;
                    }
                    let src = match target { Some(t) => &mut ctx.inserts[t], None => &mut ctx.loose };
                    src.paths.push(Path { pts, layer: layer.clone() });
                }
            }
        }
    }
}

/// Serbest varlıklar (ilk eleman) ve blok eklemeleri.
pub fn flatten(d: &Drawing) -> (Src, Vec<Src>) {
    let mut ctx = Ctx { d, loose: Src { name: "serbest".into(), ..Default::default() }, inserts: vec![] };
    let root = M::scale(if d.units > 0.0 { d.units } else { 1.0 });
    walk(&mut ctx, &d.entities, &root, None, None, 0);
    ctx.inserts.retain(|s| !s.paths.is_empty());
    (ctx.loose, ctx.inserts)
}
