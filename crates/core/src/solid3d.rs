//! 3B tel kafes çizilmiş levha parçayı düzleme indirir. Levhanın normali, aday
//! normaller içinden parçayı en ince gösteren yön olarak seçilir; o yöndeki
//! en alt yüzün çizgileri yüz olarak çözülür, olmazsa silüet alınır.
use crate::flatten::Path;
use crate::model::V3;
use crate::planar::{self, P2};
use geo::{Area, Buffer, ConvexHull, Coord, LineString, MinimumRotatedRect, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};
use geo::algorithm::buffer::{BufferStyle, LineCap, LineJoin};

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn len(a: V3) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: V3) -> V3 {
    let l = len(a);
    [a[0] / l, a[1] / l, a[2] / l]
}

/// Simetrik 3×3 matrisin özdeğer/özvektörleri (Jacobi). Küçükten büyüğe.
fn eig3(mut a: [[f64; 3]; 3]) -> ([f64; 3], [V3; 3]) {
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..50 {
        let (mut p, mut q, mut m) = (0, 1, a[0][1].abs());
        for (i, j) in [(0, 2), (1, 2)] {
            if a[i][j].abs() > m {
                m = a[i][j].abs();
                p = i;
                q = j;
            }
        }
        if m < 1e-14 {
            break;
        }
        let th = 0.5 * (2.0 * a[p][q]).atan2(a[q][q] - a[p][p]);
        let (c, s) = (th.cos(), th.sin());
        for k in 0..3 {
            let (akp, akq) = (a[k][p], a[k][q]);
            a[k][p] = c * akp - s * akq;
            a[k][q] = s * akp + c * akq;
        }
        for k in 0..3 {
            let (apk, aqk) = (a[p][k], a[q][k]);
            a[p][k] = c * apk - s * aqk;
            a[q][k] = s * apk + c * aqk;
        }
        for row in v.iter_mut() {
            let (vkp, vkq) = (row[p], row[q]);
            row[p] = c * vkp - s * vkq;
            row[q] = s * vkp + c * vkq;
        }
    }
    let mut idx = [0, 1, 2];
    idx.sort_by(|&i, &j| a[i][i].total_cmp(&a[j][j]));
    let vals = [a[idx[0]][idx[0]], a[idx[1]][idx[1]], a[idx[2]][idx[2]]];
    let vecs = [0, 1, 2].map(|k| [v[0][idx[k]], v[1][idx[k]], v[2][idx[k]]]);
    (vals, vecs)
}

fn plane_fit(pts: &[V3]) -> ([f64; 3], [V3; 3]) {
    let n = pts.len() as f64;
    let mut c = [0.0; 3];
    for p in pts {
        for k in 0..3 {
            c[k] += p[k] / n;
        }
    }
    let mut m = [[0.0; 3]; 3];
    for p in pts {
        let d = sub(*p, c);
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += d[i] * d[j];
            }
        }
    }
    eig3(m)
}

pub struct Flat {
    pub poly: MultiPolygon<f64>,
    pub thick: f64,
    pub silhouette: bool,
    /// Bütün çizgilerin dolu dış sınırı (kalınlığı tutmayan parçada yedek).
    pub sil: MultiPolygon<f64>,
}

fn lines2(paths: &[&Path], c: V3, a: V3, b: V3) -> Vec<Vec<P2>> {
    paths.iter().map(|p| p.pts.iter().map(|q| {
        let d = sub(*q, c);
        [dot(d, a), dot(d, b)]
    }).collect()).collect()
}

fn mls(lines: &[Vec<P2>]) -> MultiLineString<f64> {
    MultiLineString::new(lines.iter().filter(|l| l.len() > 1).map(|l| LineString::from(l.iter().map(|p| Coord { x: p[0], y: p[1] }).collect::<Vec<_>>())).collect())
}

fn buf(g: &MultiLineString<f64>, r: f64) -> MultiPolygon<f64> {
    g.buffer_with_style(BufferStyle::new(r).line_join(LineJoin::Miter(2.0)).line_cap(LineCap::Square))
}

fn mbuf(g: &MultiPolygon<f64>, r: f64) -> MultiPolygon<f64> {
    g.buffer_with_style(BufferStyle::new(r).line_join(LineJoin::Miter(2.0)))
}

fn exteriors(g: &MultiPolygon<f64>) -> MultiPolygon<f64> {
    let ps: Vec<Polygon<f64>> = g.0.iter().map(|p| Polygon::new(p.exterior().clone(), vec![])).collect();
    geo::unary_union(&ps)
}

/// Silüet: çizgiler `r` kadar kalınlaştırılıp birleştirilir, dış sınırdan geri çekilir.
/// Genişliği 3 mm'den büyük iç boşluklar delik olarak korunur.
fn silhouette(lines: &[Vec<P2>], r: f64) -> MultiPolygon<f64> {
    let u = buf(&mls(lines), r);
    let mut out: Vec<Polygon<f64>> = vec![];
    for q in &u.0 {
        let outer = mbuf(&MultiPolygon::new(vec![Polygon::new(q.exterior().clone(), vec![])]), -r);
        let mut holes: Vec<Polygon<f64>> = vec![];
        for h in q.interiors() {
            let hp = Polygon::new(h.clone(), vec![]);
            let w = hp.minimum_rotated_rect().map(|rr| {
                let c: Vec<Coord<f64>> = rr.exterior().0.clone();
                let d = |i: usize, j: usize| (c[i].x - c[j].x).hypot(c[i].y - c[j].y);
                d(0, 1).min(d(1, 2))
            }).unwrap_or(0.0);
            if w > 3.0 {
                holes.extend(mbuf(&MultiPolygon::new(vec![hp]), r).0);
            }
        }
        let o = if holes.is_empty() { outer } else { geo::BooleanOps::difference(&outer, &MultiPolygon::new(holes)) };
        out.extend(o.0);
    }
    geo::unary_union(&out)
}

pub fn flatten_part(paths: &[&Path]) -> Option<Flat> {
    let all: Vec<V3> = paths.iter().flat_map(|p| p.pts.iter().copied()).collect();
    if all.len() < 3 {
        return None;
    }
    let (_, ev) = plane_fit(&all);
    let mut cands: Vec<V3> = vec![ev[0]];
    for p in paths {
        if p.pts.len() >= 3 {
            let (vals, v) = plane_fit(&p.pts);
            if vals[1] > 1.0 {
                cands.push(v[0]);
            }
        }
    }
    let mut longs: Vec<V3> = paths.iter().filter(|p| p.pts.len() == 2).map(|p| sub(p.pts[1], p.pts[0])).collect();
    longs.sort_by(|a, b| len(*b).total_cmp(&len(*a)));
    longs.truncate(15);
    for i in 0..longs.len() {
        for j in i + 1..longs.len() {
            let w = cross(longs[i], longs[j]);
            if len(w) > 1e-6 * len(longs[i]) * len(longs[j]) {
                cands.push(unit(w));
            }
        }
    }
    // kısa doğrular çoğunlukla kalınlık kenarlarıdır
    let mut shorts: Vec<V3> = paths.iter().filter(|p| p.pts.len() == 2).map(|p| sub(p.pts[1], p.pts[0])).filter(|v| len(*v) > 0.5 && len(*v) < 40.0).collect();
    shorts.truncate(60);
    cands.extend(shorts.into_iter().map(unit));
    let kalin = |n: V3| {
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for p in &all {
            let d = dot(*p, n);
            lo = lo.min(d);
            hi = hi.max(d);
        }
        hi - lo
    };
    let n = cands.into_iter().filter(|v| len(*v) > 0.5).min_by(|a, b| kalin(*a).total_cmp(&kalin(*b)))?;
    // Normalin işareti adaylardan keyfi gelir (öz vektör, çizgi yönü); ters işaret parçayı
    // alttan okur, yani AYNALAR. Panel, normaline en yakın standart görünüşten bakıldığı gibi
    // okunur: üst (+Z'den), ön (−Y'den), sağ (+X'ten). (a, b, n) sağ el, bakan +n tarafında.
    let k = (0..3).max_by(|&i, &j| n[i].abs().total_cmp(&n[j].abs())).unwrap();
    let n = if [1.0, -1.0, 1.0][k] * n[k] < 0.0 { [-n[0], -n[1], -n[2]] } else { n };
    let a =unit(if n[2].abs() < 0.9 { cross(n, [0.0, 0.0, 1.0]) } else { cross(n, [1.0, 0.0, 0.0]) });
    let b = cross(n, a);
    let c = all[0];
    let ds: Vec<(f64, f64)> = paths.iter().map(|p| {
        let v: Vec<f64> = p.pts.iter().map(|q| dot(sub(*q, c), n)).collect();
        (v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max))
    }).collect();
    let dmin = ds.iter().map(|d| d.0).fold(f64::MAX, f64::min);
    let dmax = ds.iter().map(|d| d.1).fold(f64::MIN, f64::max);
    let thick = dmax - dmin;
    let face: Vec<&Path> = paths.iter().zip(&ds).filter(|(_, d)| d.0 - dmin < 0.5 && d.1 - dmin < 0.5).map(|(p, _)| *p).collect();
    let all2 = lines2(paths, c, a, b);
    let hull = MultiPoint::from(all2.iter().flatten().map(|p| Point::new(p[0], p[1])).collect::<Vec<_>>()).convex_hull().unsigned_area();
    let f2: Vec<(Vec<P2>, usize)> = lines2(&face, c, a, b).into_iter().map(|l| (l, 0)).collect();
    let mut g = MultiPolygon::new(planar::solids(&planar::faces(&f2)).into_iter().map(|x| x.0).collect());
    let mut sil_used = false;
    if g.0.is_empty() || g.unsigned_area() < 0.15 * hull {
        let ls: Vec<Vec<P2>> = all2.iter().filter(|l| {
            l.windows(2).map(|w| (w[0][0] - w[1][0]).hypot(w[0][1] - w[1][1])).sum::<f64>() > 0.2
        }).cloned().collect();
        for r in [0.3, 1.0, 2.0, 3.0] {
            g = silhouette(&ls, r);
            if g.unsigned_area() > 0.15 * hull {
                break;
            }
        }
        sil_used = true;
    }
    let g = MultiPolygon::new(mbuf(&mbuf(&g, 0.05), -0.05).0.iter().map(planar::tidy).collect());
    let sil = mbuf(&exteriors(&buf(&mls(&all2), 1.0)), -1.0);
    let (g, sil_used) = if g.0.is_empty() || g.unsigned_area() < 0.15 * sil.unsigned_area() { (sil.clone(), true) } else { (g, sil_used) };
    Some(Flat { poly: g, thick, silhouette: sil_used, sil })
}
