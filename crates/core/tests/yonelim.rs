//! Yönelim (aynalama) denetimi: çıktıdaki her parça, girdideki parçanın yalnız
//! döndürülmüş ve kaydırılmış hâli olmalı. Kaplamalı ya da desenli plakada aynalanmış
//! parça hurdadır.
//!
//! Karşılaştırma köşe eşlemesi gerektirmez: iki biçim ağırlık merkezlerinde çakıştırılır,
//! en iyi döndürme açısında simetrik farkın alanı ölçülür. Döndürmeyle eşleşip aynayla
//! eşleşmemesi (kiral parçada) beklenir.
use geo::{Area, BooleanOps, Centroid, Coord, LineString, MapCoords, Polygon, Rotate, Translate};
use nestio_core::model::{Block, Drawing, Ent, Geom, V3};
use nestio_core::nest::{self, Event, NestInput, NestPart, PlateSpec, Progress, Solver};
use nestio_core::parts::{self, AnalyzeOptions, Role};
use nestio_core::strip::{self, StripJob, StripResult};
use nestio_core::{dxf_in, dxf_out};
use std::collections::HashMap;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

type P2 = [f64; 2];

struct Threads;
impl Solver for Threads {
    fn width(&self) -> usize {
        2
    }
    fn solve_many(&self, jobs: Vec<StripJob>) -> impl Future<Output = Vec<StripResult>> {
        let out = std::thread::scope(|s| {
            let hs: Vec<_> = jobs.iter().map(|j| s.spawn(move || strip::solve(j))).collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        std::future::ready(out)
    }
}

struct Quiet;
impl Progress for Quiet {
    fn report(&self, _: Event) {}
}

fn block_on<F: Future>(f: F) -> F::Output {
    let mut f = pin!(f);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

// ── biçim karşılaştırma ─────────────────────────────────────────────────────

fn ring(r: &[P2]) -> LineString<f64> {
    LineString::from(r.iter().map(|p| Coord { x: p[0], y: p[1] }).collect::<Vec<_>>())
}

fn poly(rings: &[Vec<P2>]) -> Polygon<f64> {
    Polygon::new(ring(&rings[0]), rings[1..].iter().map(|r| ring(r)).collect())
}

fn centered(p: &Polygon<f64>) -> Polygon<f64> {
    let c = p.centroid().unwrap();
    p.translate(-c.x(), -c.y())
}

/// `a`yı (istenirse önce aynalayıp) döndürerek `b`ye en iyi oturtur; kalan simetrik
/// farkın `b` alanına oranını döndürür. 0 ≈ aynı biçim.
fn fit(a: &Polygon<f64>, b: &Polygon<f64>, mirror: bool) -> f64 {
    let a = if mirror { a.map_coords(|c| Coord { x: -c.x, y: c.y }) } else { a.clone() };
    let (a, b) = (centered(&a), centered(b));
    let area = b.unsigned_area();
    let err = |deg: f64| a.rotate_around_point(deg, geo::Point::new(0.0, 0.0)).xor(&b).unsigned_area() / area;
    let mut best = (0..360).map(|d| d as f64).map(|d| (err(d), d)).min_by(|x, y| x.0.total_cmp(&y.0)).unwrap();
    for step in [0.1, 0.01, 0.001] {
        let c = best.1;
        for k in -10..=10 {
            let d = c + k as f64 * step;
            let e = err(d);
            if e < best.0 {
                best = (e, d);
            }
        }
    }
    best.0
}

/// Çıktı biçimi girdinin döndürülmüşü mü; kiral parçada aynası olmamalı.
fn assert_rigid(what: &str, input: &Polygon<f64>, output: &Polygon<f64>) {
    let proper = fit(input, output, false);
    assert!(proper < 2e-3, "{what}: döndürmeyle eşleşmiyor (fark {proper:.4})");
    if fit(input, input, true) > 0.02 {
        let mirrored = fit(input, output, true);
        assert!(mirrored > 0.02, "{what}: AYNALANMIŞ (aynayla fark {mirrored:.4}, döndürmeyle {proper:.4})");
    }
}

// ── çizim kurma ─────────────────────────────────────────────────────────────

/// Kiral L braket: 300 × 250, kol 60; iki kolun boyu farklı, aynası kendisinden ayırt edilir.
fn l_braket() -> Vec<P2> {
    vec![[0.0, 0.0], [300.0, 0.0], [300.0, 60.0], [60.0, 60.0], [60.0, 250.0], [0.0, 250.0]]
}

/// Büyük kiral parça: 2400 × 1200 levha, bir köşesi pahlı, bir kenarında cep.
fn buyuk() -> Vec<P2> {
    vec![[0.0, 0.0], [2400.0, 0.0], [2400.0, 900.0], [2100.0, 1200.0], [700.0, 1200.0], [700.0, 1000.0], [400.0, 1000.0], [400.0, 1200.0], [0.0, 1200.0]]
}

fn ucgen() -> Vec<P2> {
    vec![[0.0, 0.0], [220.0, 0.0], [0.0, 180.0]]
}

fn ent(g: Geom) -> Ent {
    Ent { layer: "PARCA".into(), g }
}

fn lw(pts: &[P2], n: V3) -> Ent {
    ent(Geom::Poly { v: pts.iter().map(|p| [p[0], p[1], 0.0, 0.0]).collect(), closed: true, n })
}

fn drawing(entities: Vec<Ent>, blocks: Vec<(&str, Vec<Ent>)>) -> Drawing {
    Drawing {
        units: 1.0,
        entities,
        blocks: blocks.into_iter().map(|(n, e)| (n.to_string(), Block { base: [0.0; 3], entities: e })).collect::<HashMap<_, _>>(),
    }
}

fn insert(name: &str, p: V3, s: V3, rot: f64, n: V3) -> Ent {
    ent(Geom::Insert { name: name.into(), p, s, rot, n, cols: 1, rows: 1, dc: 0.0, dr: 0.0 })
}

fn only_part(d: &Drawing) -> parts::Part {
    let a = parts::analyze(d, &AnalyzeOptions::default());
    let ps: Vec<_> = a.parts.into_iter().filter(|p| p.role == Role::Part).collect();
    assert_eq!(ps.len(), 1, "tek parça bekleniyordu");
    ps.into_iter().next().unwrap()
}

/// Dizer, DXF yazar, DXF'i geri okur; her yerleşimin hem doğrulayıcıdaki hem DXF'teki
/// biçimini girdiyle karşılaştırır.
fn nest_and_compare(what: &str, parts: Vec<NestPart>, plate: (f64, f64)) {
    let input = NestInput {
        parts,
        plates: vec![PlateSpec { name: "P".into(), w: plate.0, h: plate.1, count: None }],
        gap: 6.0,
        margin: 5.0,
        secs: 2.0,
        fill_holes: true,
        improve: true,
        seed: 7,
    };
    let out = block_on(nest::nest(&input, &Threads, &Quiet));
    assert!(out.check.ok, "{what}: denetim: {:?}", out.check.errors);
    assert!(out.unplaced.is_empty(), "{what}: yerleşmeyen var");
    let dxf = dxf_out::write(&input.parts, &out.plates, input.gap).unwrap();
    let back = dxf_in::read(&dxf).unwrap();
    let mut cut = back.entities.iter().filter(|e| e.layer == "KESIM").map(|e| match &e.g {
        Geom::Poly { v, .. } => v.iter().map(|q| [q[0], q[1]]).collect::<Vec<P2>>(),
        _ => panic!("KESIM'de çoklu çizgi dışı varlık"),
    });
    let mut cache: HashMap<usize, Polygon<f64>> = HashMap::new();
    for (k, pl) in out.plates.iter().enumerate() {
        for it in &pl.items {
            let part = &input.parts[it.part];
            let src = cache.entry(it.part).or_insert_with(|| part.shape.poly()).clone();
            let tag = format!("{what} / plaka {} / {} (dönüş {:.1}°)", k + 1, part.name, it.rot);
            assert_rigid(&format!("{tag} / yerleşim"), &src, &nestio_core::verify::placed_poly(&input.parts, it));
            assert!(!nestio_core::verify::mirrored(&input.parts, it), "{tag}: doğrulayıcı ayna buldu");
            // önizleme (web/js/app.js placedRings) aynı formülü kullanır: x·c − y·s, x·s + y·c
            let (s, c) = it.rot.to_radians().sin_cos();
            let pre: Vec<Vec<P2>> = part.shape.rings.iter().map(|r| r.iter().map(|p| [p[0] * c - p[1] * s + it.x, p[0] * s + p[1] * c + it.y]).collect()).collect();
            assert_rigid(&format!("{tag} / önizleme"), &src, &poly(&pre));
            let rings: Vec<Vec<P2>> = (0..part.shape.rings.len()).map(|_| cut.next().expect("DXF'te halka eksik")).collect();
            assert_rigid(&format!("{tag} / DXF"), &src, &poly(&rings));
        }
    }
    assert!(cut.next().is_none(), "{what}: DXF'te fazla kesim halkası");
}

// ── örnek çizim ─────────────────────────────────────────────────────────────

#[test]
fn ornek_cizimde_parcalar_aynalanmaz() {
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ornek-flans-braket.dxf")).unwrap();
    let a = parts::analyze(&dxf_in::read(&bytes).unwrap(), &AnalyzeOptions::default());
    // girdi: uret.py'deki kiral biçimler okunduğu gibi
    let by_area = |area: f64| a.parts.iter().filter(|p| p.role == Role::Part).min_by(|x, y| (x.shape.poly().unsigned_area() - area).abs().total_cmp(&(y.shape.poly().unsigned_area() - area).abs())).unwrap();
    let tri = poly(&[ucgen()]);
    assert_rigid("örnek / üçgen okuma", &tri, &by_area(tri.unsigned_area()).shape.poly());
    let l = poly(&[l_braket()]);
    let lp = by_area(l.unsigned_area() - 2.0 * std::f64::consts::PI * 81.0);
    assert_rigid("örnek / L braket okuma", &l, &Polygon::new(lp.shape.poly().exterior().clone(), vec![]));
    let (parts, plates) = parts::to_nest(&a);
    let input = NestInput { parts, plates, gap: 10.0, margin: 10.0, secs: 3.0, fill_holes: true, improve: true, seed: 1 };
    let out = block_on(nest::nest(&input, &Threads, &Quiet));
    assert!(out.in_holes > 0);
    assert!(out.check.ok, "{:?}", out.check.errors);
    nest_and_compare("örnek", input.parts.clone(), (3000.0, 1500.0));
}

// ── 2B: ters OCS ve negatif ölçekli blok ─────────────────────────────────────

/// Beklenen biçim çizimin üstten (dünya XY) görünüşüdür; CAD'de ekranda görülen budur.
#[test]
fn iki_boyutlu_ocs_ve_negatif_olcek() {
    let neg = |v: &[P2]| v.iter().map(|p| [-p[0], p[1]]).collect::<Vec<P2>>();
    let cases: Vec<(&str, Drawing, Vec<P2>)> = vec![
        ("düz", drawing(vec![lw(&l_braket(), [0.0, 0.0, 1.0])], vec![]), l_braket()),
        // OCS normali (0,0,-1): dünyada x ters döner — ekranda aynalı görünür, öyle okunmalı
        ("OCS -Z", drawing(vec![lw(&l_braket(), [0.0, 0.0, -1.0])], vec![]), neg(&l_braket())),
        ("blok", drawing(vec![insert("B", [500.0, 0.0, 0.0], [1.0, 1.0, 1.0], 30.0, [0.0, 0.0, 1.0])], vec![("B", vec![lw(&l_braket(), [0.0, 0.0, 1.0])])]), l_braket()),
        ("blok x ölçeği -1", drawing(vec![insert("B", [500.0, 0.0, 0.0], [-1.0, 1.0, 1.0], 0.0, [0.0, 0.0, 1.0])], vec![("B", vec![lw(&l_braket(), [0.0, 0.0, 1.0])])]), neg(&l_braket())),
        ("blok y ölçeği -2", drawing(vec![insert("B", [0.0, 0.0, 0.0], [2.0, -2.0, 1.0], 15.0, [0.0, 0.0, 1.0])], vec![("B", vec![lw(&ucgen(), [0.0, 0.0, 1.0])])]), ucgen().iter().map(|p| [2.0 * p[0], -2.0 * p[1]]).collect()),
        ("blok OCS -Z", drawing(vec![insert("B", [0.0, 0.0, 0.0], [1.0, 1.0, 1.0], 0.0, [0.0, 0.0, -1.0])], vec![("B", vec![lw(&l_braket(), [0.0, 0.0, 1.0])])]), neg(&l_braket())),
        // iki ayna birbirini götürür
        ("blok OCS -Z ve x ölçeği -1", drawing(vec![insert("B", [0.0, 0.0, 0.0], [-1.0, 1.0, 1.0], 0.0, [0.0, 0.0, -1.0])], vec![("B", vec![lw(&l_braket(), [0.0, 0.0, 1.0])])]), l_braket()),
        ("büyük parça", drawing(vec![lw(&buyuk(), [0.0, 0.0, 1.0])], vec![]), buyuk()),
        ("büyük parça OCS -Z", drawing(vec![lw(&buyuk(), [0.0, 0.0, -1.0])], vec![]), neg(&buyuk())),
    ];
    for (what, d, expect) in cases {
        let p = only_part(&d);
        assert_rigid(&format!("2B {what} / okuma"), &poly(&[expect]), &p.shape.poly());
        let qty = if p.area > 1e6 { 1 } else { 3 };
        nest_and_compare(&format!("2B {what}"), vec![NestPart { id: p.id, name: p.name, shape: p.shape, qty }], (3000.0, 1500.0));
    }
}

// ── 3B: kalınlıklı panel ─────────────────────────────────────────────────────

fn rot(axis: usize, deg: f64) -> [[f64; 3]; 3] {
    let (s, c) = deg.to_radians().sin_cos();
    match axis {
        0 => [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]],
        1 => [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]],
        _ => [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]],
    }
}

fn mul(m: &[[f64; 3]; 3], p: V3) -> V3 {
    [0, 1, 2].map(|i| m[i][0] * p[0] + m[i][1] * p[1] + m[i][2] * p[2])
}

/// Panelin üst ve alt konturu ile dik kenarları, dünyaya `m` ile döndürülüp `t` kadar
/// kaydırılmış çizgiler olarak. `reverse` çizgi yönlerini ve sırasını çevirir (aynı geometri).
fn panel(outline: &[P2], thick: f64, m: &[[f64; 3]; 3], t: V3, reverse: bool) -> Vec<Ent> {
    let w = |p: V3| {
        let q = mul(m, p);
        [q[0] + t[0], q[1] + t[1], q[2] + t[2]]
    };
    let mut segs: Vec<(V3, V3)> = vec![];
    let n = outline.len();
    for z in [0.0, thick] {
        for i in 0..n {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            segs.push((w([a[0], a[1], z]), w([b[0], b[1], z])));
        }
    }
    for p in outline {
        segs.push((w([p[0], p[1], 0.0]), w([p[0], p[1], thick])));
    }
    if reverse {
        segs.reverse();
        for s in segs.iter_mut() {
            *s = (s.1, s.0);
        }
    }
    segs.into_iter().map(|(a, b)| ent(Geom::Line { a, b })).collect()
}

/// Panelin beklenen biçimi: üstten (+Z'den) bakıldığı gibi; tam dik panelde ön (−Y'den),
/// o da değilse sağ (+X'ten) görünüş. Panelin kendi +Z'si bakana dönükse çizildiği gibi,
/// değilse aynası (levha takla atmaz, kullanıcının gördüğü yüz okunur).
fn standard_view(outline: &[P2], m: &[[f64; 3]; 3]) -> Vec<P2> {
    let nrm = mul(m, [0.0, 0.0, 1.0]);
    let k = if nrm[2].abs() > 1e-3 { 2 } else if nrm[1].abs() > 1e-3 { 1 } else { 0 };
    let toward = [1.0, -1.0, 1.0][k] * nrm[k] > 0.0;
    outline.iter().map(|p| if toward { *p } else { [-p[0], p[1]] }).collect()
}

#[test]
fn uc_boyutlu_panel_ustten_gorunus_gibi_okunur() {
    let ident = rot(2, 0.0);
    let comp = |a: [[f64; 3]; 3], b: [[f64; 3]; 3]| [0, 1, 2].map(|i| [0, 1, 2].map(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum::<f64>()));
    let cases: Vec<(&str, [[f64; 3]; 3])> = vec![
        ("yatay", ident),
        ("yatay, z'de 37°", rot(2, 37.0)),
        ("ters çevrilmiş (alttan)", rot(0, 180.0)),
        ("ters çevrilmiş, y ekseninde", rot(1, 180.0)),
        ("ters, z'de 110°", comp(rot(2, 110.0), rot(0, 180.0))),
        ("dik, ön görünüş", rot(0, 90.0)),
        ("dik, ön görünüş ters", rot(0, -90.0)),
        ("dik, sağ görünüş", comp(rot(2, 90.0), rot(0, 90.0))),
        ("dik, sağ görünüş ters", comp(rot(2, -90.0), rot(0, 90.0))),
        ("hafif eğik", comp(rot(1, 8.0), rot(0, 5.0))),
        ("hafif eğik ters", comp(rot(1, 8.0), rot(0, 185.0))),
        // normalin z bileşeni küçük ama var: yine üstten okunur
        ("çok eğik (70°)", rot(0, 70.0)),
        ("çok eğik ters (110°)", rot(0, 110.0)),
        ("çok eğik, z'de 60°", comp(rot(2, 60.0), rot(0, -75.0))),
    ];
    for (what, m) in cases {
        for (shape_name, outline) in [("L", l_braket()), ("üçgen", ucgen()), ("büyük", buyuk())] {
            let expect = poly(&[standard_view(&outline, &m)]);
            for reverse in [false, true] {
                let d = drawing(panel(&outline, 18.0, &m, [1000.0, -400.0, 250.0], reverse), vec![]);
                let p = only_part(&d);
                let tag = format!("3B {what} / {shape_name}{}", if reverse { " / çizgiler ters" } else { "" });
                assert!((p.thick.unwrap() - 18.0).abs() < 0.2, "{tag}: kalınlık {:?}", p.thick);
                assert_rigid(&format!("{tag} / okuma"), &expect, &p.shape.poly());
                // blok içinde de aynı
                let b = drawing(vec![insert("PANEL", [0.0; 3], [1.0, 1.0, 1.0], 0.0, [0.0, 0.0, 1.0])], vec![("PANEL", panel(&outline, 18.0, &m, [10.0, 20.0, 30.0], reverse))]);
                assert_rigid(&format!("{tag} / blok okuma"), &expect, &only_part(&b).shape.poly());
            }
        }
    }
    // dizme + DXF: alttan okunan paneller, küçükler büyüğün cebine/yanına girer
    let big = only_part(&drawing(panel(&buyuk(), 8.0, &rot(0, 180.0), [0.0; 3], true), vec![]));
    let small = only_part(&drawing(panel(&l_braket(), 8.0, &comp(rot(2, 33.0), rot(0, 180.0)), [0.0; 3], false), vec![]));
    let tri = only_part(&drawing(panel(&ucgen(), 8.0, &rot(1, 180.0), [0.0; 3], true), vec![]));
    nest_and_compare(
        "3B ters paneller",
        vec![
            NestPart { id: big.id, name: "BUYUK".into(), shape: big.shape, qty: 2 },
            NestPart { id: small.id, name: "L".into(), shape: small.shape, qty: 4 },
            NestPart { id: tri.id, name: "UCGEN".into(), shape: tri.shape, qty: 6 },
        ],
        (3000.0, 1500.0),
    );
}
