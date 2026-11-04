// Havuz işçisi: tek bir şerit doldurma işini (sparrow) çözer.
import init, { solve_strip } from '../pkg/nestio_wasm.js';

const ready = init();
self.onmessage = async (ev) => {
  await ready;
  let out;
  try {
    out = solve_strip(ev.data.job);
  } catch (e) {
    out = JSON.stringify({ width: 0, placed: [], error: String(e?.message ?? e) });
  }
  self.postMessage({ id: ev.data.id, out });
};
