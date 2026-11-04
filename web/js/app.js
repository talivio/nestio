// Nestio arayüzü. Bütün hesap Web Worker'larda (WASM) yapılır; ağa yalnız sayfanın
// kendi dosyaları için istek gider, çizim hiçbir yere gönderilmez.
import { t, tm, numLocale } from './i18n.js?v=20261104b';

const $ = (s) => document.querySelector(s);
const fmt = (n, d = 0) => Number(n).toLocaleString(numLocale, { maximumFractionDigits: d, minimumFractionDigits: d });
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

const cores = Math.max(1, Math.min(12, (navigator.hardwareConcurrency || 4) - 1));
$('#cores').textContent = cores;

const S = {
  fileName: 'cizim',
  drawing: null,
  analysis: null,
  role: {}, // id → part | plate | ignore
  qty: {},
  ignoreLayers: new Set(),
  plates: [],
  secs: 45,
  input: null,
  out: null,
};

// ── yönetici işçi ────────────────────────────────────────────────────────────
let coord = null;
let seq = 0;
const pending = new Map();
function startCoord() {
  coord = new Worker(new URL('./coord.js?v=20261104b', import.meta.url), { type: 'module' });
  coord.onmessage = (ev) => {
    const m = ev.data;
    if (m.type === 'progress') return onProgress(JSON.parse(m.ev));
    const p = pending.get(m.id);
    if (!p) return;
    pending.delete(m.id);
    m.type === 'error' ? p.reject(new Error(m.error)) : p.resolve(m);
  };
  coord.onerror = (e) => status(t('app.workerFail', { e: tm(e.message) || t('app.workerFailHint') }), true);
}
function ask(msg, transfer) {
  if (!coord) startCoord();
  const id = ++seq;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    coord.postMessage({ ...msg, id }, transfer || []);
  });
}

function status(msg, bad) {
  const el = $('#status');
  el.textContent = msg;
  el.classList.toggle('hidden', !msg);
  el.style.color = bad ? 'var(--bad)' : '';
}

// ── dosya ────────────────────────────────────────────────────────────────────
async function readDwg(bytes) {
  const w = new Worker(new URL('./dwg.js?v=20261104b', import.meta.url), { type: 'module' });
  try {
    return await new Promise((resolve, reject) => {
      w.onmessage = (ev) => (ev.data.ok ? resolve(ev.data.drawing) : reject(new Error(ev.data.error)));
      w.onerror = () => reject(new Error(t('app.dwgFail')));
      w.postMessage({ bytes }, [bytes]);
    });
  } finally {
    w.terminate();
  }
}

async function loadFile(name, buf) {
  S.fileName = name.replace(/\.(dxf|dwg)$/i, '');
  S.role = {};
  S.qty = {};
  S.ignoreLayers = new Set();
  $('#result').classList.add('hidden');
  try {
    if (/\.dwg$/i.test(name)) {
      status('DWG okunuyor (okuyucu ilk seferde ~11 MB iner)…');
      S.drawing = await readDwg(buf);
    } else {
      status(t('app.reading'));
      S.drawing = (await ask({ type: 'dxf', bytes: buf }, [buf])).drawing;
    }
    await analyze(true);
  } catch (e) {
    status(t('app.readFail', { e: tm(e.message) }), true);
  }
}

async function analyze(first) {
  status(t('app.extracting'));
  const t0 = performance.now();
  const r = await ask({ type: 'analyze', drawing: S.drawing, options: { ignore_layers: [...S.ignoreLayers], plates: [] } });
  S.analysis = JSON.parse(r.analysis);
  if (first) {
    S.plates = groupPlates(S.analysis.parts.filter((p) => p.role === 'plate'));
    if (!S.plates.length) S.plates = [{ w: 3000, h: 1500, count: null }];
  }
  const n = S.analysis.parts.filter((p) => role(p) === 'part').length;
  status(n ? '' : t('app.noContours'), !n);
  console.debug('çözümleme', Math.round(performance.now() - t0), 'ms');
  renderSetup();
  $('#setup').classList.remove('hidden');
  if (first) $('#setup').scrollIntoView({ behavior: 'smooth', block: 'start' });
}

function groupPlates(list) {
  const out = [];
  for (const p of list) {
    const w = Math.round(Math.max(p.w, p.h) * 10) / 10;
    const h = Math.round(Math.min(p.w, p.h) * 10) / 10;
    const g = out.find((x) => Math.abs(x.w - w) < 0.5 && Math.abs(x.h - h) < 0.5);
    g ? g.count++ : out.push({ w, h, count: 1 });
  }
  return out.sort((a, b) => b.w * b.h - a.w * a.h);
}

const role = (p) => S.role[p.id] ?? p.role;
const roleOptions = () => ['part', 'plate', 'ignore'].map((r) => `<option value="${r}">${t('role.' + r)}</option>`).join('');

function setRole(p, r) {
  if (r === p.role) delete S.role[p.id];
  else S.role[p.id] = r;
  S.plates = mergePlates();
  renderSetup();
}
const qty = (p) => S.qty[p.id] ?? p.qty;

// ── kurulum ekranı ───────────────────────────────────────────────────────────
function drawShape(ctx, rings, tf, fill, line) {
  ctx.beginPath();
  for (const r of rings) {
    r.forEach((p, i) => {
      const [x, y] = tf(p);
      i ? ctx.lineTo(x, y) : ctx.moveTo(x, y);
    });
    ctx.closePath();
  }
  ctx.fillStyle = fill;
  ctx.fill('evenodd');
  ctx.strokeStyle = line;
  ctx.lineWidth = 1;
  ctx.stroke();
}

const css = (v) => getComputedStyle(document.documentElement).getPropertyValue(v).trim();

function thumb(canvas, p) {
  const dpr = devicePixelRatio || 1;
  const W = canvas.clientWidth || 100;
  const H = canvas.clientHeight || 86;
  canvas.width = W * dpr;
  canvas.height = H * dpr;
  const ctx = canvas.getContext('2d');
  ctx.scale(dpr, dpr);
  const s = Math.min((W - 6) / p.w, (H - 6) / p.h);
  const ox = (W - p.w * s) / 2;
  const oy = (H - p.h * s) / 2;
  const r = role(p);
  drawShape(ctx, p.shape.rings, ([x, y]) => [ox + x * s, H - oy - y * s],
    r === 'plate' ? css('--blue-200') : css('--part'), r === 'plate' ? css('--plate-line') : css('--part-line'));
}

function renderSetup() {
  const a = S.analysis;
  const parts = a.parts;
  const active = parts.filter((p) => role(p) === 'part');
  const count = active.reduce((s, p) => s + qty(p), 0);
  const area = active.reduce((s, p) => s + p.area * qty(p), 0);
  $('#partsum').textContent = t('app.partsum', { n: count, a: fmt(area / 1e6, 2) });

  const notes = a.notes.map(tm);
  const warned = parts.filter((p) => p.notes.length);
  if (a.units !== 1) notes.push(t('app.units', { u: a.units }));
  if (warned.length) notes.push(t('app.warned', { n: warned.length }));
  const manual = parts.filter((p) => S.role[p.id] && S.role[p.id] !== p.role);
  const label = { part: t('role.partL'), plate: t('role.plateL'), ignore: t('role.ignoreL') };
  if (manual.length) notes.push(t('app.manual', { list: manual.map((p) => `${p.name} → ${label[S.role[p.id]]}`).join(', ') }));
  $('#notes').innerHTML = notes.length ? `<b>${t('app.attention')}</b><ul>` + notes.map((n) => `<li>${esc(n)}</li>`).join('') + '</ul>' + (manual.length ? `<button type="button" class="linkbtn" id="resetroles">${t('app.resetRoles')}</button>` : '') : '';
  if (manual.length) $('#resetroles').onclick = () => { S.role = {}; S.plates = mergePlates(); renderSetup(); };
  $('#notes').classList.toggle('hidden', !notes.length);

  const tiles = $('#tiles');
  tiles.innerHTML = '';
  for (const p of parts) {
    const r = role(p);
    const d = document.createElement('div');
    d.className = `tile cut-sm ${r === 'part' ? '' : r} ${p.notes.length ? 'warn' : ''}`;
    d.title = [`${p.name} — ${fmt(p.w)} × ${fmt(p.h)} mm`, p.thick ? t('app.thickness', { t: fmt(p.thick, 1) }) : '', ...p.notes.map(tm)].filter(Boolean).join('\n');
    d.innerHTML = `<canvas></canvas><div class="nm">${esc(p.name)}</div><div class="sz">${fmt(p.w)} × ${fmt(p.h)}</div>${r === 'part' && qty(p) > 1 ? `<span class="qty">×${qty(p)}</span>` : ''}<select aria-label="${esc(t('app.typeOf', { name: p.name }))}">${roleOptions()}</select>`;
    const sel = d.querySelector('select');
    sel.value = r;
    sel.onchange = () => setRole(p, sel.value);
    tiles.append(d);
    requestAnimationFrame(() => thumb(d.querySelector('canvas'), p));
  }

  const tb = $('#ptable tbody');
  tb.innerHTML = '';
  for (const p of parts) {
    const tr = document.createElement('tr');
    tr.innerHTML = `<td>${esc(p.name)}${p.notes.length ? ' <span title="' + esc(p.notes.map(tm).join('\n')) + '" style="color:var(--warn)">⚠</span>' : ''}</td><td class="mono">${fmt(p.w)} × ${fmt(p.h)}</td><td><input type="number" min="0" step="1" value="${qty(p)}" aria-label="${t('app.qty')}"></td><td><select aria-label="${t('app.type')}">${roleOptions()}</select></td>`;
    const [inp, sel] = [tr.querySelector('input'), tr.querySelector('select')];
    sel.value = role(p);
    inp.onchange = () => { S.qty[p.id] = Math.max(0, Math.round(+inp.value || 0)); renderSetup(); };
    sel.onchange = () => setRole(p, sel.value);
    tb.append(tr);
  }

  const ly = $('#layers');
  ly.innerHTML = '';
  for (const l of a.layers.concat([...S.ignoreLayers].filter((x) => !a.layers.includes(x)))) {
    const lab = document.createElement('label');
    lab.className = 'check';
    lab.innerHTML = `<input type="checkbox" ${S.ignoreLayers.has(l) ? '' : 'checked'}><span>${esc(l || '0')}</span>`;
    lab.querySelector('input').onchange = (e) => {
      e.target.checked ? S.ignoreLayers.delete(l) : S.ignoreLayers.add(l);
      analyze(false);
    };
    ly.append(lab);
  }
  renderPlates();
}

// Çizimden gelen plakalar elle girilenlerle birleşir.
function mergePlates() {
  const manual = S.plates.filter((p) => p.manual);
  const drawn = groupPlates(S.analysis.parts.filter((p) => role(p) === 'plate'));
  const out = drawn.concat(manual);
  return out.length ? out : [{ w: 3000, h: 1500, count: null, manual: true }];
}

function renderPlates() {
  const tb = $('#plates tbody');
  tb.innerHTML = '';
  S.plates.forEach((p, i) => {
    const tr = document.createElement('tr');
    tr.innerHTML = `<td><input type="number" min="1" step="1" value="${p.w}" aria-label="${t('app.lengthMm')}"></td><td><input type="number" min="1" step="1" value="${p.h}" aria-label="${t('app.widthMm')}"></td><td><input type="number" min="1" step="1" value="${p.count ?? ''}" placeholder="∞" aria-label="${t('app.qty')}"></td><td><button type="button" class="linkbtn">${t('app.delete')}</button></td>`;
    const [w, h, c] = tr.querySelectorAll('input');
    const upd = () => { p.w = +w.value; p.h = +h.value; p.count = c.value === '' ? null : Math.max(1, Math.round(+c.value)); p.manual = true; };
    w.onchange = h.onchange = c.onchange = upd;
    tr.querySelector('button').onclick = () => { S.plates.splice(i, 1); renderPlates(); };
    tb.append(tr);
  });
}

$('#addplate').onclick = () => { S.plates.push({ w: 3000, h: 1500, count: null, manual: true }); renderPlates(); };
for (const b of document.querySelectorAll('#time button')) {
  b.onclick = () => {
    document.querySelectorAll('#time button').forEach((x) => x.setAttribute('aria-pressed', String(x === b)));
    S.secs = +b.dataset.s;
  };
}

// ── dizme ────────────────────────────────────────────────────────────────────
let t0 = 0;
let timer = null;

async function run() {
  const parts = S.analysis.parts.filter((p) => role(p) === 'part' && qty(p) > 0)
    .map((p) => ({ id: p.id, name: p.name, shape: p.shape, qty: qty(p) }));
  const plates = S.plates.filter((p) => p.w > 0 && p.h > 0).map((p) => ({ name: `${p.w} × ${p.h}`, w: p.w, h: p.h, count: p.count }));
  if (!parts.length) return status(t('app.noParts'), true);
  if (!plates.length) return status(t('app.noSheets'), true);
  const gap = Math.max(0, +$('#gap').value || 0);
  const margin = Math.max(0, +$('#margin').value || 0);
  S.input = { parts, plates, gap, margin, secs: S.secs, fill_holes: $('#holes').checked, improve: $('#improve').checked, seed: Math.floor(Math.random() * 1e9) };
  status('');
  $('#log').textContent = '';
  $('#bar').style.width = '0';
  $('#worktitle').textContent = t('app.nesting');
  $('#work').classList.remove('hidden');
  $('#result').classList.add('hidden');
  $('#run').disabled = true;
  $('#work').scrollIntoView({ behavior: 'smooth', block: 'start' });
  t0 = performance.now();
  timer = setInterval(() => ($('#elapsed').textContent = clock((performance.now() - t0) / 1000)), 1000);
  try {
    const r = await ask({ type: 'nest', input: S.input, width: cores });
    S.out = JSON.parse(r.out);
    S.out.secs = (performance.now() - t0) / 1000;
    showResult();
  } catch (e) {
    if (e.message !== 'durduruldu') status(t('app.nestFail', { e: tm(e.message) }), true);
  } finally {
    clearInterval(timer);
    $('#run').disabled = false;
    $('#work').classList.add('hidden');
  }
}

const clock = (s) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`;

function onProgress(ev) {
  const log = $('#log');
  log.textContent += `[${clock((performance.now() - t0) / 1000)}] ${tm(ev.msg)}\n`;
  log.scrollTop = log.scrollHeight;
  $('#bar').style.width = Math.round(Math.min(1, ev.frac) * 100) + '%';
  if (ev.plates) {
    S.out = { plates: ev.plates, live: true };
    $('#worktitle').textContent = t('app.nestingReady', { n: ev.plates.length });
    renderPlatesView(ev.plates);
    $('#result').classList.remove('hidden');
  }
}

$('#run').onclick = run;
$('#stop').onclick = () => {
  coord?.terminate();
  coord = null;
  for (const p of pending.values()) p.reject(new Error('durduruldu'));
  pending.clear();
  status(t('app.stopped'));
};

// ── sonuç ────────────────────────────────────────────────────────────────────
function placedRings(it) {
  const p = S.input.parts[it.part];
  const c = Math.cos((it.rot * Math.PI) / 180);
  const s = Math.sin((it.rot * Math.PI) / 180);
  return p.shape.rings.map((r) => r.map(([x, y]) => [x * c - y * s + it.x, x * s + y * c + it.y]));
}

function renderPlatesView(plates) {
  const box = $('#plateviews');
  box.innerHTML = '';
  plates.forEach((pl, k) => {
    const d = document.createElement('div');
    d.className = 'plate';
    const fill = pl.fill * 100;
    const left = pl.w - pl.used - S.input.gap;
    d.innerHTML = `<div class="row" style="margin-bottom:6px"><h3>${t('app.sheet', { k: k + 1 })}</h3><span class="muted">${t('app.sheetLine', { w: fmt(pl.w), h: fmt(pl.h), n: pl.items.length, f: fmt(fill, 1), u: fmt(pl.used) })}${left > 1 ? t('app.remnant', { r: fmt(left), h: fmt(pl.h) }) : ''}</span></div><canvas class="cut-sm"></canvas>`;
    box.append(d);
    const cv = d.querySelector('canvas');
    requestAnimationFrame(() => drawPlate(cv, pl));
  });
}

function drawPlate(cv, pl) {
  const dpr = devicePixelRatio || 1;
  const W = cv.clientWidth;
  const s = (W - 2) / pl.w;
  const H = pl.h * s + 2;
  cv.style.height = H + 'px';
  cv.width = W * dpr;
  cv.height = H * dpr;
  const ctx = cv.getContext('2d');
  ctx.scale(dpr, dpr);
  const tf = ([x, y]) => [1 + x * s, H - 1 - y * s];
  const cut = Math.min(pl.w, pl.used + S.input.gap);
  if (pl.w - cut > 1) {
    ctx.fillStyle = css('--scrap');
    ctx.fillRect(1 + cut * s, 1, (pl.w - cut) * s, pl.h * s);
  }
  ctx.strokeStyle = css('--plate-line');
  ctx.lineWidth = 1.5;
  ctx.strokeRect(1, 1, pl.w * s, pl.h * s);
  for (const it of pl.items) drawShape(ctx, placedRings(it), tf, css('--part'), css('--part-line'));
  if (s > 0.12) {
    ctx.fillStyle = css('--text');
    ctx.font = `${Math.max(8, Math.min(11, 60 * s))}px Inter, sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    for (const it of pl.items) {
      const r = placedRings(it)[0];
      let cx = 0, cy = 0;
      for (const p of r) { cx += p[0]; cy += p[1]; }
      const [x, y] = tf([cx / r.length, cy / r.length]);
      ctx.fillText(S.input.parts[it.part].name, x, y);
    }
  }
}

function showResult() {
  const o = S.out;
  const total = o.plates.reduce((a, p) => a + p.w * p.h, 0);
  const used = o.plates.reduce((a, p) => a + p.fill * p.w * p.h, 0);
  const last = o.plates[o.plates.length - 1];
  const need = S.input.parts.reduce((a, p) => a + p.qty, 0);
  $('#summary').innerHTML = [
    [o.plates.length, t('stat.sheets')],
    [t('app.pct', { v: fmt((100 * used) / Math.max(1, total), 1) }), t('stat.fill')],
    [last ? fmt(last.used) + ' mm' : '–', t('stat.lastUsed')],
    [`${o.check.placed} / ${need}`, t('stat.placed')],
    [o.in_holes, t('stat.holes')],
    [clock(o.secs), t('stat.time')],
  ].map(([b, s]) => `<div class="stat cut-sm"><b>${b}</b><span>${s}</span></div>`).join('');
  $('#verdict').innerHTML = o.check.ok
    ? `<span class="badge ok cut-sm">${t('app.checkOk')}</span>`
    : `<span class="badge bad cut-sm">${t('app.checkBad', { n: o.check.errors.length })}</span>`;
  const notes = [];
  if (o.unplaced.length) notes.push(t('app.unplaced', { n: o.unplaced.length }) + [...new Set(o.unplaced.map((u) => `${S.input.parts[u[0]].name} (${u[2]})`))].join(', '));
  notes.push(...o.check.errors.slice(0, 8).map(tm));
  $('#rnotes').innerHTML = notes.length ? '<ul>' + notes.map((n) => `<li>${esc(n)}</li>`).join('') + '</ul>' : '';
  $('#rnotes').classList.toggle('hidden', !notes.length);
  renderPlatesView(o.plates);
  $('#result').classList.remove('hidden');
  $('#result').scrollIntoView({ behavior: 'smooth', block: 'start' });
}

$('#dl').onclick = async () => {
  if (!S.out?.plates) return;
  const r = await ask({ type: 'write', parts: S.input.parts, plates: S.out.plates, gap: S.input.gap });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([r.bytes], { type: 'application/dxf' }));
  a.download = `${S.fileName}-${t('app.fileSuffix')}.dxf`;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 5000);
};
$('#again').onclick = () => $('#setup').scrollIntoView({ behavior: 'smooth' });

addEventListener('resize', () => {
  if (S.out?.plates && !$('#result').classList.contains('hidden')) renderPlatesView(S.out.plates);
});

// ── giriş ────────────────────────────────────────────────────────────────────
const drop = $('#drop');
const pick = async (f) => f && loadFile(f.name, await f.arrayBuffer());
$('#file').onchange = (e) => pick(e.target.files[0]);
drop.addEventListener('dragover', (e) => { e.preventDefault(); drop.classList.add('over'); });
drop.addEventListener('dragleave', () => drop.classList.remove('over'));
drop.addEventListener('drop', (e) => { e.preventDefault(); drop.classList.remove('over'); pick(e.dataTransfer.files[0]); });
drop.addEventListener('keydown', (e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); $('#file').click(); } });
$('#sample').onclick = async (e) => {
  e.preventDefault();
  e.stopPropagation();
  const r = await fetch('ornek/ornek-flans-braket.dxf');
  loadFile('ornek-flans-braket.dxf', await r.arrayBuffer());
};
startCoord();
