//! Sentetik örnek çizimde uçtan uca: analiz, delik doldurma, dizme, denetim, DXF.
use nestio_core::nest::{self, Event, NestInput, Progress, Solver};
use nestio_core::parts::{self, AnalyzeOptions, Role};
use nestio_core::strip::{self, StripJob, StripResult};
use nestio_core::{dxf_in, dxf_out};
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

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

fn ornek() -> parts::Analysis {
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ornek-flans-braket.dxf")).unwrap();
    parts::analyze(&dxf_in::read(&bytes).unwrap(), &AnalyzeOptions::default())
}

#[test]
fn analiz_parcalari_ve_plakalari_bulur() {
    let a = ornek();
    let plates: Vec<_> = a.parts.iter().filter(|p| p.role == Role::Plate).collect();
    assert_eq!(plates.len(), 2);
    assert!(plates.iter().all(|p| (p.w - 3000.0).abs() < 0.01 && (p.h - 1500.0).abs() < 0.01));
    let parts: Vec<_> = a.parts.iter().filter(|p| p.role == Role::Part).collect();
    assert_eq!(parts.len(), 7);
    assert_eq!(parts.iter().map(|p| p.qty).sum::<u32>(), 46);
    let flans = parts.iter().find(|p| p.name == "FLANS").unwrap();
    assert_eq!(flans.qty, 4);
    assert_eq!(flans.shape.rings.len(), 10); // dış + orta delik + 8 cıvata deliği
}

#[test]
fn dizer_ve_denetimden_gecer() {
    let a = ornek();
    let (parts, plates) = parts::to_nest(&a);
    for (gap, margin) in [(10.0, 10.0), (0.0, 5.0), (6.0, 0.0)] {
        let input = NestInput { parts: parts.clone(), plates: plates.clone(), gap, margin, secs: 3.0, fill_holes: true, improve: true, seed: 1 };
        let out = block_on(nest::nest(&input, &Threads, &Quiet));
        assert!(out.check.ok, "boşluk {gap}, pay {margin}: {:?}", out.check.errors);
        assert!(out.unplaced.is_empty());
        assert_eq!(out.check.placed, 46);
        assert!(out.in_holes > 0);
        let dxf = dxf_out::write(&input.parts, &out.plates, gap).unwrap();
        assert!(dxf.len() > 1000);
    }
}
