// Üst çubuktaki açılır menüler (dil, tema) ve tema seçimi.
//
// Erişilebilirlik: açılır düğme (disclosure) kalıbı — düğme aria-expanded
// taşır, menü hidden ile açılıp kapanır. Ok tuşları seçenekler arasında
// gezer, Esc ve dışarı tıklama kapatır, Esc odağı düğmeye geri verir.
//
// Tema: localStorage 'theme' = 'light' | 'dark'; yoksa sistem tercihi izlenir
// (Talivio ürünlerindeki anahtarla aynı). Sayfanın ilk boyaması index.html'deki
// baş betiğinde yapılır; burası yalnız sonraki değişiklikleri uygular.
import './i18n.js?v=20261105b';

const root = document.documentElement;
const media = matchMedia('(prefers-color-scheme: dark)');

function storedTheme() {
  try { return localStorage.getItem('theme') ?? 'system'; } catch { return 'system'; }
}

function applyTheme(mode) {
  try {
    if (mode === 'system') localStorage.removeItem('theme');
    else localStorage.setItem('theme', mode);
  } catch { /* özel pencere: yalnız bu sayfa için uygulanır */ }
  const dark = mode === 'dark' || (mode === 'system' && media.matches);
  const changed = root.dataset.theme !== (dark ? 'dark' : 'light');
  root.dataset.theme = dark ? 'dark' : 'light';
  syncTheme(mode);
  // Tuval çizimleri renklerini CSS değişkenlerinden okur; tema değişince yeniden çizilsin.
  if (changed) window.dispatchEvent(new Event('nestio-theme'));
}

function syncTheme(mode) {
  const menu = document.getElementById('theme-menu');
  if (!menu) return;
  let cur = null;
  for (const b of menu.querySelectorAll('[data-theme-set]')) {
    const on = b.dataset.themeSet === mode;
    b.setAttribute('aria-pressed', String(on));
    if (on) cur = b;
  }
  const slot = document.querySelector('[aria-controls="theme-menu"] [data-dd-cur]');
  if (slot && cur) slot.innerHTML = cur.querySelector('svg').outerHTML + `<span class="dd-label">${cur.querySelector('span').textContent}</span>`;
}

media.addEventListener('change', () => { if (storedTheme() === 'system') applyTheme('system'); });

for (const b of document.querySelectorAll('[data-theme-set]')) {
  b.addEventListener('click', () => applyTheme(b.dataset.themeSet));
}
syncTheme(storedTheme());

// dil düğmesinde seçili dilin adı
{
  const cur = document.querySelector(`#lang-menu [data-lang="${root.lang}"]`);
  const slot = document.querySelector('[aria-controls="lang-menu"] [data-dd-cur]');
  if (cur && slot) slot.innerHTML = `<span class="dd-label">${cur.querySelector('span').textContent}</span><span class="dd-code">${root.lang.toUpperCase()}</span>`;
}

// ── açılır menü davranışı ───────────────────────────────────────────────────
const items = (menu) => [...menu.querySelectorAll('a, button')];

function close(dd, focusBtn = false) {
  const btn = dd.querySelector('.dd-btn');
  const menu = dd.querySelector('.dd-menu');
  if (menu.hidden) return;
  menu.hidden = true;
  btn.setAttribute('aria-expanded', 'false');
  if (focusBtn) btn.focus();
}

function open(dd) {
  for (const other of document.querySelectorAll('[data-dd]')) if (other !== dd) close(other);
  const btn = dd.querySelector('.dd-btn');
  const menu = dd.querySelector('.dd-menu');
  menu.hidden = false;
  btn.setAttribute('aria-expanded', 'true');
  const list = items(menu);
  (list.find((el) => el.getAttribute('aria-current') === 'true' || el.getAttribute('aria-pressed') === 'true') ?? list[0]).focus();
}

for (const dd of document.querySelectorAll('[data-dd]')) {
  const btn = dd.querySelector('.dd-btn');
  const menu = dd.querySelector('.dd-menu');
  btn.addEventListener('click', () => (menu.hidden ? open(dd) : close(dd)));
  btn.addEventListener('keydown', (e) => { if (e.key === 'ArrowDown') { e.preventDefault(); open(dd); } });
  menu.addEventListener('keydown', (e) => {
    const list = items(menu);
    const i = list.indexOf(document.activeElement);
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      list[(i + (e.key === 'ArrowDown' ? 1 : -1) + list.length) % list.length].focus();
    } else if (e.key === 'Home' || e.key === 'End') {
      e.preventDefault();
      list[e.key === 'Home' ? 0 : list.length - 1].focus();
    } else if (e.key === 'Tab') {
      close(dd);
    }
  });
  menu.addEventListener('click', (e) => { if (e.target.closest('button')) close(dd, true); });
}

document.addEventListener('keydown', (e) => {
  if (e.key !== 'Escape') return;
  for (const dd of document.querySelectorAll('[data-dd]')) if (!dd.querySelector('.dd-menu').hidden) close(dd, true);
});
document.addEventListener('click', (e) => {
  for (const dd of document.querySelectorAll('[data-dd]')) if (!dd.contains(e.target)) close(dd);
});
