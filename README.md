# Nestio

Plakadan parça kesenler için ücretsiz, açık kaynak dizme (nesting) aracı.
DXF ya da DWG çizimini alır, parçaları plakaya en az plaka ve en az fireyle
dizer, kesime hazır DXF verir.

- **Ücretsiz:** ödeme yok, üyelik yok, sınır yok.
- **Çiziminiz bilgisayarınızdan çıkmaz:** hesap tarayıcıda, sizin işlemcinizde
  (WebAssembly + Web Worker) yapılır. Sayfa yalnızca kendi dosyalarını yükler;
  çerez, izleme kodu ya da hesap yoktur.
- **Açık kaynak:** GPL-3.0-or-later.

## Ne yapar

1. **Parçaları ayırır.** Çizgi, yay, çember, elips, spline, çoklu çizgi ve
   blokları (INSERT) kapalı konturlara çevirir; iç içe konturlardan delikleri
   bulur. 3B tel kafes çizilmiş levha parçalarını düzleme indirir (levhanın
   normali, parçayı en ince gösteren yön olarak bulunur). Kalınlığı
   çoğunluktan farklı ya da yüzü çözülemeyen parçalar işaretlenir.
2. **Plakaları bulur.** Adında `PLAKA`, `PLATE`, `SHEET`, `LEVHA`, `SAC`
   geçen katmandaki dikdörtgenler ve parçalardan belirgin büyük dikdörtgenler
   plaka sayılır. Ölçü ve adet arayüzden değiştirilebilir, yeni plaka
   eklenebilir; farklı ölçülerde plakalar birlikte kullanılabilir.
3. **Delikleri değerlendirir.** Büyük parçaların boşluklarına sığan küçük
   parçalar, boşluk payı korunarak oraya yerleştirilir (ızgarada FFT ile
   tarama, ardından kesin geometriyle doğrulama).
4. **Plakaları doldurur.** [sparrow](https://github.com/JeroenGar/sparrow)
   şerit doldurma algoritması parçaları serbest açıyla döndürerek sıkıştırır
   (aynalama yok). Plaka plaka ilerler; en sonda son plakadaki parçalar önceki
   plakalara taşınmaya çalışılır ve son plaka sıkıştırılır, böylece boş alan
   plakanın ucunda tek parça artık olarak kalır. Birden çok çekirdek varsa
   aynı iş farklı tohumlarla paralel çözülür, en iyisi seçilir.
5. **Denetler.** Sonuç çözücüden bağımsız bir denetimden geçer: her parça
   kenar payı içinde mi, iki parça arası en az istenen boşluk kadar mı.
6. **DXF yazar.** Katmanlar: `PLAKA`, `KESIM`, `ETIKET`, `ARTIK`.

Ayarlar: parçalar arası boşluk ve kenar payı (0 dahil, varsayılan 10 mm),
plaka başına arama süresi, delik doldurma ve son plakayı boşaltma.

## Kullanım

### Tarayıcı

`web/` klasörü durağan bir sitedir; herhangi bir durağan sunucuyla yayınlanır:

```bash
python3 -m http.server 8765 --directory web
```

Sonra `http://localhost:8765` adresini açın. (Modül Web Worker'ları
`file://` üzerinden çalışmaz; bir sunucu gerekir.)

### Komut satırı

Aynı çekirdek komut satırından da çalışır (ölçüm ve toplu iş için):

```bash
cargo run --release -p nestio-cli -- analiz cizim.dxf
cargo run --release -p nestio-cli -- diz cizim.dxf --plaka 3000x1500:4 --bosluk 10 --pay 10 --sure 60 --cikti sonuc.dxf
```

`--plaka GENxYUK[:adet]` birden çok kez verilebilir; verilmezse çizimdeki
plakalar kullanılır. `--is` paralel iş sayısı, `--delik 0` delik doldurmayı,
`--iyilestir 0` son plaka iyileştirmesini kapatır.

## Derleme

Gerekenler: Rust (1.90+, `wasm32-unknown-unknown` hedefi) ve
`wasm-bindgen-cli` 0.2.130.

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130
./build.sh          # web/pkg altına WASM ve JS yapıştırıcısını yazar
cargo test          # çekirdek testleri
```

Derlenmiş `web/pkg` depoya dahildir, böylece site Rust kurmadan yayınlanabilir.

## Yapı

```
crates/core   Rust çekirdeği: çizim modeli, DXF okuma, eğri düzleştirme, kontur
              çıkarma (planar), 3B → 2B (solid3d), parça analizi, delik doldurma
              (holes), plaka döngüsü (nest), şerit çözücü (strip, sparrow),
              denetim (verify), DXF yazma
crates/wasm   Tarayıcı bağlaması (wasm-bindgen)
crates/cli    Komut satırı
web/          Arayüz: index.html, style.css, js/app.js (arayüz), js/i18n.js
              (dil: EN varsayılan, ?lang=tr / ?lang=et), js/coord.js
              (yönetici işçi), js/pool.js (çözücü işçileri), js/dwg.js (DWG okuyucu)
examples/     Sentetik örnek çizim ve üreticisi
```

DWG, tarayıcıda [LibreDWG](https://www.gnu.org/software/libredwg/)'nin
WebAssembly derlemesiyle ([libredwg-web](https://github.com/mlightcad/libredwg-web))
okunup Nestio'nun çizim modeline çevrilir. Çıktı yalnızca DXF'tir; tüm CAD
programları açar.

## Sınırlar

- Plakalar dikdörtgendir (dikdörtgen olmayan plakada dış çerçevesi kullanılır).
- Çıktıda yaylar kısa doğru parçalarıyla (0,05 mm hata) yazılır.
- Parçaların içindeki iç çizgiler (büküm, gravür) aynı katmandaysa kontur
  sayılabilir; böyle katmanları arayüzden kapatın.

## Lisans

GPL-3.0-or-later; bkz. [LICENSE](LICENSE). Üçüncü taraf bileşenler:
[LISANSLAR.txt](LISANSLAR.txt). Talivio adı ve logosu Talivio Technology OÜ'nün
ticari markasıdır ve GPL kapsamında değildir.

Algoritma: J. Gardeyn, G. Vanden Berghe, T. Wauters, *An open-source heuristic
to reboot 2D nesting research*, 2025 — [arXiv:2509.13329](https://doi.org/10.48550/arXiv.2509.13329).

---

**English:** Nestio is a free, open-source nesting tool for sheet cutting.
Drop a DXF or DWG; it extracts the parts (holes included, 3D wireframe sheets
flattened), fills holes with small parts, nests everything with the
[sparrow](https://github.com/JeroenGar/sparrow) heuristic on as few sheets as
possible, verifies spacing and margins independently, and writes a DXF.
Everything runs locally in your browser (WebAssembly); nothing is uploaded.
