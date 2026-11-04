//! Düzlemsel çizgi kümesinden kapalı yüzleri (delikli dolu bölgeler) çıkarır.
//! Çizgiler kesişim noktalarında bölünür, uç noktalar yakalanır, açık kalan
//! kısa aralıklar kapatılır, sarkan çizgiler atılır; yüzler yarım-kenar
//! dolaşımıyla bulunur ve iç içe kabuk sayısının tek/çift olmasına göre dolu sayılır.
use geo::{Area, Coord, InteriorPoint, LineString, MultiPolygon, Polygon};
use std::collections::{HashMap, HashSet};

pub type P2 = [f64; 2];

/// Uç nokta yakalama toleransı (mm).
pub const SNAP: f64 = 0.05;
/// Kapanmamış konturda kapatılacak en büyük aralık (mm).
pub const GAP_CLOSE: f64 = 0.5;

struct Verts {
    pts: Vec<P2>,
    grid: HashMap<(i64, i64), Vec<usize>>,
    tol: f64,
}

impl Verts {
    fn key(&self, p: P2) -> (i64, i64) {
        ((p[0] / self.tol).floor() as i64, (p[1] / self.tol).floor() as i64)
    }
    fn get(&mut self, p: P2) -> usize {
        let (kx, ky) = self.key(p);
        let mut best: Option<(f64, usize)> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(v) = self.grid.get(&(kx + dx, ky + dy)) {
                    for &i in v {
                        let q = self.pts[i];
                        let d = (q[0] - p[0]).hypot(q[1] - p[1]);
                        if d <= self.tol && best.is_none_or(|b| d < b.0) {
                            best = Some((d, i));
                        }
                    }
                }
            }
        }
        if let Some((_, i)) = best {
            return i;
        }
        self.pts.push(p);
        self.grid.entry((kx, ky)).or_default().push(self.pts.len() - 1);
        self.pts.len() - 1
    }
}

fn seg_inter(a: P2, b: P2, c: P2, d: P2, tol: f64) -> Vec<(f64, f64)> {
    // (t on ab, u on cd) çiftleri: gerçek kesişim ve uç noktanın diğer parçaya değmesi
    let mut out = vec![];
    let r = [b[0] - a[0], b[1] - a[1]];
    let s = [d[0] - c[0], d[1] - c[1]];
    let rr = r[0] * r[0] + r[1] * r[1];
    let ss = s[0] * s[0] + s[1] * s[1];
    if rr < 1e-18 || ss < 1e-18 {
        return out;
    }
    let den = r[0] * s[1] - r[1] * s[0];
    let qp = [c[0] - a[0], c[1] - a[1]];
    if den.abs() > 1e-12 * (rr * ss).sqrt() {
        let t = (qp[0] * s[1] - qp[1] * s[0]) / den;
        let u = (qp[0] * r[1] - qp[1] * r[0]) / den;
        let et = tol / rr.sqrt();
        let eu = tol / ss.sqrt();
        if t > -et && t < 1.0 + et && u > -eu && u < 1.0 + eu {
            out.push((t.clamp(0.0, 1.0), u.clamp(0.0, 1.0)));
        }
    }
    // uç noktaların öbür parçaya izdüşümü (paralel/çakışık durumlar dahil)
    let proj = |p: P2, o: P2, v: [f64; 2], vv: f64| -> Option<f64> {
        let t = ((p[0] - o[0]) * v[0] + (p[1] - o[1]) * v[1]) / vv;
        if !(-1e-9..=1.0 + 1e-9).contains(&t) {
            return None;
        }
        let q = [o[0] + v[0] * t, o[1] + v[1] * t];
        if (q[0] - p[0]).hypot(q[1] - p[1]) <= tol { Some(t.clamp(0.0, 1.0)) } else { None }
    };
    if let Some(u) = proj(a, c, s, ss) { out.push((0.0, u)); }
    if let Some(u) = proj(b, c, s, ss) { out.push((1.0, u)); }
    if let Some(t) = proj(c, a, r, rr) { out.push((t, 0.0)); }
    if let Some(t) = proj(d, a, r, rr) { out.push((t, 1.0)); }
    out
}

/// Bir yüz: dış halka (saat yönü tersine) ve delikleri.
#[derive(Clone, Debug)]
pub struct Face {
    pub shell: Vec<P2>,
    pub holes: Vec<Vec<P2>>,
    pub layer: usize,
}

pub fn ring_area(r: &[P2]) -> f64 {
    let n = r.len();
    (0..n).map(|i| {
        let (a, b) = (r[i], r[(i + 1) % n]);
        a[0] * b[1] - b[0] * a[1]
    }).sum::<f64>() / 2.0
}

pub fn pip(r: &[P2], p: P2) -> bool {
    let mut inside = false;
    let n = r.len();
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (r[i], r[j]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn bbox(r: &[P2]) -> [f64; 4] {
    let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for p in r {
        b[0] = b[0].min(p[0]);
        b[1] = b[1].min(p[1]);
        b[2] = b[2].max(p[0]);
        b[3] = b[3].max(p[1]);
    }
    b
}

/// Çizgilerden yüzleri bulur. Her çizginin katman numarası taşınır; yüzün katmanı
/// sınır kenarlarında en sık geçen katmandır.
pub fn faces(paths: &[(Vec<P2>, usize)]) -> Vec<Face> {
    let mut vs = Verts { pts: vec![], grid: HashMap::new(), tol: SNAP };
    // 1. parçalar
    let mut segs: Vec<(P2, P2, usize)> = vec![];
    for (pts, l) in paths {
        for w in pts.windows(2) {
            if (w[0][0] - w[1][0]).hypot(w[0][1] - w[1][1]) > SNAP * 0.2 {
                segs.push((w[0], w[1], *l));
            }
        }
    }
    // 2. kesişimler (x'e göre süpürme)
    let mut order: Vec<usize> = (0..segs.len()).collect();
    let minx = |s: &(P2, P2, usize)| s.0[0].min(s.1[0]);
    let maxx = |s: &(P2, P2, usize)| s.0[0].max(s.1[0]);
    order.sort_by(|&i, &j| minx(&segs[i]).total_cmp(&minx(&segs[j])));
    let mut cuts: Vec<Vec<f64>> = vec![vec![0.0, 1.0]; segs.len()];
    let mut active: Vec<usize> = vec![];
    for &i in &order {
        let x0 = minx(&segs[i]) - SNAP;
        active.retain(|&j| maxx(&segs[j]) >= x0);
        let (a, b, _) = segs[i];
        let (ylo, yhi) = (a[1].min(b[1]) - SNAP, a[1].max(b[1]) + SNAP);
        for &j in &active {
            let (c, d, _) = segs[j];
            if c[1].max(d[1]) < ylo || c[1].min(d[1]) > yhi {
                continue;
            }
            for (t, u) in seg_inter(a, b, c, d, SNAP) {
                cuts[i].push(t);
                cuts[j].push(u);
            }
        }
        active.push(i);
    }
    // 3. kenarlar
    let mut edges: HashMap<(usize, usize), usize> = HashMap::new();
    for (i, (a, b, l)) in segs.iter().enumerate() {
        let c = &mut cuts[i];
        c.sort_by(|x, y| x.total_cmp(y));
        c.dedup_by(|x, y| (*x - *y).abs() < 1e-12);
        let mut prev = vs.get(*a);
        for &t in c.iter().skip(1) {
            let p = if t >= 1.0 { *b } else { [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t] };
            let v = vs.get(p);
            if v != prev {
                edges.entry((prev.min(v), prev.max(v))).or_insert(*l);
            }
            prev = v;
        }
    }
    let nv = vs.pts.len();
    let mut adj: Vec<HashSet<usize>> = vec![HashSet::new(); nv];
    for &(u, v) in edges.keys() {
        adj[u].insert(v);
        adj[v].insert(u);
    }
    // 4. kısa aralıkları kapat: tek bağlı uçları en yakın tek bağlı uca bağla
    let ends: Vec<usize> = (0..nv).filter(|&v| adj[v].len() == 1).collect();
    if !ends.is_empty() {
        let mut eg: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        let k = |p: P2| ((p[0] / GAP_CLOSE).floor() as i64, (p[1] / GAP_CLOSE).floor() as i64);
        for &v in &ends {
            eg.entry(k(vs.pts[v])).or_default().push(v);
        }
        let mut used = HashSet::new();
        for &v in &ends {
            if used.contains(&v) || adj[v].len() != 1 {
                continue;
            }
            let p = vs.pts[v];
            let (kx, ky) = k(p);
            let mut best: Option<(f64, usize)> = None;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for &w in eg.get(&(kx + dx, ky + dy)).into_iter().flatten() {
                        if w == v || used.contains(&w) || adj[v].contains(&w) {
                            continue;
                        }
                        let q = vs.pts[w];
                        let d = (q[0] - p[0]).hypot(q[1] - p[1]);
                        if d <= GAP_CLOSE && best.is_none_or(|b| d < b.0) {
                            best = Some((d, w));
                        }
                    }
                }
            }
            if let Some((_, w)) = best {
                let l = *edges.get(&(v.min(*adj[v].iter().next().unwrap()), v.max(*adj[v].iter().next().unwrap()))).unwrap_or(&0);
                edges.insert((v.min(w), v.max(w)), l);
                adj[v].insert(w);
                adj[w].insert(v);
                used.insert(v);
                used.insert(w);
            }
        }
    }
    // 5. sarkan kenarları ayıkla
    let mut stack: Vec<usize> = (0..nv).filter(|&v| adj[v].len() == 1).collect();
    while let Some(v) = stack.pop() {
        if adj[v].len() != 1 {
            continue;
        }
        let w = *adj[v].iter().next().unwrap();
        adj[v].clear();
        adj[w].remove(&v);
        if adj[w].len() == 1 {
            stack.push(w);
        }
    }
    // 6. yarım-kenar dolaşımı
    let pts = &vs.pts;
    let sorted: Vec<Vec<usize>> = (0..nv).map(|u| {
        let mut n: Vec<usize> = adj[u].iter().copied().collect();
        n.sort_by(|&a, &b| {
            let aa = (pts[a][1] - pts[u][1]).atan2(pts[a][0] - pts[u][0]);
            let bb = (pts[b][1] - pts[u][1]).atan2(pts[b][0] - pts[u][0]);
            aa.total_cmp(&bb)
        });
        n
    }).collect();
    // bileşen numarası (birleşim-bul)
    let mut comp: Vec<usize> = (0..nv).collect();
    fn find(c: &mut [usize], x: usize) -> usize {
        let mut x = x;
        while c[x] != x {
            c[x] = c[c[x]];
            x = c[x];
        }
        x
    }
    for u in 0..nv {
        for &v in &sorted[u] {
            let (a, b) = (find(&mut comp, u), find(&mut comp, v));
            if a != b {
                comp[a] = b;
            }
        }
    }
    let mut visited: HashSet<(usize, usize)> = HashSet::new();
    // (köşe kimlikleri, bileşen, katman, bileşenin dış sınırı mı)
    let mut pos_rings: Vec<(Vec<usize>, usize, usize)> = vec![];
    let mut neg_rings: Vec<(Vec<usize>, usize, bool)> = vec![];
    for u in 0..nv {
        for &v in &sorted[u] {
            if visited.contains(&(u, v)) {
                continue;
            }
            let mut ring = vec![];
            let mut lay: HashMap<usize, usize> = HashMap::new();
            let (mut a, mut b) = (u, v);
            let mut guard = 0;
            loop {
                visited.insert((a, b));
                ring.push(a);
                *lay.entry(*edges.get(&(a.min(b), a.max(b))).unwrap_or(&0)).or_default() += 1;
                let nb = &sorted[b];
                let i = nb.iter().position(|&x| x == a).unwrap();
                let c = nb[(i + nb.len() - 1) % nb.len()];
                a = b;
                b = c;
                guard += 1;
                if (a == u && b == v) || guard > 4 * edges.len() + 8 {
                    break;
                }
            }
            let cp = find(&mut comp, u);
            let l = lay.into_iter().max_by_key(|x| x.1).map(|x| x.0).unwrap_or(0);
            let whole = ring_area(&ring.iter().map(|&i| pts[i]).collect::<Vec<_>>());
            // tek noktada değen halkalar basit döngülere bölünür
            for lp in split_simple(&ring) {
                if lp.len() < 3 {
                    continue;
                }
                let ar = ring_area(&lp.iter().map(|&i| pts[i]).collect::<Vec<_>>());
                if ar > 1e-6 {
                    pos_rings.push((lp, cp, l));
                } else if ar < -1e-6 {
                    neg_rings.push((lp, cp, whole < 0.0));
                }
            }
        }
    }
    // 7. negatif döngüleri, onları içeren en küçük yüze delik yap
    let ring_pts = |r: &Vec<usize>| r.iter().map(|&i| pts[i]).collect::<Vec<P2>>();
    let pr: Vec<Vec<P2>> = pos_rings.iter().map(|r| ring_pts(&r.0)).collect();
    let pb: Vec<[f64; 4]> = pr.iter().map(|r| bbox(r)).collect();
    let pa: Vec<f64> = pr.iter().map(|r| ring_area(r)).collect();
    let pset: Vec<HashSet<usize>> = pos_rings.iter().map(|r| r.0.iter().copied().collect()).collect();
    let mut out: Vec<Face> = pos_rings.iter().zip(&pr).map(|(r, p)| Face { shell: p.clone(), holes: vec![], layer: r.2 }).collect();
    for (nr, cp, outer) in neg_rings {
        let mut best: Option<usize> = None;
        for i in 0..pos_rings.len() {
            if outer && pos_rings[i].1 == cp {
                continue;
            }
            let Some(&tv) = nr.iter().find(|v| !pset[i].contains(v)) else { continue };
            let p = pts[tv];
            let b = pb[i];
            if p[0] < b[0] || p[0] > b[2] || p[1] < b[1] || p[1] > b[3] {
                continue;
            }
            if pip(&pr[i], p) && best.is_none_or(|j| pa[i] < pa[j]) {
                best = Some(i);
            }
        }
        if let Some(i) = best {
            out[i].holes.push(ring_pts(&nr));
        }
    }
    out
}

/// Köşesi tekrar eden kapalı yolu basit döngülere ayırır.
fn split_simple(ring: &[usize]) -> Vec<Vec<usize>> {
    let mut out = vec![];
    let mut stack: Vec<usize> = vec![];
    let mut pos: HashMap<usize, usize> = HashMap::new();
    for &v in ring {
        if let Some(&k) = pos.get(&v) {
            let lp: Vec<usize> = stack.drain(k..).collect();
            for x in &lp {
                pos.remove(x);
            }
            out.push(lp);
        }
        pos.insert(v, stack.len());
        stack.push(v);
    }
    if !stack.is_empty() {
        out.push(stack);
    }
    out
}

pub fn to_poly(shell: &[P2], holes: &[Vec<P2>]) -> Polygon<f64> {
    let ls = |r: &[P2]| LineString::from(r.iter().map(|p| Coord { x: p[0], y: p[1] }).collect::<Vec<_>>());
    Polygon::new(ls(shell), holes.iter().map(|h| ls(h)).collect())
}

/// Tek/çift kuralıyla dolu yüzleri seçip birleştirir. Dönen her çokgen bir parçadır.
pub fn solids(fs: &[Face]) -> Vec<(Polygon<f64>, usize)> {
    let bb: Vec<[f64; 4]> = fs.iter().map(|f| bbox(&f.shell)).collect();
    let mut solid = vec![];
    for f in fs.iter() {
        let poly = to_poly(&f.shell, &f.holes);
        let Some(q) = poly.interior_point() else { continue };
        let q = [q.x(), q.y()];
        let cnt = fs.iter().enumerate().filter(|(j, g)| {
            let b = bb[*j];
            q[0] >= b[0] && q[0] <= b[2] && q[1] >= b[1] && q[1] <= b[3] && pip(&g.shell, q)
        }).count();
        if cnt % 2 == 1 {
            solid.push((poly, f.layer));
        }
    }
    if solid.is_empty() {
        return vec![];
    }
    let polys: Vec<Polygon<f64>> = solid.iter().map(|s| s.0.clone()).collect();
    let u: MultiPolygon<f64> = geo::unary_union(&polys);
    u.0.into_iter().filter(|p| p.unsigned_area() > 1e-6).map(|p| {
        // katman: içindeki ilk dolu yüzün katmanı
        let l = solid.iter().find(|s| {
            let ip = s.0.interior_point().unwrap();
            geo::Contains::contains(&p, &ip)
        }).map(|s| s.1).unwrap_or(0);
        (p, l)
    }).collect()
}

/// Çift çizgiden doğan ince halka biçimli delikleri kapatır (ortalama genişlik < 2 mm).
pub fn tidy(p: &Polygon<f64>) -> Polygon<f64> {
    let ext = Polygon::new(p.exterior().clone(), vec![]);
    let ea = ext.unsigned_area();
    let el = geo::Euclidean.length(p.exterior());
    let holes = p.interiors().iter().filter(|h| {
        let ha = Polygon::new((*h).clone(), vec![]).unsigned_area();
        (ea - ha) / el >= 2.0 && ha > 0.01
    }).cloned().collect();
    Polygon::new(p.exterior().clone(), holes)
}

use geo::Length;

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<P2> {
        vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]]
    }
    #[test]
    fn delikli_dikdortgen_ve_icindeki_parca() {
        let paths = vec![(rect(0., 0., 100., 50.), 0), (rect(10., 10., 40., 40.), 0), (rect(15., 15., 25., 25.), 0)];
        let s = solids(&faces(&paths));
        assert_eq!(s.len(), 2);
        let mut a: Vec<f64> = s.iter().map(|x| x.0.unsigned_area()).collect();
        a.sort_by(|x, y| x.total_cmp(y));
        assert!((a[0] - 100.0).abs() < 1e-6);
        assert!((a[1] - (5000.0 - 900.0)).abs() < 1e-6);
    }
    #[test]
    fn dis_cizgiye_degen_delik() {
        // delik, dış sınıra tek köşede değiyor
        let paths = vec![(rect(0., 0., 100., 50.), 0), (vec![[0., 25.], [30., 10.], [30., 40.], [0., 25.]], 0)];
        let s = solids(&faces(&paths));
        assert_eq!(s.len(), 1);
        assert!((s[0].0.unsigned_area() - (5000.0 - 450.0)).abs() < 1e-6, "{}", s[0].0.unsigned_area());
    }
    #[test]
    fn ic_cizgi_ve_aralik() {
        // ortadan bölünmüş dikdörtgen (iç çizgi) + 0.3 mm açık köşe
        let paths = vec![
            (vec![[0., 0.], [100., 0.], [100., 50.], [0., 50.], [0., 0.3]], 0),
            (vec![[50., 0.], [50., 50.]], 0),
            (vec![[200., 0.], [210., 0.]], 0), // sarkan
        ];
        let s = solids(&faces(&paths));
        assert_eq!(s.len(), 1);
        assert!((s[0].0.unsigned_area() - 5000.0).abs() < 20.0);
    }
}
