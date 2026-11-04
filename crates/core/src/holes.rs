//! Deliklerin içine küçük parça yerleştirir. Ev sahibi ve konukları birlikte
//! tek bir "kit" olarak dizilir. Yer arama ızgarada FFT ilişkisiyle yapılır,
//! bulunan aday kesin geometriyle doğrulanır.
use crate::nest::NestPart;
use geo::{Area, BooleanOps, BoundingRect, Buffer, Contains, Intersects, MultiPolygon, Polygon, Rotate, Translate};
use geo::algorithm::buffer::{BufferStyle, LineJoin};
use rustfft::{FftPlanner, num_complex::Complex32};
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct Member {
    pub part: usize,
    pub copy: u32,
    /// Ev sahibinin koordinatında: biçim `rot` derece döndürülür, (dx, dy) kaydırılır.
    pub rot: f64,
    pub dx: f64,
    pub dy: f64,
}

/// İlk üye ev sahibidir (dönüşümsüz).
#[derive(Clone, Debug)]
pub struct Kit {
    pub members: Vec<Member>,
}

const MAX_CELLS: f64 = 160_000.0;
const STEP_DEG: usize = 4;
const TRIES: usize = 40;

fn miter(p: &MultiPolygon<f64>, d: f64) -> MultiPolygon<f64> {
    p.buffer_with_style(BufferStyle::new(d).line_join(LineJoin::Miter(2.0)))
}

/// Hücre merkezlerine göre tek/çift kuralıyla doldurur. Satır başına bir dilim.
fn rasterize(g: &MultiPolygon<f64>, x0: f64, y0: f64, nx: usize, ny: usize, res: f64) -> Vec<bool> {
    let mut m = vec![false; nx * ny];
    let mut edges: Vec<(f64, f64, f64, f64)> = vec![];
    for p in &g.0 {
        for r in std::iter::once(p.exterior()).chain(p.interiors()) {
            for w in r.0.windows(2) {
                edges.push((w[0].x, w[0].y, w[1].x, w[1].y));
            }
        }
    }
    let mut xs: Vec<f64> = vec![];
    for iy in 0..ny {
        let y = y0 + (iy as f64 + 0.5) * res;
        xs.clear();
        for &(ax, ay, bx, by) in &edges {
            if (ay > y) != (by > y) {
                xs.push(ax + (y - ay) * (bx - ax) / (by - ay));
            }
        }
        xs.sort_by(|a, b| a.total_cmp(b));
        for c in xs.chunks(2) {
            if c.len() < 2 {
                break;
            }
            let i0 = ((c[0] - x0) / res - 0.5).ceil().max(0.0) as usize;
            let i1 = ((c[1] - x0) / res - 0.5).floor().min(nx as f64 - 1.0);
            if i1 < 0.0 {
                continue;
            }
            for ix in i0..=(i1 as usize) {
                m[iy * nx + ix] = true;
            }
        }
    }
    m
}

struct Fft2 {
    ny: usize,
    nx: usize,
    row: std::sync::Arc<dyn rustfft::Fft<f32>>,
    col: std::sync::Arc<dyn rustfft::Fft<f32>>,
    irow: std::sync::Arc<dyn rustfft::Fft<f32>>,
    icol: std::sync::Arc<dyn rustfft::Fft<f32>>,
}

impl Fft2 {
    fn new(ny: usize, nx: usize) -> Fft2 {
        let mut pl = FftPlanner::new();
        Fft2 { ny, nx, row: pl.plan_fft_forward(nx), col: pl.plan_fft_forward(ny), irow: pl.plan_fft_inverse(nx), icol: pl.plan_fft_inverse(ny) }
    }
    fn run(&self, d: &mut [Complex32], inv: bool) {
        let (r, c) = if inv { (&self.irow, &self.icol) } else { (&self.row, &self.col) };
        r.process(d);
        let mut tmp = vec![Complex32::new(0.0, 0.0); self.ny];
        for x in 0..self.nx {
            for y in 0..self.ny {
                tmp[y] = d[y * self.nx + x];
            }
            c.process(&mut tmp);
            for y in 0..self.ny {
                d[y * self.nx + x] = tmp[y];
            }
        }
    }
}

fn good_size(n: usize) -> usize {
    // 2, 3, 5 çarpanlı en yakın büyük boy
    let mut m = n.max(1);
    loop {
        let mut k = m;
        for f in [2, 3, 5] {
            while k % f == 0 {
                k /= f;
            }
        }
        if k == 1 {
            return m;
        }
        m += 1;
    }
}

/// Bir deliğin o anki serbest bölgesi için hazırlanmış arama.
struct Field {
    alan: MultiPolygon<f64>,
    engel: Option<MultiPolygon<f64>>,
    x0: f64,
    y0: f64,
    nx: usize,
    ny: usize,
    res: f64,
    fft: Fft2,
    fspec: Vec<Complex32>,
    w: f64,
    h: f64,
}

impl Field {
    fn new(alan: &MultiPolygon<f64>, engel: Option<&MultiPolygon<f64>>) -> Option<Field> {
        let free = match engel {
            Some(e) => alan.difference(e),
            None => alan.clone(),
        };
        if free.unsigned_area() < 1.0 {
            return None;
        }
        let b = free.bounding_rect()?;
        let res = (b.width() * b.height() / MAX_CELLS).sqrt().max(1.0);
        let nx0 = (b.width() / res).ceil() as usize + 2;
        let ny0 = (b.height() / res).ceil() as usize + 2;
        let (nx, ny) = (good_size(nx0), good_size(ny0));
        let (x0, y0) = (b.min().x - res, b.min().y - res);
        let inside = rasterize(&free, x0, y0, nx, ny, res);
        let mut fspec: Vec<Complex32> = inside.iter().map(|&v| Complex32::new(if v { 0.0 } else { 1.0 }, 0.0)).collect();
        let fft = Fft2::new(ny, nx);
        fft.run(&mut fspec, false);
        Some(Field { alan: alan.clone(), engel: engel.cloned(), x0, y0, nx, ny, res, fft, fspec, w: b.width(), h: b.height() })
    }

    /// En alttaki (sonra en soldaki) uygun yer: (döndürme, dx, dy).
    fn place(&self, g: &Polygon<f64>, gbuf: &MultiPolygon<f64>) -> Option<(f64, f64, f64)> {
        let mut best: Option<((i64, i64), f64, f64, f64)> = None;
        for rot in (0..360).step_by(STEP_DEG) {
            let rot = rot as f64;
            let gr = g.rotate_around_point(rot, geo::Point::new(0.0, 0.0));
            let gb = gr.bounding_rect()?;
            if gb.width() > self.w + 1e-6 || gb.height() > self.h + 1e-6 {
                continue;
            }
            let mb_poly = gbuf.rotate_around_point(rot, geo::Point::new(0.0, 0.0));
            let mb = mb_poly.bounding_rect()?;
            let (gx0, gy0) = (mb.min().x - self.res, mb.min().y - self.res);
            let mx = (mb.width() / self.res).ceil() as usize + 2;
            let my = (mb.height() / self.res).ceil() as usize + 2;
            if mx > self.nx || my > self.ny {
                continue;
            }
            let mask = rasterize(&mb_poly, gx0, gy0, mx, my, self.res);
            let mut ms = vec![Complex32::new(0.0, 0.0); self.nx * self.ny];
            for y in 0..my {
                for x in 0..mx {
                    if mask[y * mx + x] {
                        ms[y * self.nx + x] = Complex32::new(1.0, 0.0);
                    }
                }
            }
            self.fft.run(&mut ms, false);
            for (m, f) in ms.iter_mut().zip(&self.fspec) {
                *m = f * m.conj();
            }
            self.fft.run(&mut ms, true);
            let scale = 1.0 / (self.nx * self.ny) as f32;
            let mut tried = 0;
            'pos: for iy in 0..=(self.ny - my) {
                // daha alttaki bir sonuç zaten varsa bu açı onu geçemez
                if let Some(b) = &best {
                    let dy = self.y0 + iy as f64 * self.res - gy0;
                    if (dy.round() as i64) > b.0.0 {
                        break;
                    }
                }
                for ix in 0..=(self.nx - mx) {
                    if ms[iy * self.nx + ix].re * scale >= 0.5 {
                        continue;
                    }
                    let dx = self.x0 + ix as f64 * self.res - gx0;
                    let dy = self.y0 + iy as f64 * self.res - gy0;
                    let cand = gr.translate(dx, dy);
                    tried += 1;
                    if self.alan.contains(&cand) && self.engel.as_ref().is_none_or(|e| !cand.intersects(e)) {
                        let key = (dy.round() as i64, dx.round() as i64);
                        if best.as_ref().is_none_or(|b| key < b.0) {
                            best = Some((key, rot, dx, dy));
                        }
                        break 'pos;
                    }
                    if tried >= TRIES {
                        break 'pos;
                    }
                }
            }
        }
        best.map(|b| (b.1, b.2, b.3))
    }
}

pub fn build(parts: &[NestPart], gap: f64, enabled: bool, log: &dyn Fn(&str)) -> (Vec<Kit>, usize) {
    let mut inst: Vec<(usize, u32)> = vec![];
    for (i, p) in parts.iter().enumerate() {
        for c in 0..p.qty {
            inst.push((i, c));
        }
    }
    let polys: Vec<Polygon<f64>> = parts.iter().map(|p| p.shape.poly()).collect();
    let areas: Vec<f64> = polys.iter().map(|p| p.unsigned_area()).collect();
    let mut kits_of: Vec<Vec<Member>> = vec![vec![]; inst.len()];
    let mut used = vec![false; inst.len()];
    let mut in_holes = 0;
    if enabled {
        let largest_hole = |i: usize| polys[inst[i].0].interiors().iter().map(|r| Polygon::new(r.clone(), vec![]).unsigned_area()).fold(0.0, f64::max);
        let mut hosts: Vec<usize> = (0..inst.len()).filter(|&i| largest_hole(i) > 0.0).collect();
        hosts.sort_by(|&a, &b| largest_hole(b).total_cmp(&largest_hole(a)));
        let mut guests: Vec<usize> = (0..inst.len()).collect();
        guests.sort_by(|&a, &b| areas[inst[b].0].total_cmp(&areas[inst[a].0]));
        let smallest = guests.last().map(|&g| areas[inst[g].0]).unwrap_or(0.0);
        let gbufs: Vec<MultiPolygon<f64>> = polys.iter().map(|p| MultiPolygon::new(vec![Polygon::new(p.exterior().clone(), vec![])])).collect();
        let shells: Vec<Polygon<f64>> = polys.iter().map(|p| Polygon::new(p.exterior().clone(), vec![])).collect();
        for (hi, &h) in hosts.iter().enumerate() {
            if used[h] {
                continue;
            }
            let hp = &polys[inst[h].0];
            let mut holes: Vec<Polygon<f64>> = hp.interiors().iter().map(|r| Polygon::new(r.clone(), vec![])).collect();
            holes.sort_by(|a, b| b.unsigned_area().total_cmp(&a.unsigned_area()));
            for hole in holes {
                if hole.unsigned_area() < smallest {
                    break;
                }
                let alan = miter(&MultiPolygon::new(vec![hole]), -gap);
                if alan.unsigned_area() < smallest {
                    continue;
                }
                let mut konan: Vec<Polygon<f64>> = vec![];
                let mut field = Field::new(&alan, None);
                let mut failed: HashSet<usize> = HashSet::new();
                for &g in &guests {
                    let Some(f) = field.as_ref() else { break };
                    let gp = inst[g].0;
                    if used[g] || g == h || !kits_of[g].is_empty() || failed.contains(&gp) || areas[gp] > f.alan.unsigned_area() {
                        continue;
                    }
                    // biraz şişirilmiş maske: ızgara hatası kesin sınamayla yakalanır
                    let gb = miter(&gbufs[gp], 1.5 * f.res);
                    match f.place(&shells[gp], &gb) {
                        Some((rot, dx, dy)) => {
                            used[g] = true;
                            in_holes += 1;
                            kits_of[h].push(Member { part: gp, copy: inst[g].1, rot, dx, dy });
                            konan.push(polys[gp].rotate_around_point(rot, geo::Point::new(0.0, 0.0)).translate(dx, dy));
                            let shells_k: Vec<Polygon<f64>> = konan.iter().map(|p| Polygon::new(p.exterior().clone(), vec![])).collect();
                            let engel = miter(&MultiPolygon::new(shells_k), gap);
                            field = Field::new(&alan, Some(&engel));
                            failed.clear();
                            log(&format!("{} deliğine {} yerleşti", parts[inst[h].0].name, parts[gp].name));
                        }
                        None => {
                            failed.insert(gp);
                        }
                    }
                }
            }
            if hi % 8 == 0 {
                log(&format!("Delik taraması {}/{}", hi + 1, hosts.len()));
            }
        }
    }
    let mut kits = vec![];
    for (i, &(p, c)) in inst.iter().enumerate() {
        if used[i] {
            continue;
        }
        let mut m = vec![Member { part: p, copy: c, rot: 0.0, dx: 0.0, dy: 0.0 }];
        m.extend(kits_of[i].drain(..));
        kits.push(Kit { members: m });
    }
    (kits, in_holes)
}
