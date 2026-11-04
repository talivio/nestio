// Yönetici işçi: çizimi çözümler, plaka döngüsünü yürütür, şerit işlerini havuza dağıtır.
import init, { dxf_to_drawing, analyze, nest, write_dxf, solve_strip } from '../pkg/nestio_wasm.js?v=20261104b';

const ready = init();
let pool = [];
let cancelled = false;

function spawn() {
  const w = new Worker(new URL('./pool.js?v=20261104b', import.meta.url), { type: 'module' });
  w.busy = false;
  return w;
}

function ensurePool(n) {
  if (inline) return;
  while (pool.length < n) pool.push(spawn());
}

// Düşen ya da süresi aşılan işçi atılır, yerine yenisi açılır; iş hatayla döner.
function replace(w, id, why) {
  clearTimeout(w.watch);
  w.terminate();
  const i = pool.indexOf(w);
  if (i >= 0) pool[i] = spawn();
  const done = waiting.get(id);
  waiting.delete(id);
  done?.(JSON.stringify({ width: 0, placed: [], error: why }));
  pump();
}

let seq = 0;
const waiting = new Map();
const queue = [];

// İç içe işçi açılamıyorsa (bazı tarayıcılar) işler burada, sırayla çözülür.
let inline = false;
function solveInline(t) {
  let out;
  try {
    out = solve_strip(t.job);
  } catch (e) {
    out = JSON.stringify({ width: 0, placed: [], error: String(e?.message ?? e) });
  }
  const done = waiting.get(t.id);
  waiting.delete(t.id);
  done?.(out);
}

function pump() {
  if (inline) {
    while (queue.length) {
      const t = queue.shift();
      setTimeout(() => solveInline(t), 0);
    }
    return;
  }
  for (const w of pool) {
    if (w.busy || !queue.length) continue;
    const t = queue.shift();
    w.busy = true;
    w.job = t;
    const secs = JSON.parse(t.job).secs || 60;
    w.watch = setTimeout(() => replace(w, t.id, 'çözücü işçisi yanıt vermedi'), (secs * 3 + 60) * 1000);
    w.onerror = (e) => {
      e.preventDefault?.();
      if (!w.alive && !(e instanceof ErrorEvent)) {
        // betik hiç yüklenemedi: havuzu bırak, işleri burada çöz
        inline = true;
        clearTimeout(w.watch);
        for (const p of pool) {
          clearTimeout(p.watch);
          p.terminate();
          if (p.busy && p.job) queue.unshift(p.job);
        }
        pool = [];
        return pump();
      }
      replace(w, t.id, 'çözücü işçisi düştü: ' + (e.message || 'bilinmeyen hata'));
    };
    w.onmessage = (ev) => {
      w.alive = true;
      if (ev.data.dead) return replace(w, t.id, ev.data.out);
      clearTimeout(w.watch);
      w.busy = false;
      waiting.get(ev.data.id)?.(ev.data.out);
      waiting.delete(ev.data.id);
      pump();
    };
    w.postMessage(t);
  }
}

function run(job) {
  return new Promise((resolve) => {
    const id = ++seq;
    waiting.set(id, resolve);
    queue.push({ id, job });
    pump();
  });
}

async function solver(jobsJson) {
  const jobs = JSON.parse(jobsJson);
  // tek iş parçacığında süre bölüşülür, toplam süre aynı kalır
  if (inline) for (const j of jobs) j.secs = Math.max(3, j.secs / jobs.length);
  const outs = await Promise.all(jobs.map((j) => run(JSON.stringify(j))));
  return '[' + outs.join(',') + ']';
}

// Rust paniği (WASM tuzağı) sözü hiç çözmez: arayüz sonsuza dek beklemesin.
let nestId = null;
const fail = (e) => {
  if (nestId === null) return;
  self.postMessage({ type: 'error', id: nestId, error: 'Hesap beklenmedik biçimde durdu: ' + String(e?.message ?? e) });
  nestId = null;
};
self.__nestioPanic = (msg) => fail('Rust paniği: ' + msg);
self.addEventListener('error', (e) => fail(e.error ?? e.message));
self.addEventListener('unhandledrejection', (e) => fail(e.reason));

self.onmessage = async (ev) => {
  await ready;
  const m = ev.data;
  try {
    if (m.type === 'dxf') {
      self.postMessage({ type: 'drawing', id: m.id, drawing: dxf_to_drawing(new Uint8Array(m.bytes)) });
    } else if (m.type === 'analyze') {
      self.postMessage({ type: 'analysis', id: m.id, analysis: analyze(m.drawing, JSON.stringify(m.options ?? {})) });
    } else if (m.type === 'nest') {
      cancelled = false;
      nestId = m.id;
      ensurePool(m.width);
      const out = await nest(
        JSON.stringify(m.input), solver, m.width,
        (e) => self.postMessage({ type: 'progress', ev: e }),
        () => cancelled,
      );
      nestId = null;
      self.postMessage({ type: 'result', id: m.id, out });
    } else if (m.type === 'cancel') {
      cancelled = true;
    } else if (m.type === 'write') {
      const bytes = write_dxf(JSON.stringify(m.parts), JSON.stringify(m.plates), m.gap);
      self.postMessage({ type: 'dxfout', id: m.id, bytes }, [bytes.buffer]);
    }
  } catch (e) {
    self.postMessage({ type: 'error', id: m.id, error: String(e?.message ?? e) });
  }
};
