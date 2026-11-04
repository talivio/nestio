//! Çizimden parça ve plaka adaylarını çıkarır.
use crate::flatten::{self, Path, Src};
use crate::model::{Drawing, V3};
use crate::planar::{self, P2};
use crate::solid3d;
use geo::{Area, BoundingRect, Contains, Coord, Distance, Euclidean, LineString, MultiPolygon, Point, Polygon};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Halkalar: ilki dış sınır (saat yönü tersine), diğerleri delik.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Shape {
    pub rings: Vec<Vec<P2>>,
}

impl Shape {
    pub fn from_poly(p: &Polygon<f64>) -> Shape {
        let r = |l: &LineString<f64>| {
            let mut v: Vec<P2> = l.0.iter().map(|c| [c.x, c.y]).collect();
            if v.len() > 1 && v.first() == v.last() {
                v.pop();
            }
            v
        };
        let mut rings = vec![r(p.exterior())];
        if planar::ring_area(&rings[0]) < 0.0 {
            rings[0].reverse();
        }
        for h in p.interiors() {
            let mut v = r(h);
            if planar::ring_area(&v) > 0.0 {
                v.reverse();
            }
            rings.push(v);
        }
        Shape { rings }
    }
    pub fn poly(&self) -> Polygon<f64> {
        planar::to_poly(&self.rings[0], &self.rings[1..])
    }
    pub fn bounds(&self) -> [f64; 4] {
        let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        for p in &self.rings[0] {
            b[0] = b[0].min(p[0]);
            b[1] = b[1].min(p[1]);
            b[2] = b[2].max(p[0]);
            b[3] = b[3].max(p[1]);
        }
        b
    }
    pub fn area(&self) -> f64 {
        self.poly().unsigned_area()
    }
    fn shifted(&self, dx: f64, dy: f64) -> Shape {
        Shape { rings: self.rings.iter().map(|r| r.iter().map(|p| [p[0] + dx, p[1] + dy]).collect()).collect() }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Part,
    Plate,
    Ignore,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Part {
    pub id: String,
    pub name: String,
    pub layer: String,
    /// Sol alt köşesi (0, 0) olacak biçimde kaydırılmış biçim.
    pub shape: Shape,
    /// Çizimdeki konumu (biçimin (0, 0) noktası).
    pub at: P2,
    pub area: f64,
    pub w: f64,
    pub h: f64,
    pub qty: u32,
    pub role: Role,
    pub thick: Option<f64>,
    pub notes: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AnalyzeOptions {
    /// Kullanıcının plaka olarak işaretlediği serbest parça kimlikleri.
    #[serde(default)]
    pub plates: Vec<String>,
    /// Okunmayacak katmanlar.
    #[serde(default)]
    pub ignore_layers: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Analysis {
    pub parts: Vec<Part>,
    pub layers: Vec<String>,
    pub notes: Vec<String>,
    pub units: f64,
}

pub fn is_plate_layer(l: &str) -> bool {
    let l = l.to_lowercase();
    ["plaka", "plate", "sheet", "levha", "stock", "sac"].iter().any(|k| l.contains(k))
}

fn z_span(paths: &[&Path]) -> f64 {
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for p in paths {
        for q in &p.pts {
            lo = lo.min(q[2]);
            hi = hi.max(q[2]);
        }
    }
    hi - lo
}

fn is_rect(p: &Polygon<f64>) -> bool {
    let Some(b) = p.bounding_rect() else { return false };
    p.interiors().is_empty() && (p.unsigned_area() - b.width() * b.height()).abs() < 1e-3 * b.width() * b.height()
}

/// Metinden adet okur: "4 AD", "4 ADET", "x4", "4x", "4 pcs".
pub fn qty_of(s: &str) -> Option<u32> {
    let t = s.to_lowercase().replace(['\\', ';', ':', '=', '{', '}'], " ");
    let w: Vec<&str> = t.split_whitespace().collect();
    for (i, x) in w.iter().enumerate() {
        let next = w.get(i + 1).copied().unwrap_or("");
        let prev = if i > 0 { w[i - 1] } else { "" };
        if let Ok(n) = x.parse::<u32>() {
            if ["ad", "ad.", "adet", "pcs", "pc", "x"].contains(&next) || ["adet", "x"].contains(&prev) {
                return Some(n).filter(|n| *n > 0 && *n < 100000);
            }
        }
        for (pre, suf) in [("x", ""), ("", "x"), ("", "ad"), ("", "adet")] {
            if let Some(r) = x.strip_prefix(pre).and_then(|r| r.strip_suffix(suf)) {
                if (!pre.is_empty() || !suf.is_empty()) && !r.is_empty() {
                    if let Ok(n) = r.parse::<u32>() {
                        return Some(n).filter(|n| *n > 0 && *n < 100000);
                    }
                }
            }
        }
    }
    None
}

struct Raw {
    poly: Polygon<f64>,
    name: String,
    layer: String,
    thick: Option<f64>,
    sil: Option<MultiPolygon<f64>>,
    notes: Vec<String>,
    id: String,
    qty: u32,
    role: Role,
}

fn rid(prefix: &str, p: &Polygon<f64>) -> String {
    let b = p.bounding_rect().unwrap();
    format!("{prefix}{:.0}_{:.0}_{:.0}_{:.0}", b.min().x, b.min().y, b.max().x, b.max().y)
}

fn split_names(name: &str, n: usize, i: usize) -> String {
    if n == 1 { name.to_string() } else { format!("{name}_{}", (b'a' + (i % 26) as u8) as char) }
}

/// Bağlı bileşenler (uç noktası 0,5 mm'den yakın çizgiler); kutusu başka bir
/// bileşenin kutusunun içinde kalan bileşen ona katılır (3B deliklerin çevreyle birleşmesi).
fn components(paths: &[&Path]) -> Vec<Vec<usize>> {
    let n = paths.len();
    let mut par: Vec<usize> = (0..n).collect();
    fn find(c: &mut [usize], x: usize) -> usize {
        let mut x = x;
        while c[x] != x {
            c[x] = c[c[x]];
            x = c[x];
        }
        x
    }
    let cell = 0.5;
    let mut grid: HashMap<(i64, i64, i64), Vec<usize>> = HashMap::new();
    let k = |p: &V3| ((p[0] / cell).floor() as i64, (p[1] / cell).floor() as i64, (p[2] / cell).floor() as i64);
    for (i, p) in paths.iter().enumerate() {
        for q in &p.pts {
            let (a, b, c) = k(q);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        if let Some(v) = grid.get(&(a + dx, b + dy, c + dz)) {
                            for &j in v {
                                if paths[j].pts.iter().any(|r| (r[0] - q[0]).hypot(r[1] - q[1]).hypot(r[2] - q[2]) < cell) {
                                    let (x, y) = (find(&mut par, i), find(&mut par, j));
                                    if x != y {
                                        par[x] = y;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        for q in &p.pts {
            let e = grid.entry(k(q)).or_default();
            if e.last() != Some(&i) {
                e.push(i);
            }
        }
    }
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        groups.entry(find(&mut par, i)).or_default().push(i);
    }
    let mut gs: Vec<Vec<usize>> = groups.into_values().collect();
    let bb = |g: &Vec<usize>| {
        let mut b = [f64::MAX, f64::MAX, f64::MAX, f64::MIN, f64::MIN, f64::MIN];
        for &i in g {
            for q in &paths[i].pts {
                for k in 0..3 {
                    b[k] = b[k].min(q[k]);
                    b[k + 3] = b[k + 3].max(q[k]);
                }
            }
        }
        b
    };
    gs.sort_by(|a, b| {
        let (x, y) = (bb(a), bb(b));
        let v = |b: [f64; 6]| (b[3] - b[0]) * (b[4] - b[1]) + (b[5] - b[2]);
        v(y).total_cmp(&v(x))
    });
    let boxes: Vec<[f64; 6]> = gs.iter().map(bb).collect();
    let mut owner: Vec<usize> = (0..gs.len()).collect();
    for i in 0..gs.len() {
        for j in 0..i {
            if owner[j] != j {
                continue;
            }
            let (a, b) = (boxes[i], boxes[j]);
            if (0..3).all(|k| a[k] >= b[k] - 1.0 && a[k + 3] <= b[k + 3] + 1.0) {
                owner[i] = j;
                break;
            }
        }
    }
    let mut out: Vec<Vec<usize>> = vec![vec![]; gs.len()];
    for i in 0..gs.len() {
        let o = owner[i];
        let g = gs[i].clone();
        out[o].extend(g);
    }
    out.into_iter().filter(|g| !g.is_empty()).collect()
}

fn planar_raws(paths: &[&Path], layer_ids: &mut Vec<String>) -> Vec<(Polygon<f64>, String)> {
    let mut idx = |l: &str| match layer_ids.iter().position(|x| x == l) {
        Some(i) => i,
        None => {
            layer_ids.push(l.to_string());
            layer_ids.len() - 1
        }
    };
    let ps: Vec<(Vec<P2>, usize)> = paths.iter().map(|p| (p.pts.iter().map(|q| [q[0], q[1]]).collect(), idx(&p.layer))).collect();
    planar::solids(&planar::faces(&ps)).into_iter().map(|(p, l)| (planar::tidy(&p), layer_ids[l].clone())).collect()
}

fn flat3d(paths: &[&Path], name: &str, layer: &str, prefix: &str) -> Vec<Raw> {
    let Some(f) = solid3d::flatten_part(paths) else { return vec![] };
    let polys: Vec<Polygon<f64>> = f.poly.0.into_iter().filter(|p| p.unsigned_area() > 100.0).collect();
    let n = polys.len();
    polys.into_iter().enumerate().map(|(i, p)| {
        let mut notes = vec![];
        if f.silhouette {
            notes.push("Yüz çözülemedi, dış sınır (silüet) alındı; delikleri kontrol edin.".to_string());
        }
        Raw {
            id: rid(prefix, &p),
            poly: p,
            name: split_names(name, n, i),
            layer: layer.to_string(),
            thick: Some((f.thick * 10.0).round() / 10.0),
            sil: if n == 1 { Some(f.sil.clone()) } else { None },
            notes,
            qty: 1,
            role: Role::Part,
        }
    }).collect()
}

/// Her yazıyı, onu içine alan en küçük parçaya; hiçbiri içine almıyorsa 150 mm'den
/// yakın en yakın parçaya bağlar.
fn assign_texts(src: &Src, polys: &[&Polygon<f64>]) -> Vec<Vec<String>> {
    let mut out = vec![vec![]; polys.len()];
    let shells: Vec<Polygon<f64>> = polys.iter().map(|p| Polygon::new(p.exterior().clone(), vec![])).collect();
    for (q, s) in &src.texts {
        let pt = Point::new(q[0], q[1]);
        let inside = (0..polys.len()).filter(|&i| shells[i].contains(&pt)).min_by(|&a, &b| shells[a].unsigned_area().total_cmp(&shells[b].unsigned_area()));
        let pick = inside.or_else(|| {
            (0..polys.len()).map(|i| (i, Euclidean.distance(&pt, polys[i]))).filter(|x| x.1 < 150.0).min_by(|a, b| a.1.total_cmp(&b.1)).map(|x| x.0)
        });
        if let Some(i) = pick {
            out[i].push(clean_text(s));
        }
    }
    out
}

/// MTEXT biçim kodlarını ayıklar.
fn clean_text(s: &str) -> String {
    let mut t = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\\' => match it.next() {
                Some('P') | Some('~') => t.push(' '),
                Some(k) if "fFHWQTACcp".contains(k) => {
                    for d in it.by_ref() {
                        if d == ';' {
                            break;
                        }
                    }
                }
                Some(k) => t.push(k),
                None => {}
            },
            '{' | '}' => {}
            _ => t.push(c),
        }
    }
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Adet ifadesi çıkarılmış yazı (parça adı olarak).
fn strip_qty(s: &str) -> String {
    let w: Vec<&str> = s.split_whitespace().collect();
    let mut keep = vec![];
    let mut i = 0;
    while i < w.len() {
        let pair = if i + 1 < w.len() { format!("{} {}", w[i], w[i + 1]) } else { String::new() };
        if !pair.is_empty() && qty_of(&pair).is_some() && (w[i].parse::<u32>().is_ok() || w[i + 1].parse::<u32>().is_ok()) {
            i += 2;
            continue;
        }
        if qty_of(w[i]).is_some() {
            i += 1;
            continue;
        }
        keep.push(w[i]);
        i += 1;
    }
    keep.join(" ")
}

fn loose_raws(src: &Src, opts: &AnalyzeOptions, notes: &mut Vec<String>) -> Vec<Raw> {
    let ign: HashSet<&str> = opts.ignore_layers.iter().map(|s| s.as_str()).collect();
    let all: Vec<&Path> = src.paths.iter().filter(|p| !ign.contains(p.layer.as_str())).collect();
    let (plate_paths, rest): (Vec<&Path>, Vec<&Path>) = all.into_iter().partition(|p| is_plate_layer(&p.layer));
    let mut out = vec![];
    let mut lids = vec![];
    for (p, l) in planar_raws(&plate_paths, &mut lids) {
        let mut n = vec![];
        if !is_rect(&p) {
            n.push("Plaka dikdörtgen değil; dış çerçevesi kullanılır.".into());
        }
        out.push(Raw { id: rid("L", &p), poly: p, name: "Plaka".into(), layer: l, thick: None, sil: None, notes: n, qty: 1, role: Role::Plate });
    }
    // 3B bileşenler ayrı, düzlemsel olanlar birlikte
    let comps = components(&rest);
    let mut flat: Vec<&Path> = vec![];
    let mut k3 = 0;
    for c in comps {
        let ps: Vec<&Path> = c.iter().map(|&i| rest[i]).collect();
        if z_span(&ps) <= 0.01 {
            flat.extend(ps);
        } else {
            k3 += 1;
            let layer = ps[0].layer.clone();
            out.extend(flat3d(&ps, &format!("{}_{k3}", layer), &layer, "L"));
        }
    }
    // Kullanıcının ya da çerçeve sezgisinin plaka saydığı dış sınırların çizgileri ayrılır.
    let mut marked: BTreeSet<String> = opts.plates.iter().cloned().collect();
    for pass in 0..3 {
        let raws = planar_raws(&flat, &mut lids);
        let mut auto = false;
        if pass == 0 {
            // içindeki deliklerde başka parça taşıyan dikdörtgen dış çerçeve: plaka sayılır
            for (p, _) in &raws {
                let shell = Polygon::new(p.exterior().clone(), vec![]);
                let inside = raws.iter().filter(|(q, _)| !std::ptr::eq(p, q) && shell.contains(q)).count();
                if is_rect(&shell) && inside >= 2 && !marked.contains(&rid("L", p)) && p.interiors().len() >= 2 {
                    marked.insert(rid("L", p));
                    auto = true;
                }
            }
        }
        let hit: Vec<&Polygon<f64>> = raws.iter().map(|r| &r.0).filter(|p| marked.contains(&rid("L", p))).collect();
        if hit.is_empty() || pass == 2 {
            let tx = assign_texts(src, &raws.iter().map(|r| &r.0).collect::<Vec<_>>());
            for (i, (p, l)) in raws.into_iter().enumerate() {
                let name = tx[i].iter().map(|t| strip_qty(t)).find(|t| !t.is_empty() && t.chars().count() <= 40).unwrap_or_else(|| format!("P{}", i + 1));
                let qty = tx[i].iter().find_map(|t| qty_of(t)).unwrap_or(1);
                out.push(Raw { id: rid("L", &p), poly: p, name, layer: l, thick: None, sil: None, notes: vec![], qty, role: Role::Part });
            }
            break;
        }
        let mut cut: Vec<usize> = vec![];
        for (i, path) in flat.iter().enumerate() {
            if hit.iter().any(|h| path.pts.iter().all(|q| Euclidean.distance(&Point::new(q[0], q[1]), h.exterior()) < 0.2)) {
                cut.push(i);
            }
        }
        let pl: Vec<&Path> = flat.iter().enumerate().filter(|(i, _)| cut.contains(i)).map(|x| *x.1).collect();
        flat = flat.iter().enumerate().filter(|(i, _)| !cut.contains(i)).map(|x| *x.1).collect();
        for h in &hit {
            let shell = Polygon::new(h.exterior().clone(), vec![]);
            out.push(Raw {
                id: rid("L", h),
                poly: shell,
                name: "Plaka".into(),
                layer: pl.first().map(|p| p.layer.clone()).unwrap_or_default(),
                thick: None,
                sil: None,
                notes: if auto { vec!["İçinde parça olan dikdörtgen: plaka sayıldı.".into()] } else { vec![] },
                qty: 1,
                role: Role::Plate,
            });
        }
        if auto {
            notes.push("Parçaları çevreleyen dikdörtgen plaka olarak algılandı; yanlışsa parça yapın.".into());
        }
    }
    out
}

fn insert_raws(src: &Src, idx: usize, opts: &AnalyzeOptions) -> Vec<Raw> {
    let ign: HashSet<&str> = opts.ignore_layers.iter().map(|s| s.as_str()).collect();
    let ps: Vec<&Path> = src.paths.iter().filter(|p| !ign.contains(p.layer.as_str())).collect();
    if ps.is_empty() {
        return vec![];
    }
    let prefix = format!("I{idx}_");
    let plate = is_plate_layer(&src.layer);
    let mut v = if z_span(&ps) <= 0.01 {
        let mut lids = vec![];
        let polys: Vec<Polygon<f64>> = planar_raws(&ps, &mut lids).into_iter().map(|x| x.0).filter(|p| p.unsigned_area() > 1.0).collect();
        let n = polys.len();
        polys.into_iter().enumerate().map(|(i, p)| Raw {
            id: format!("{prefix}{i}"),
            poly: p,
            name: split_names(&src.name, n, i),
            layer: src.layer.clone(),
            thick: None,
            sil: None,
            notes: vec![],
            qty: 1,
            role: Role::Part,
        }).collect()
    } else {
        let mut v = flat3d(&ps, &src.name, &src.layer, &prefix);
        for (i, r) in v.iter_mut().enumerate() {
            r.id = format!("{prefix}{i}");
        }
        v
    };
    if plate {
        for r in v.iter_mut() {
            r.role = Role::Plate;
        }
    }
    v
}

pub fn analyze(d: &Drawing, opts: &AnalyzeOptions) -> Analysis {
    let (loose, inserts) = flatten::flatten(d);
    let mut notes = vec![];
    let mut raws = loose_raws(&loose, opts, &mut notes);
    for (i, s) in inserts.iter().enumerate() {
        let mut r = insert_raws(s, i, opts);
        if opts.plates.iter().any(|p| r.iter().any(|x| &x.id == p)) {
            for x in r.iter_mut() {
                x.role = Role::Plate;
            }
        }
        raws.extend(r);
    }
    // 3B parçalarda baskın kalınlık; ondan belirgin kalın olan parça büyük olasılıkla
    // eğik çizilmiştir, dış sınırı alınır.
    let mut hist: HashMap<i64, usize> = HashMap::new();
    for r in &raws {
        if let Some(t) = r.thick {
            *hist.entry((t * 2.0).round() as i64).or_default() += 1;
        }
    }
    if let Some((&mode, _)) = hist.iter().max_by_key(|x| (x.1, -x.0)) {
        let m = mode as f64 / 2.0;
        for r in raws.iter_mut() {
            let Some(t) = r.thick else { continue };
            if (t - m).abs() > 0.5 && t > 0.5 {
                r.notes.push(format!("Kalınlık {t:.1} mm, çoğunluk {m:.1} mm: parça eğik ya da farklı kalınlıkta olabilir."));
                if t > m + 0.5 {
                    if let Some(s) = r.sil.take() {
                        if let Some(p) = s.0.into_iter().max_by(|a, b| a.unsigned_area().total_cmp(&b.unsigned_area())) {
                            r.poly = p;
                            r.notes.push("Dış sınırı alındı.".into());
                        }
                    }
                }
            }
        }
    }
    // Her parçadan en az iki kat büyük dikdörtgenler plaka sayılır (plakalar çizimde
    // parçaların yanına çizilmişse).
    let rect_area = |r: &Raw| if r.role == Role::Part && is_rect(&r.poly) { r.poly.unsigned_area() } else { 0.0 };
    let big = raws.iter().map(rect_area).fold(0.0, f64::max);
    if big > 0.0 {
        let others = raws.iter().filter(|r| r.role == Role::Part && rect_area(r) < big * 0.5).map(|r| r.poly.unsigned_area()).fold(0.0, f64::max);
        if big >= 2.0 * others && raws.iter().filter(|r| r.role == Role::Part).count() > 1 {
            let mut n = 0;
            for r in raws.iter_mut() {
                if rect_area(r) >= big * 0.5 && rect_area(r) >= 2.0 * others {
                    r.role = Role::Plate;
                    r.name = "Plaka".into();
                    n += 1;
                }
            }
            notes.push(format!("{n} büyük dikdörtgen plaka olarak algılandı; yanlışsa parça yapın."));
        }
    }
    let mut layers: BTreeSet<String> = loose.paths.iter().map(|p| p.layer.clone()).collect();
    for s in &inserts {
        layers.extend(s.paths.iter().map(|p| p.layer.clone()));
    }
    let parts = raws.into_iter().map(|r| {
        let b = r.poly.bounding_rect().unwrap();
        let shape = Shape::from_poly(&r.poly).shifted(-b.min().x, -b.min().y);
        let w = b.width();
        let h = b.height();
        let area = r.poly.unsigned_area();
        Part { id: r.id, name: r.name, layer: r.layer, shape, at: [b.min().x, b.min().y], area, w, h, qty: r.qty, role: r.role, thick: r.thick, notes: r.notes }
    }).collect();
    let _ = Coord { x: 0.0, y: 0.0 };
    Analysis { parts, layers: layers.into_iter().collect(), notes, units: d.units }
}

/// Analizden dizme girdisi: plakalar ölçüye göre gruplanır, adetleri çizimdeki sayıdır.
pub fn to_nest(a: &Analysis) -> (Vec<crate::nest::NestPart>, Vec<crate::nest::PlateSpec>) {
    let parts = a.parts.iter().filter(|p| p.role == Role::Part && p.qty > 0).map(|p| crate::nest::NestPart { id: p.id.clone(), name: p.name.clone(), shape: p.shape.clone(), qty: p.qty }).collect();
    let mut plates: Vec<crate::nest::PlateSpec> = vec![];
    for p in a.parts.iter().filter(|p| p.role == Role::Plate) {
        let (w, h) = ((p.w * 10.0).round() / 10.0, (p.h * 10.0).round() / 10.0);
        let (w, h) = (w.max(h), w.min(h));
        match plates.iter_mut().find(|x| (x.w - w).abs() < 0.5 && (x.h - h).abs() < 0.5) {
            Some(x) => x.count = Some(x.count.unwrap_or(0) + 1),
            None => plates.push(crate::nest::PlateSpec { name: format!("{w:.0} × {h:.0}"), w, h, count: Some(1) }),
        }
    }
    plates.sort_by(|a, b| (b.w * b.h).total_cmp(&(a.w * a.h)));
    (parts, plates)
}
