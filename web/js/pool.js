// Havuz işçisi: tek bir şerit doldurma işini (sparrow) çözer.
import init, { solve_strip } from '../pkg/nestio_wasm.js?v=20261104c';

let panic = null;
self.__nestioPanic = (msg) => { panic = msg; };
const ready = init();
self.onmessage = async (ev) => {
  let out;
  try {
    await ready;
    out = solve_strip(ev.data.job);
  } catch (e) {
    // WASM tuzağından sonra örnek güvenilmez: yönetici bu işçiyi yeniler
    out = JSON.stringify({ width: 0, placed: [], error: panic ?? String(e?.message ?? e) });
    self.postMessage({ id: ev.data.id, out, dead: true });
    return;
  }
  self.postMessage({ id: ev.data.id, out });
};
