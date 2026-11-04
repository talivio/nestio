//! Komut satırı: `nestio analiz <çizim>` ve `nestio diz <çizim> [seçenekler]`.
//! Tarayıcıdaki sürümle aynı çekirdeği kullanır; çözücü işleri iş parçacıklarında koşar.
use nestio_core::nest::{self, Event, NestInput, PlateSpec, Progress, Solver};
use nestio_core::strip::{self, StripJob, StripResult};
use nestio_core::{dxf_in, dxf_out, model::Drawing, parts};
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};
use std::time::Instant;

struct Threads(usize);

impl Solver for Threads {
    fn width(&self) -> usize {
        self.0
    }
    fn solve_many(&self, jobs: Vec<StripJob>) -> impl Future<Output = Vec<StripResult>> {
        let out = std::thread::scope(|s| {
            let hs: Vec<_> = jobs.iter().map(|j| s.spawn(move || strip::solve(j))).collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        std::future::ready(out)
    }
}

struct Log(Instant);

impl Progress for Log {
    fn report(&self, ev: Event) {
        eprintln!("[{:6.1} sn] {}: {}", self.0.elapsed().as_secs_f64(), ev.stage, ev.msg);
    }
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

fn load(path: &str) -> Drawing {
    let bytes = std::fs::read(path).expect("dosya okunamadı");
    if path.to_lowercase().ends_with(".json") {
        serde_json::from_slice(&bytes).expect("çizim JSON'u okunamadı")
    } else {
        dxf_in::read(&bytes).expect("DXF okunamadı")
    }
}

fn arg(a: &[String], k: &str) -> Option<String> {
    a.iter().position(|x| x == k).and_then(|i| a.get(i + 1).cloned())
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("kullanım:\n  nestio analiz <çizim.dxf>\n  nestio diz <çizim.dxf> [--plaka 3000x1500[:adet]]... [--bosluk 10] [--pay 10] [--sure 60] [--is 4] [--delik 0] [--cikti sonuc.dxf] [--json sonuc.json]");
        std::process::exit(2);
    }
    let t = Instant::now();
    let d = load(&a[2]);
    let an = parts::analyze(&d, &parts::AnalyzeOptions::default());
    eprintln!("analiz {:.1} sn, {} şekil", t.elapsed().as_secs_f64(), an.parts.len());
    if a[1] == "analiz" || a[1] == "analyze" {
        for p in &an.parts {
            println!("{:14} {:?} x{} t={:?} alan={:.4} m² {:.0}×{:.0} delik={} {}", p.name, p.role, p.qty, p.thick, p.area / 1e6, p.w, p.h, p.shape.rings.len() - 1, p.notes.join(" | "));
        }
        println!("{:?}", an.notes);
        return;
    }
    let (nparts, mut plates) = parts::to_nest(&an);
    let given: Vec<PlateSpec> = a.windows(2).filter(|w| w[0] == "--plaka").map(|w| {
        let (dims, cnt) = w[1].split_once(':').map(|(d, c)| (d, c.parse().ok())).unwrap_or((&w[1], None));
        let (x, y) = dims.split_once('x').expect("plaka: GENxYUK");
        PlateSpec { name: dims.into(), w: x.parse().unwrap(), h: y.parse().unwrap(), count: cnt }
    }).collect();
    if !given.is_empty() {
        plates = given;
    }
    if plates.is_empty() {
        eprintln!("plaka yok: --plaka GENxYUK verin");
        std::process::exit(2);
    }
    let workers = arg(&a, "--is").and_then(|v| v.parse().ok()).unwrap_or_else(|| (std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) / 3).max(1));
    let input = NestInput {
        parts: nparts,
        plates,
        gap: arg(&a, "--bosluk").and_then(|v| v.parse().ok()).unwrap_or(10.0),
        margin: arg(&a, "--pay").and_then(|v| v.parse().ok()).unwrap_or(10.0),
        secs: arg(&a, "--sure").and_then(|v| v.parse().ok()).unwrap_or(60.0),
        fill_holes: arg(&a, "--delik").is_none_or(|v| v != "0"),
        improve: arg(&a, "--iyilestir").is_none_or(|v| v != "0"),
        seed: arg(&a, "--tohum").and_then(|v| v.parse().ok()).unwrap_or(0),
    };
    eprintln!("{} parça çeşidi, {} adet; plakalar {:?}; {} paralel iş", input.parts.len(), input.parts.iter().map(|p| p.qty).sum::<u32>(), input.plates.iter().map(|p| (p.w, p.h, p.count)).collect::<Vec<_>>(), workers);
    let t = Instant::now();
    let out = block_on(nest::nest(&input, &Threads(workers), &Log(t)));
    let secs = t.elapsed().as_secs_f64();
    for (i, (p, c)) in out.plates.iter().zip(&out.check.plates).enumerate() {
        println!("plaka {}: {:.0}×{:.0}, {} parça, doluluk %{:.1}, kullanılan boy {:.0} mm, en yakın iki parça {:?} mm", i + 1, p.w, p.h, c.parts, c.fill * 100.0, c.used, c.min_dist.map(|d| (d * 10.0).round() / 10.0));
    }
    println!("yerleşen {} / {}, delikte {}, yerleşmeyen {}, denetim {}, süre {:.0} sn", out.check.placed, input.parts.iter().map(|p| p.qty as usize).sum::<usize>(), out.in_holes, out.unplaced.len(), if out.check.ok { "hatasız".to_string() } else { format!("{} hata: {:?}", out.check.errors.len(), &out.check.errors[..out.check.errors.len().min(5)]) }, secs);
    if let Some(p) = arg(&a, "--cikti") {
        std::fs::write(&p, dxf_out::write(&input.parts, &out.plates, input.gap).unwrap()).unwrap();
    }
    if let Some(p) = arg(&a, "--json") {
        std::fs::write(&p, serde_json::to_vec(&out).unwrap()).unwrap();
    }
}
