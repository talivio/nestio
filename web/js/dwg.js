// DWG okuyucu (LibreDWG, GPL-3.0). Yalnız DWG seçildiğinde yüklenir.
// LibreDWG'nin veri modelini Nestio'nun biçimden bağımsız çizim modeline çevirir.
import { LibreDwg, Dwg_File_Type } from '../vendor/libredwg/dist/libredwg-web.js';

let lib = null;
const deg = (r) => (r * 180) / Math.PI;
const p3 = (p) => [p?.x ?? 0, p?.y ?? 0, p?.z ?? 0];
const nz = (p) => (p && (p.x || p.y || p.z) ? p3(p) : [0, 0, 1]);

// $INSUNITS → mm çarpanı
const UNITS = { 1: 25.4, 2: 304.8, 4: 1, 5: 10, 6: 1000, 14: 100 };

function conv(e) {
  if (e.isInPaperSpace) return null;
  const layer = e.layer ?? '0';
  switch (e.type) {
    case 'LINE':
      return { layer, t: 'line', a: p3(e.startPoint), b: p3(e.endPoint) };
    case 'ARC':
      return { layer, t: 'arc', c: p3(e.center), r: e.radius, a0: deg(e.startAngle), a1: deg(e.endAngle), n: nz(e.extrusionDirection) };
    case 'CIRCLE':
      return { layer, t: 'circle', c: p3(e.center), r: e.radius, n: nz(e.extrusionDirection) };
    case 'LWPOLYLINE':
      return {
        layer, t: 'poly', closed: (e.flag & 512) !== 0 || (e.flag & 1) !== 0, n: nz(e.extrusionDirection),
        v: e.vertices.map((v) => [v.x, v.y, e.elevation ?? 0, v.bulge ?? 0]),
      };
    case 'POLYLINE2D':
      if (e.flag & (16 | 64)) return null;
      return {
        layer, t: 'poly', closed: (e.flag & 1) !== 0, n: nz(e.extrusionDirection),
        v: e.vertices.filter((v) => !(v.flag & 16)).map((v) => [v.x, v.y, e.elevation ?? 0, v.bulge ?? 0]),
      };
    case 'POLYLINE3D':
      return { layer, t: 'poly', closed: (e.flag & 1) !== 0, n: [0, 0, 1], v: e.vertices.filter((v) => !(v.flag & 16)).map((v) => [v.x, v.y, v.z ?? 0, 0]) };
    case 'SPLINE':
      return {
        layer, t: 'spline', deg: e.degree, knots: e.knots ?? [], ctrl: (e.controlPoints ?? []).map(p3),
        w: e.weights ?? [], fit: (e.fitPoints ?? []).map(p3), closed: (e.flag & 1) !== 0,
      };
    case 'ELLIPSE':
      return {
        layer, t: 'ellipse', c: p3(e.center), major: p3(e.majorAxisEndPoint), ratio: e.axisRatio,
        p0: e.startAngle, p1: e.endAngle, n: nz(e.extrusionDirection),
      };
    case 'INSERT':
      return {
        layer, t: 'insert', name: e.name, p: p3(e.insertionPoint), s: [e.xScale ?? 1, e.yScale ?? 1, e.zScale ?? 1],
        rot: deg(e.rotation ?? 0), n: nz(e.extrusionDirection), cols: Math.max(1, e.columnCount ?? 1),
        rows: Math.max(1, e.rowCount ?? 1), dc: e.columnSpacing ?? 0, dr: e.rowSpacing ?? 0,
      };
    case 'TEXT':
      return { layer, t: 'text', p: [e.startPoint?.x ?? 0, e.startPoint?.y ?? 0, 0], s: e.text ?? '', n: nz(e.extrusionDirection) };
    case 'MTEXT':
      return { layer, t: 'text', p: p3(e.insertionPoint), s: e.text ?? '', n: nz(e.extrusionDirection) };
    default:
      return null;
  }
}

function toDrawing(db) {
  const blocks = {};
  for (const b of db.tables?.BLOCK_RECORD?.entries ?? []) {
    if (!b.name || /^\*(model|paper)_space/i.test(b.name)) continue;
    blocks[b.name] = { base: p3(b.basePoint), entities: (b.entities ?? []).map(conv).filter(Boolean) };
  }
  return { units: UNITS[db.header?.INSUNITS] ?? 1, entities: (db.entities ?? []).map(conv).filter(Boolean), blocks };
}

self.onmessage = async (ev) => {
  try {
    if (!lib) lib = await LibreDwg.create(new URL('../vendor/libredwg/wasm', import.meta.url).href);
    const ptr = lib.dwg_read_data(ev.data.bytes, Dwg_File_Type.DWG);
    if (ptr === undefined) throw new Error('DWG okunamadı');
    const db = lib.convert(ptr);
    lib.dwg_free(ptr);
    self.postMessage({ ok: true, drawing: JSON.stringify(toDrawing(db)) });
  } catch (e) {
    self.postMessage({ ok: false, error: String(e?.message ?? e) });
  }
};
