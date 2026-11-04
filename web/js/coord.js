// Yönetici işçi: çizimi çözümler, plaka döngüsünü yürütür, şerit işlerini havuza dağıtır.
import init, { dxf_to_drawing, analyze, nest, write_dxf } from '../pkg/nestio_wasm.js';

const ready = init();
let pool = [];
let cancelled = false;

function ensurePool(n) {
  while (pool.length < n) {
    const w = new Worker(new URL('./pool.js', import.meta.url), { type: 'module' });
    w.busy = false;
    pool.push(w);
  }
}

let seq = 0;
const waiting = new Map();
const queue = [];

function pump() {
  for (const w of pool) {
    if (w.busy || !queue.length) continue;
    const t = queue.shift();
    w.busy = true;
    w.onmessage = (ev) => {
      w.busy = false;
      waiting.get(ev.data.id)(ev.data.out);
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
  const outs = await Promise.all(jobs.map((j) => run(JSON.stringify(j))));
  return '[' + outs.join(',') + ']';
}

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
      ensurePool(m.width);
      const out = await nest(
        JSON.stringify(m.input), solver, m.width,
        (e) => self.postMessage({ type: 'progress', ev: e }),
        () => cancelled,
      );
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
