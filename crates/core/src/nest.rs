//! Plaka döngüsü: kalan parçalarla şerit doldurulur, plakaya sığan kısım kesilir;
//! en sonda son plakadaki parçalar önceki plakalara taşınmaya çalışılır ve son
//! plaka sıkıştırılır, böylece boş alan tek parça artık olarak sağda kalır.
use crate::holes::{self, Kit};
use crate::parts::Shape;
use crate::strip::{JobItem, StripJob, StripResult};
use crate::verify::{self, Check};
use geo::{Area, Buffer, Coord, LineString, Polygon, Simplify};
use geo::algorithm::buffer::{BufferStyle, LineJoin};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NestPart {
    pub id: String,
    pub name: String,
    pub shape: Shape,
    pub qty: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlateSpec {
    #[serde(default)]
    pub name: String,
    pub w: f64,
    pub h: f64,
    /// Elde kaç tane var; boşsa sınırsız.
    #[serde(default)]
    pub count: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NestInput {
    pub parts: Vec<NestPart>,
    pub plates: Vec<PlateSpec>,
    pub gap: f64,
    pub margin: f64,
    /// Plaka başına arama süresi (sn).
    pub secs: f64,
    #[serde(default = "yes")]
    pub fill_holes: bool,
    #[serde(default = "yes")]
    pub improve: bool,
    #[serde(default)]
    pub seed: u64,
}

fn yes() -> bool {
    true
}

/// Parçanın yerleşimi: biçim (0, 0) etrafında `rot` derece döndürülür, (x, y) kadar kaydırılır.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Placed {
    pub part: usize,
    pub copy: u32,
    pub rot: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlateOut {
    pub spec: usize,
    pub w: f64,
    pub h: f64,
    pub items: Vec<Placed>,
    /// Soldan itibaren kullanılan boy (mm); sağındaki dikdörtgen artıktır.
    pub used: f64,
    pub fill: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NestOutput {
    pub plates: Vec<PlateOut>,
    /// (parça, kopya, neden)
    pub unplaced: Vec<(usize, u32, String)>,
    pub check: Check,
    pub kits: usize,
    pub in_holes: usize,
}

#[derive(Serialize, Clone, Debug)]
pub struct Event {
    pub stage: String,
    pub msg: String,
    pub frac: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plates: Option<Vec<PlateOut>>,
}

pub trait Solver {
    /// Aynı anda çalışabilecek iş sayısı.
    fn width(&self) -> usize;
    fn solve_many(&self, jobs: Vec<StripJob>) -> impl Future<Output = Vec<StripResult>>;
}

pub trait Progress {
    fn report(&self, ev: Event);
    fn cancelled(&self) -> bool {
        false
    }
}

/// Bir kitin şerit çözücüye giden dış biçimi: 0,25 mm şişirilip 0,2 mm sadeleştirilir
/// (sadeleştirme gerçek çizginin içine girmez).
pub fn item_shape(s: &Shape) -> Vec<(f32, f32)> {
    let ext = Polygon::new(LineString::from(s.rings[0].iter().map(|p| Coord { x: p[0], y: p[1] }).collect::<Vec<_>>()), vec![]);
    let b = ext.buffer_with_style(BufferStyle::new(0.25).line_join(LineJoin::Miter(2.0)));
    let p = b.0.into_iter().max_by(|a, b| a.unsigned_area().total_cmp(&b.unsigned_area())).unwrap_or(ext);
    let p = p.simplify(0.2);
    let mut v: Vec<(f32, f32)> = p.exterior().0.iter().map(|c| (c.x as f32, c.y as f32)).collect();
    if v.len() > 1 && v.first() == v.last() {
        v.pop();
    }
    v
}

struct Geo {
    /// Şerit yüksekliği, şeritte kullanılabilir boy, şerit → plaka kaydırması
    hs: f32,
    ls: f64,
    off: f64,
    l: f64,
    s: f64,
}

fn geo_of(p: &PlateSpec, gap: f64, margin: f64) -> Geo {
    let (l, s) = if p.w >= p.h { (p.w, p.h) } else { (p.h, p.w) };
    Geo { hs: (s - 2.0 * margin + 2.0 * gap) as f32, ls: l - 2.0 * margin + gap, off: margin - gap, l, s }
}

fn rot_pt(p: (f32, f32), deg: f32) -> (f64, f64) {
    let (s, c) = (deg as f64).to_radians().sin_cos();
    (p.0 as f64 * c - p.1 as f64 * s, p.0 as f64 * s + p.1 as f64 * c)
}

struct Ctx<'a> {
    kits: &'a [Kit],
    shapes: Vec<Vec<(f32, f32)>>,
    /// aynı biçimli kitler tek öğe (talep > 1) olarak gönderilir
    class: Vec<usize>,
    area: Vec<f64>,
    gap: f64,
    margin: f64,
    seed: u64,
}

/// Plakadaki bir kit yerleşimi (şerit koordinatında).
#[derive(Clone, Debug)]
struct KP {
    kit: usize,
    rot: f32,
    tx: f32,
    ty: f32,
}

impl Ctx<'_> {
    fn next_seed(&mut self) -> u64 {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.seed >> 11
    }
    fn job(&mut self, kits: &[usize], g: &Geo, secs: f64) -> StripJob {
        let mut by: HashMap<usize, u64> = HashMap::new();
        for &k in kits {
            *by.entry(self.class[k]).or_default() += 1;
        }
        let mut items: Vec<JobItem> = by.into_iter().map(|(c, d)| JobItem { id: c as u64, demand: d, shape: self.shapes[c].clone() }).collect();
        items.sort_by_key(|i| i.id);
        StripJob { items, height: g.hs, gap: self.gap as f32, secs, seed: self.next_seed() }
    }
    /// Çözümü kit yerleşimine çevirir; aynı sınıftaki kopyalar sırayla atanır.
    fn assign(&self, kits: &[usize], r: &StripResult) -> Vec<KP> {
        let mut pool: HashMap<usize, Vec<usize>> = HashMap::new();
        for &k in kits {
            pool.entry(self.class[k]).or_default().push(k);
        }
        r.placed.iter().filter_map(|&(id, rot, tx, ty)| {
            let k = pool.get_mut(&(id as usize))?.pop()?;
            Some(KP { kit: k, rot, tx, ty })
        }).collect()
    }
    fn max_x(&self, kp: &KP) -> f64 {
        self.shapes[self.class[kp.kit]].iter().map(|&p| rot_pt(p, kp.rot).0 + kp.tx as f64).fold(f64::MIN, f64::max)
    }
    fn fits(&self, kps: &[KP], g: &Geo) -> Vec<KP> {
        kps.iter().filter(|kp| self.max_x(kp) <= g.ls + 1e-6).cloned().collect()
    }
    fn used(&self, kps: &[KP]) -> f64 {
        kps.iter().map(|k| self.max_x(k)).fold(0.0, f64::max)
    }
    fn area_of(&self, kps: &[KP]) -> f64 {
        kps.iter().map(|k| self.area[k.kit]).sum()
    }
}

struct Plate {
    spec: usize,
    kps: Vec<KP>,
}

fn emit(ctx: &Ctx, specs: &[PlateSpec], plates: &[Plate]) -> Vec<PlateOut> {
    plates.iter().map(|pl| {
        let g = geo_of(&specs[pl.spec], ctx.gap, ctx.margin);
        let mut items = vec![];
        let mut a = 0.0;
        for kp in &pl.kps {
            let kit = &ctx.kits[kp.kit];
            a += ctx.area[kp.kit];
            for m in &kit.members {
                // üye: önce kendi dönüşümü (ev sahibine göre), sonra kitin dönüşümü
                let (dx, dy) = rot_pt((m.dx as f32, m.dy as f32), kp.rot);
                items.push(Placed { part: m.part, copy: m.copy, rot: kp.rot as f64 + m.rot, x: dx + kp.tx as f64 + g.off, y: dy + kp.ty as f64 + g.off });
            }
        }
        let used = if pl.kps.is_empty() { 0.0 } else { ctx.used(&pl.kps) + g.off };
        PlateOut { spec: pl.spec, w: g.l, h: g.s, items, used: used.min(g.l), fill: a / (g.l * g.s) }
    }).collect()
}

pub async fn nest<S: Solver, P: Progress>(input: &NestInput, solver: &S, prog: &P) -> NestOutput {
    let gap = input.gap.max(0.0);
    let margin = input.margin.max(0.0);
    let specs = &input.plates;
    // 1. delik doldurma → kitler
    prog.report(Event { stage: "holes".into(), msg: "Delikler taranıyor".into(), frac: 0.0, plates: None });
    let (kits, in_holes) = holes::build(&input.parts, gap, input.fill_holes, &|m: &str| {
        prog.report(Event { stage: "holes".into(), msg: m.into(), frac: 0.0, plates: None })
    });
    // sınıflar: aynı parçanın konuksuz kopyaları tek sınıf
    let mut class = vec![0; kits.len()];
    let mut shapes: Vec<Vec<(f32, f32)>> = vec![];
    let mut seen: HashMap<usize, usize> = HashMap::new();
    for (i, k) in kits.iter().enumerate() {
        let host = k.members[0].part;
        if k.members.len() == 1 {
            if let Some(&c) = seen.get(&host) {
                class[i] = c;
                continue;
            }
            seen.insert(host, shapes.len());
        }
        class[i] = shapes.len();
        shapes.push(item_shape(&input.parts[host].shape));
    }
    let area: Vec<f64> = kits.iter().map(|k| k.members.iter().map(|m| input.parts[m.part].shape.area()).sum()).collect();
    let mut ctx = Ctx { kits: &kits, shapes, class, area, gap, margin, seed: input.seed.wrapping_add(0x9e3779b97f4a7c15) };
    let mut unplaced: Vec<(usize, u32, String)> = vec![];
    let mut stock: Vec<Option<u32>> = specs.iter().map(|p| p.count).collect();
    let geos: Vec<Geo> = specs.iter().map(|p| geo_of(p, gap, margin)).collect();
    // hiçbir plakaya sığmayan parçalar
    let mut remaining: Vec<usize> = vec![];
    for (i, k) in kits.iter().enumerate() {
        let sh = &ctx.shapes[ctx.class[i]];
        let ok = geos.iter().any(|g| fits_somehow(sh, g, gap));
        if ok {
            remaining.push(i);
        } else {
            for m in &k.members {
                unplaced.push((m.part, m.copy, "Hiçbir plakaya sığmıyor".into()));
            }
        }
    }
    let total_area: f64 = remaining.iter().map(|&k| ctx.area[k]).sum();
    let p = solver.width().max(1);
    let secs = input.secs.max(1.0);
    let mut plates: Vec<Plate> = vec![];
    let mut done_area = 0.0;
    while !remaining.is_empty() {
        if prog.cancelled() {
            break;
        }
        let avail: Vec<usize> = (0..specs.len()).filter(|&i| stock[i].is_none_or(|c| c > 0)).collect();
        if avail.is_empty() {
            break;
        }
        let rem_area: f64 = remaining.iter().map(|&k| ctx.area[k]).sum();
        let frac = done_area / total_area.max(1.0);
        // kalanın hepsi tek plakaya sığabilir mi? en küçük uygun plakadan başla
        let mut fin: Vec<usize> = avail.iter().copied().filter(|&i| geos[i].ls * geos[i].hs as f64 * 0.98 >= rem_area && remaining.iter().all(|&k| fits_somehow(&ctx.shapes[ctx.class[k]], &geos[i], gap))).collect();
        fin.sort_by(|&a, &b| (geos[a].l * geos[a].s).total_cmp(&(geos[b].l * geos[b].s)));
        let mut chosen: Option<(usize, Vec<KP>)> = None;
        for &t in fin.iter().take(2) {
            prog.report(Event { stage: "plate".into(), msg: format!("Plaka {}: kalan {} parça tek plakaya deneniyor", plates.len() + 1, remaining.len()), frac, plates: None });
            let jobs: Vec<StripJob> = (0..p).map(|_| ctx.job(&remaining, &geos[t], secs)).collect();
            let res = solver.solve_many(jobs).await;
            let best = res.iter().map(|r| ctx.assign(&remaining, r)).filter(|kps| kps.len() == remaining.len() && ctx.fits(kps, &geos[t]).len() == kps.len()).min_by(|a, b| ctx.used(a).total_cmp(&ctx.used(b)));
            if let Some(b) = best {
                chosen = Some((t, b));
                break;
            }
        }
        let (t, kps) = match chosen {
            Some(c) => c,
            None => {
                let t = avail[0];
                prog.report(Event { stage: "plate".into(), msg: format!("Plaka {}: {} parça diziliyor", plates.len() + 1, remaining.len()), frac, plates: None });
                let jobs: Vec<StripJob> = (0..p).map(|_| ctx.job(&remaining, &geos[t], secs)).collect();
                let res = solver.solve_many(jobs).await;
                if let Some(e) = res.iter().find_map(|r| r.error.clone()) {
                    prog.report(Event { stage: "warn".into(), msg: format!("Çözücü uyarısı: {e}"), frac, plates: None });
                }
                let best = res.iter().map(|r| ctx.fits(&ctx.assign(&remaining, r), &geos[t])).max_by(|a, b| ctx.area_of(a).total_cmp(&ctx.area_of(b)).then(ctx.used(b).total_cmp(&ctx.used(a)))).unwrap_or_default();
                (t, best)
            }
        };
        if kps.is_empty() {
            for &k in &remaining {
                for m in &kits[k].members {
                    unplaced.push((m.part, m.copy, "Plakaya yerleştirilemedi".into()));
                }
            }
            break;
        }
        if let Some(c) = stock[t].as_mut() {
            *c -= 1;
        }
        let ids: std::collections::HashSet<usize> = kps.iter().map(|k| k.kit).collect();
        remaining.retain(|k| !ids.contains(k));
        done_area += ctx.area_of(&kps);
        plates.push(Plate { spec: t, kps });
        prog.report(Event { stage: "plate".into(), msg: format!("Plaka {} hazır", plates.len()), frac: done_area / total_area.max(1.0), plates: Some(emit(&ctx, specs, &plates)) });
    }
    for &k in &remaining {
        for m in &kits[k].members {
            unplaced.push((m.part, m.copy, "Plaka stoğu yetmedi".into()));
        }
    }
    // 2. iyileştirme: son plakadaki parçaları önceki plakalara taşı
    if input.improve && plates.len() >= 2 && !prog.cancelled() {
        let short = (secs * 0.1).clamp(3.0, 20.0);
        let n = plates.len();
        for k in 0..n - 1 {
            let mut pending: Vec<usize> = plates[n - 1].kps.iter().map(|x| x.kit).collect();
            pending.sort_by(|&a, &b| ctx.area[b].total_cmp(&ctx.area[a]));
            while !pending.is_empty() && !prog.cancelled() {
                let batch: Vec<usize> = pending.drain(..p.min(pending.len())).collect();
                prog.report(Event { stage: "improve".into(), msg: format!("Son plakadaki parçalar {}. plakaya taşınmaya çalışılıyor", k + 1), frac: 0.9, plates: None });
                let g = &geos[plates[k].spec];
                let base: Vec<usize> = plates[k].kps.iter().map(|x| x.kit).collect();
                let jobs: Vec<StripJob> = batch.iter().map(|&c| {
                    let mut v = base.clone();
                    v.push(c);
                    ctx.job(&v, g, short)
                }).collect();
                let res = solver.solve_many(jobs).await;
                let mut ok: Vec<(usize, Vec<KP>)> = vec![];
                for (i, r) in res.iter().enumerate() {
                    let mut v = base.clone();
                    v.push(batch[i]);
                    let kps = ctx.assign(&v, r);
                    if kps.len() == v.len() && ctx.fits(&kps, g).len() == kps.len() {
                        ok.push((batch[i], kps));
                    }
                }
                if let Some(best) = ok.iter().max_by(|a, b| ctx.area[a.0].total_cmp(&ctx.area[b.0])).cloned() {
                    plates[k].kps = best.1;
                    plates[n - 1].kps.retain(|x| x.kit != best.0);
                    // başarılı diğer adaylar yeniden denenir
                    let again: Vec<usize> = ok.iter().map(|x| x.0).filter(|&c| c != best.0).collect();
                    pending.splice(0..0, again);
                    prog.report(Event { stage: "improve".into(), msg: format!("Bir parça {}. plakaya taşındı", k + 1), frac: 0.92, plates: Some(emit(&ctx, specs, &plates)) });
                }
                if plates[n - 1].kps.is_empty() {
                    break;
                }
            }
            if plates[n - 1].kps.is_empty() {
                break;
            }
        }
        if plates[n - 1].kps.is_empty() {
            let gone = plates.pop().unwrap();
            if let Some(c) = stock[gone.spec].as_mut() {
                *c += 1;
            }
        }
    }
    // 3. son plakayı sıkıştır (gerekirse daha küçük plakaya geç)
    if let Some(last) = plates.last() {
        if !prog.cancelled() {
            let ids: Vec<usize> = last.kps.iter().map(|x| x.kit).collect();
            let a = ctx.area_of(&last.kps);
            if let Some(c) = stock[last.spec].as_mut() {
                *c += 1;
            }
            let mut cands: Vec<usize> = (0..specs.len()).filter(|&i| stock[i].is_none_or(|c| c > 0) && geos[i].ls * geos[i].hs as f64 >= a && ids.iter().all(|&k| fits_somehow(&ctx.shapes[ctx.class[k]], &geos[i], gap))).collect();
            cands.sort_by(|&x, &y| (geos[x].l * geos[x].s).total_cmp(&(geos[y].l * geos[y].s)));
            let cur = last.spec;
            let cur_used = ctx.used(&last.kps);
            let mut best: Option<(usize, Vec<KP>)> = None;
            for &t in &cands {
                prog.report(Event { stage: "compress".into(), msg: "Son plaka sıkıştırılıyor".into(), frac: 0.96, plates: None });
                let jobs: Vec<StripJob> = (0..p).map(|_| ctx.job(&ids, &geos[t], secs)).collect();
                let res = solver.solve_many(jobs).await;
                let b = res.iter().map(|r| ctx.assign(&ids, r)).filter(|kps| kps.len() == ids.len() && ctx.fits(kps, &geos[t]).len() == kps.len()).min_by(|x, y| ctx.used(x).total_cmp(&ctx.used(y)));
                if let Some(b) = b {
                    if t != cur || ctx.used(&b) < cur_used {
                        best = Some((t, b));
                    }
                    break;
                }
                if t == cur {
                    break;
                }
            }
            let n = plates.len();
            if let Some((t, kps)) = best {
                plates[n - 1] = Plate { spec: t, kps };
            }
            if let Some(c) = stock[plates[n - 1].spec].as_mut() {
                *c -= 1;
            }
        }
    }
    let out = emit(&ctx, specs, &plates);
    let check = verify::check(&input.parts, &out, gap, margin);
    prog.report(Event { stage: "done".into(), msg: "Bitti".into(), frac: 1.0, plates: Some(out.clone()) });
    NestOutput { plates: out, unplaced, check, kits: kits.len(), in_holes }
}

/// Kaba sığma sınaması: en az bir açıda dış kutusu plakanın kullanılabilir alanına giriyor mu.
fn fits_somehow(sh: &[(f32, f32)], g: &Geo, gap: f64) -> bool {
    let (w, h) = (g.ls - gap, g.hs as f64 - 2.0 * gap);
    (0..180).any(|d| {
        let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &p in sh {
            let (x, y) = rot_pt(p, d as f32);
            x0 = x0.min(x);
            x1 = x1.max(x);
            y0 = y0.min(y);
            y1 = y1.max(y);
        }
        x1 - x0 <= w + 1e-6 && y1 - y0 <= h + 1e-6
    })
}
