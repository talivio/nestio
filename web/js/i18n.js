// Nestio dil katmanı. Varsayılan İngilizce; Türkçe ve Estonca seçilebilir.
// Dil sırası: ?lang= → kayıtlı seçim (localStorage 'nestio-lang') → en.
// Sayfadaki sabit metin index.html'de İngilizcedir (JS'siz de okunur); diğer
// diller data-i18n* anahtarlarıyla buradan uygulanır. Arayüzün kendi metinleri
// (app.js) t() ile buradan gelir.
//
// ⚠️ Motor (Rust) ilerleme ve denetim satırlarını TÜRKÇE üretir. Motora
// dokunmadan, bilinen kalıplar tm() ile çevrilir; tanınmayan satır olduğu
// gibi gösterilir. Motor bir gün mesaj kodu döndürürse bu tablo kalkar.

const LANGS = ['en', 'tr', 'et'];

function pick() {
  const q = new URLSearchParams(location.search).get('lang');
  if (LANGS.includes(q)) return q;
  let s = null;
  try { s = localStorage.getItem('nestio-lang'); } catch { /* özel pencere */ }
  return LANGS.includes(s) ? s : 'en';
}

export const lang = pick();
try {
  const q = new URLSearchParams(location.search).get('lang');
  if (LANGS.includes(q)) localStorage.setItem('nestio-lang', q);
} catch { /* yok say */ }

export const numLocale = { en: 'en-US', tr: 'tr-TR', et: 'et-EE' }[lang];

// ── sözlük ───────────────────────────────────────────────────────────────────
// en: yalnız app.js'in kullandığı anahtarlar (sayfa metni index.html'de).
const D = {
  en: {
    'app.workerFail': 'The calculation worker could not start: {e}',
    'app.workerFailHint': 'your browser may not support module workers',
    'app.dwgFail': 'The DWG reader could not load',
    'app.reading': 'Reading the drawing…',
    'app.readFail': 'The drawing could not be read: {e}',
    'app.extracting': 'Extracting parts…',
    'app.noContours': 'No closed contours found in the drawing. Check the layers and that the drawing is closed.',
    'app.partsum': '{n} pcs · {a} m²',
    'app.units': 'The drawing unit is not mm; sizes were converted to millimetres with a factor of {u}×.',
    'app.warned': '{n} parts have warnings (tiles with an exclamation mark); hover to see them.',
    'app.manual': 'You changed: {list}.',
    'app.attention': 'Note',
    'app.resetRoles': 'Undo manual changes',
    'role.part': 'Part', 'role.plate': 'Sheet', 'role.ignore': 'Ignore',
    'role.partL': 'part', 'role.plateL': 'sheet', 'role.ignoreL': 'ignored',
    'app.typeOf': '{name} type',
    'app.thickness': 'thickness {t} mm',
    'app.qty': 'Quantity', 'app.type': 'Type',
    'app.lengthMm': 'Length (mm)', 'app.widthMm': 'Width (mm)', 'app.delete': 'Delete',
    'app.noParts': 'There are no parts to nest.',
    'app.noSheets': 'Enter at least one sheet size.',
    'app.nesting': 'Nesting…',
    'app.nestingReady': 'Nesting… {n} sheets ready',
    'app.nestFail': 'Nesting failed: {e}',
    'app.stopped': 'Stopped.',
    'app.sheet': 'Sheet {k}',
    'app.sheetLine': '{w} × {h} mm · {n} parts · fill {f}% · used length {u} mm',
    'app.remnant': ' · remnant {r} × {h} mm',
    'stat.sheets': 'sheets', 'stat.fill': 'total fill', 'stat.lastUsed': 'used length on last sheet',
    'stat.placed': 'parts placed', 'stat.holes': 'inside holes', 'stat.time': 'time',
    'app.checkOk': 'Check passed',
    'app.checkBad': '{n} check errors',
    'app.unplaced': '{n} parts were not placed: ',
    'app.fileSuffix': 'nested',
    'app.pct': '{v}%',
  },
  tr: {
    'meta.title': 'Nestio — ücretsiz plaka dizme (nesting), tarayıcıda',
    'meta.description': 'DXF ya da DWG çiziminizdeki parçaları plakaya en az fireyle dizen ücretsiz, açık kaynak araç. Kurulum ve üyelik yok; hesap tarayıcınızda yapılır, çiziminiz bilgisayarınızdan çıkmaz.',
    'theme.label': 'Tema', 'theme.light': 'Açık', 'theme.dark': 'Koyu', 'theme.system': 'Sistem',
    'nav.skip': 'Araca geç', 'nav.home': 'Nestio, ana sayfa', 'nav.label': 'Ana menü', 'nav.how': 'Nasıl çalışır', 'nav.faq': 'SSS', 'nav.lang': 'Dil',
    'hero.eyebrow': 'Ücretsiz · Açık kaynak · Kurulumsuz',
    'hero.title': 'Parçaları plakaya en az fireyle dizin',
    'hero.lead': 'DXF ya da DWG çiziminizi bırakın. Nestio parçaları ayırır, deliklerin içini değerlendirir, plaka sayısını en aza indirir ve kesime hazır DXF verir.',
    'hero.try': 'Örnek çizimle dene', 'hero.pick': 'Çizimimi seç',
    'hero.proof': 'Dosyanız hiçbir sunucuya gönderilmez. Hesap bu bilgisayarın işlemcisinde yapılır.',
    'drop.title': 'Çizimi buraya bırakın', 'drop.hint': 'ya da tıklayıp seçin · DXF, DWG', 'drop.foot': 'Üyelik yok · Sınır yok · Çerez ve izleme kodu yok',
    'setup.parts': 'Parçalar', 'legend.part': 'Parça', 'legend.plate': 'Plaka', 'legend.ignore': 'Yok sayılan', 'legend.hint': 'Tür, karenin altındaki menüden değişir',
    'setup.listLayers': 'Liste görünümü ve katmanlar', 'th.name': 'Ad', 'th.size': 'Ölçü (mm)', 'th.qty': 'Adet', 'th.type': 'Tür',
    'setup.layers': 'Katmanlar', 'setup.layersHint': 'İşareti kaldırılan katman okunmaz (yazı, ölçü, çerçeve katmanları gibi).',
    'setup.settings': 'Ayarlar', 'setup.sheets': 'Plakalar', 'th.length': 'Boy', 'th.width': 'En', 'setup.addSheet': 'Plaka ekle', 'setup.qtyHint': 'Adet boşsa sınırsız.',
    'setup.gap': 'Parçalar arası boşluk (mm)', 'setup.margin': 'Kenar payı (mm)', 'setup.time': 'Arama süresi (plaka başına)',
    'setup.fast': 'Hızlı · 15 sn', 'setup.balanced': 'Dengeli · 45 sn', 'setup.thorough': 'Titiz · 2 dk',
    'setup.holes': 'Deliklerin içine küçük parça yerleştir', 'setup.improve': 'Son plakayı boşaltmayı dene (artık tek parça kalsın)',
    'setup.cores': 'Döndürme serbest, aynalama yok. Hesap bu bilgisayarın şu kadar çekirdeğiyle yapılır:', 'setup.run': 'Diz',
    'work.title': 'Diziliyor…', 'work.stop': 'Durdur', 'result.title': 'Sonuç', 'result.download': 'DXF indir', 'result.back': 'Ayarlara dön',
    'tour.eyebrow': "Nestio'nun içi", 'tour.title': 'Çizimden kesime üç ekran',
    'tour.sub': 'Aşağıdaki ekranlar aracın kendisinden alındı; örnek çizim sentetiktir ve “Örnek çizimle dene” düğmesiyle aynı sonucu kendiniz alabilirsiniz.',
    'tour.readEyebrow': '1 · Oku', 'tour.readTitle': 'Parçaları kendisi ayırır',
    'tour.readText': 'Çizgi, yay, spline, çoklu çizgi ve blokları kapalı konturlara çevirir; iç içe konturlardan delikleri bulur. Aynı parçaları sayıp adet yazar, plakaları katman adından tanır.',
    'tour.r1t': 'DXF ve DWG', 'tour.r1': 'DWG tarayıcıda LibreDWG ile okunur, dönüştürmek gerekmez.',
    'tour.r2t': '3B tel kafes levhalar', 'tour.r2': 'Levha parçaları düzleme indirilir; kalınlığı farklı olanlar işaretlenir.',
    'tour.r3t': 'Kolay düzeltme', 'tour.r3': 'Her karenin menüsünden parça, plaka ya da yok sayılan seçilir.',
    'tour.winParts': 'nestio.talivio.com · Parçalar', 'tour.winSettings': 'nestio.talivio.com · Ayarlar',
    'tour.altParts': 'Parçalar ekranı: örnek çizimden ayrılmış 46 parça, iki 3000×1500 plaka; flanş, çerçeve, köşebent, üçgen, pul, lama ve dalgalı parça kareleri adetleriyle.',
    'tour.caption': 'Gerçek ekran · sentetik örnek çizim',
    'tour.setEyebrow': '2 · Ayarla', 'tour.setTitle': 'Plakayı, boşluğu ve süreyi siz seçin',
    'tour.setText': 'Farklı ölçülerde plakaları birlikte kullanın, adet sınırı koyun ya da boş bırakın. Parça arası boşluk ve kenar payı milimetreyle girilir; 0 da geçerlidir.',
    'tour.s1t': 'Üç arama süresi', 'tour.s1': 'Hızlı 15 sn, Dengeli 45 sn, Titiz 2 dk (plaka başına).',
    'tour.s2t': 'Delik içine yerleştirme', 'tour.s2': 'Küçük parçalar büyüklerin boşluklarına, boşluk payı korunarak girer.',
    'tour.s3t': 'Son plakayı boşaltma', 'tour.s3': 'Artık, son plakanın ucunda tek parça olarak kalır.',
    'tour.altSettings': 'Ayarlar ekranı: plaka tablosu, parça arası boşluk ve kenar payı 10 mm, arama süresi seçimi, delik içine yerleştirme ve son plakayı boşaltma seçenekleri, Diz düğmesi.',
    'flow.title': 'Nasıl çalışır',
    'flow.1t': 'Parçaları ayırır', 'flow.1': 'Kapalı konturlar ve delikler bulunur, aynı parçalar sayılır.',
    'flow.2t': 'Delikleri değerlendirir', 'flow.2': 'Boşluklara sığan küçük parçalar FFT taramasıyla bulunur, kesin geometriyle doğrulanır.',
    'flow.3t': 'Plakaları doldurur', 'flow.3': 'Açık kaynak <a href="https://github.com/JeroenGar/sparrow">sparrow</a> algoritması parçaları serbest açıyla döndürerek sıkıştırır. Çekirdek sayısı kadar paralel deneme yapılır, en iyisi seçilir.',
    'flow.4t': 'Denetler ve yazar', 'flow.4': 'Bağımsız bir denetim kenar payını ve aralıkları ölçer; sonuç katmanlı DXF olarak iner.',
    'out.eyebrow': 'Çıktı', 'out.title': 'Kesime hazır DXF, katman katman',
    'out.text': 'Dizilim, CAM yazılımınızın doğrudan açabileceği bir DXF olarak iner. Plaka sınırı, kesim konturları, parça etiketleri ve artık parça ayrı katmanlardadır.',
    'out.l1': 'Plaka sınırı', 'out.l2': 'Parça ve delik konturları', 'out.l3': 'Parça adları', 'out.l4': 'Plakanın ucunda kalan tek parça',
    'out.win': 'Plaka 1 · 3.000 × 1.500 mm',
    'out.alt': 'Dizilmiş plaka: 46 parça 3000×1500 mm plakanın sol tarafına sıkıştırılmış, küçük pullar flanşların ve çerçevelerin deliklerine yerleştirilmiş, sağda tek parça artık alan.',
    'out.caption': "Gerçek çıktı · sentetik örnek çizim, “Hızlı” ayar: 46/46 parça, 18'i delik içinde, denetim hatasız, 40 sn",
    'why.title': 'Neden Nestio',
    'why.1t': 'Ücretsiz, sınırsız', 'why.1': 'Ödeme, üyelik, parça ya da plaka sınırı yok. Filigran da yok.',
    'why.2t': 'Çiziminiz sizde kalır', 'why.2': 'Hesap tarayıcıda, WebAssembly ile yapılır. Sayfa yalnız kendi dosyalarını yükler; çerez ve izleme kodu yoktur.',
    'why.3t': 'Açık kaynak', 'why.3': "Kodun tamamı GPL-3.0 ile GitHub'da. İsterseniz kendi sunucunuzda ya da komut satırında çalıştırın.",
    'why.4t': 'Kurulum yok', 'why.4': 'Güncel bir tarayıcı yeter. Çok çekirdekli bilgisayarda aynı iş paralel denenir.',
    'faq.title': 'Sık sorulanlar',
    'faq.1q': 'Çizimim bir sunucuya gidiyor mu?', 'faq.1a': 'Hayır. Dosya tarayıcınızda okunur ve dizilir; sayfa açıldıktan sonra yapılan tek ağ isteği aracın kendi dosyalarıdır (WASM, yazı tipleri). Bunu tarayıcının geliştirici araçlarındaki Ağ sekmesinden doğrulayabilirsiniz.',
    'faq.2q': 'DWG dosyası da yazabiliyor mu?', 'faq.2a': "DWG okunur, çıktı DXF'tir. Tarayıcıda DWG yazabilen açık kaynak bir kitaplık henüz yok; DXF'i tüm CAM yazılımları açar.",
    'faq.3q': 'Hangi plaka ölçülerini kullanabilirim?', 'faq.3a': 'İstediğiniz kadar farklı ölçü ekleyebilirsiniz; her ölçüye adet sınırı koyabilir ya da sınırsız bırakabilirsiniz. Çizimde <code>PLAKA</code>, <code>SHEET</code>, <code>LEVHA</code>, <code>SAC</code> gibi katmandaki dikdörtgenler kendiliğinden plaka sayılır.',
    'faq.4q': 'Parçaları aynalıyor mu?', 'faq.4a': 'Hayır. Parçalar serbest açıyla döndürülür ama aynalanmaz; böylece yönlü parçalar ve kaplamalı saclar güvende kalır.',
    'faq.5q': 'Sonucun doğru olduğunu nasıl bilirim?', 'faq.5a': 'Her sonuç, dizme algoritmasından bağımsız bir denetimden geçer: her parça kenar payının içinde mi, iki parça arası en az istenen boşluk kadar mı. Sonuç ekranında “Denetim hatasız” ya da bulunan hatalar yazar.',
    'faq.6q': 'Neden ücretsiz?', 'faq.6a': "Nestio, Talivio'nun açık kaynak araçlarındandır. Satılan bir sürümü, sınırlı bir katmanı ya da reklamı yoktur. Hata ve öneriler için GitHub'da kayıt açabilirsiniz.",
    'end.title': "Çiziminizi bırakın, kesime hazır DXF'i alın",
    'foot.tagline': 'Tarayıcıda çalışan ücretsiz, açık kaynak plaka dizme aracı.',
    'foot.product': 'Ürün', 'foot.source': 'Kaynak kodu', 'foot.blog': 'Tanıtım yazısı', 'foot.blogHref': 'https://talivio.com/tr/blog/nestio-free-open-source-nesting-in-your-browser', 'foot.licenses': 'Lisanslar',
    'foot.legal': 'Yasal', 'foot.privacy': 'Gizlilik', 'foot.terms': 'Kullanım şartları', 'foot.imprint': 'Künye',
    'foot.copy': '© 2026 Talivio Technology OÜ · Nestio GPL-3.0-or-later; Talivio adı ve işareti lisansın dışındadır.', 'foot.by': 'Yapımcı: Talivio',
    'img.parts': 'img/ekran-parcalar-tr.webp', 'img.partsDark': 'img/ekran-parcalar-tr-dark.webp', 'img.settings': 'img/ekran-ayarlar-tr.webp', 'img.settingsDark': 'img/ekran-ayarlar-tr-dark.webp',

    'app.workerFail': 'Hesap işçisi başlatılamadı: {e}',
    'app.workerFailHint': 'tarayıcınız modül işçilerini desteklemiyor olabilir',
    'app.dwgFail': 'DWG okuyucu yüklenemedi',
    'app.reading': 'Çizim okunuyor…',
    'app.readFail': 'Çizim okunamadı: {e}',
    'app.extracting': 'Parçalar çıkarılıyor…',
    'app.noContours': 'Çizimde kapalı kontur bulunamadı. Katmanları ve çizimin kapalı olduğunu kontrol edin.',
    'app.partsum': '{n} adet · {a} m²',
    'app.units': 'Çizim birimi mm değil; ölçüler {u}× çarpanla milimetreye çevrildi.',
    'app.warned': '{n} parçada uyarı var (ünlemli kareler); üzerine gelince görünür.',
    'app.manual': 'Elle değiştirdiniz: {list}.',
    'app.attention': 'Dikkat',
    'app.resetRoles': 'Elle değişiklikleri geri al',
    'role.part': 'Parça', 'role.plate': 'Plaka', 'role.ignore': 'Yok say',
    'role.partL': 'parça', 'role.plateL': 'plaka', 'role.ignoreL': 'yok sayıldı',
    'app.typeOf': '{name} türü',
    'app.thickness': 'kalınlık {t} mm',
    'app.qty': 'Adet', 'app.type': 'Tür',
    'app.lengthMm': 'Boy (mm)', 'app.widthMm': 'En (mm)', 'app.delete': 'Sil',
    'app.noParts': 'Dizilecek parça yok.',
    'app.noSheets': 'En az bir plaka ölçüsü girin.',
    'app.nesting': 'Diziliyor…',
    'app.nestingReady': 'Diziliyor… {n} plaka hazır',
    'app.nestFail': 'Dizme başarısız: {e}',
    'app.stopped': 'Durduruldu.',
    'app.sheet': 'Plaka {k}',
    'app.sheetLine': '{w} × {h} mm · {n} parça · doluluk %{f} · kullanılan boy {u} mm',
    'app.remnant': ' · artık {r} × {h} mm',
    'stat.sheets': 'plaka', 'stat.fill': 'toplam doluluk', 'stat.lastUsed': 'son plakada kullanılan boy',
    'stat.placed': 'yerleşen parça', 'stat.holes': 'delik içinde', 'stat.time': 'süre',
    'app.checkOk': 'Denetim hatasız',
    'app.checkBad': '{n} denetim hatası',
    'app.unplaced': '{n} parça yerleşmedi: ',
    'app.fileSuffix': 'dizilim',
    'app.pct': '%{v}',
  },
  et: {
    'meta.title': 'Nestio — tasuta lehtmaterjali paigutus brauseris',
    'meta.description': 'Tasuta avatud lähtekoodiga tööriist, mis paigutab DXF- või DWG-joonise detailid lehtedele võimalikult väikese jäägiga. Ei vaja paigaldust ega kontot: kõik arvutused tehakse brauseris ja joonis ei lahku sinu arvutist.',
    'theme.label': 'Teema', 'theme.light': 'Hele', 'theme.dark': 'Tume', 'theme.system': 'Süsteem',
    'nav.skip': 'Mine tööriista juurde', 'nav.home': 'Nestio, avaleht', 'nav.label': 'Peamenüü', 'nav.how': 'Kuidas see töötab', 'nav.faq': 'KKK', 'nav.lang': 'Keel',
    'hero.eyebrow': 'Tasuta · Avatud lähtekood · Paigaldust pole vaja',
    'hero.title': 'Paiguta detailid lehtedele võimalikult väikese jäägiga',
    'hero.lead': 'Lohista oma DXF- või DWG-joonis siia. Nestio eraldab detailid, kasutab ära nende sees olevad avad, hoiab lehtede arvu väikese ja annab lõikamiseks valmis DXF-faili.',
    'hero.try': 'Proovi näidisjoonisega', 'hero.pick': 'Vali oma joonis',
    'hero.proof': 'Sinu faili ei saadeta kunagi serverisse. Arvutused teeb selle arvuti protsessor.',
    'drop.title': 'Lohista joonis siia', 'drop.hint': 'või klõpsa ja vali · DXF, DWG', 'drop.foot': 'Kontot pole · Piiranguid pole · Küpsiseid ega jälgijaid pole',
    'setup.parts': 'Detailid', 'legend.part': 'Detail', 'legend.plate': 'Leht', 'legend.ignore': 'Eiratud', 'legend.hint': 'Tüüpi muudad iga ruudu all olevast menüüst',
    'setup.listLayers': 'Loendivaade ja kihid', 'th.name': 'Nimi', 'th.size': 'Mõõt (mm)', 'th.qty': 'Kogus', 'th.type': 'Tüüp',
    'setup.layers': 'Kihid', 'setup.layersHint': 'Märkimata kihte ei loeta (näiteks teksti-, mõõtme- või raamikihid).',
    'setup.settings': 'Seaded', 'setup.sheets': 'Lehed', 'th.length': 'Pikkus', 'th.width': 'Laius', 'setup.addSheet': 'Lisa leht', 'setup.qtyHint': 'Tühi kogus tähendab piiramatut.',
    'setup.gap': 'Vahe detailide vahel (mm)', 'setup.margin': 'Servavaru (mm)', 'setup.time': 'Otsinguaeg (lehe kohta)',
    'setup.fast': 'Kiire · 15 s', 'setup.balanced': 'Tasakaalus · 45 s', 'setup.thorough': 'Põhjalik · 2 min',
    'setup.holes': 'Paiguta väikesed detailid avadesse', 'setup.improve': 'Proovi viimane leht tühjendada (jääk jääb ühes tükis)',
    'setup.cores': 'Vaba pööramine, peegeldamist pole. Arvutus kasutab selle arvuti tuumi:', 'setup.run': 'Paiguta',
    'work.title': 'Paigutan…', 'work.stop': 'Peata', 'result.title': 'Tulemus', 'result.download': 'Laadi DXF alla', 'result.back': 'Tagasi seadetesse',
    'tour.eyebrow': 'Nestio seest', 'tour.title': 'Joonisest lõikamiseni kolmel ekraanil',
    'tour.sub': 'Need ekraanid on võetud tööriistast endast. Näidisjoonis on sünteetiline; nupp „Proovi näidisjoonisega“ annab sama tulemuse.',
    'tour.readEyebrow': '1 · Loe', 'tour.readTitle': 'Detailid eraldatakse sinu eest',
    'tour.readText': 'Jooned, kaared, splainid, polüjooned ja plokid muudetakse suletud kontuurideks; üksteise sees olevad kontuurid muutuvad avadeks. Ühesugused detailid loendatakse ja lehed tuntakse ära kihi nime järgi.',
    'tour.r1t': 'DXF ja DWG', 'tour.r1': 'DWG loetakse brauseris LibreDWG abil, teisendamist pole vaja.',
    'tour.r2t': '3D-traatmudeli plaadid', 'tour.r2': 'Plaatdetailid laotakse tasapinnale; erineva paksusega detailid märgitakse.',
    'tour.r3t': 'Lihtne parandamine', 'tour.r3': 'Iga ruudu menüüst saab selle muuta detailiks, leheks või eiratuks.',
    'tour.winParts': 'nestio.talivio.com · Detailid', 'tour.winSettings': 'nestio.talivio.com · Seaded',
    'tour.altParts': 'Detailide ekraan: näidisjooniselt eraldatud 46 detaili ja kaks 3000×1500 lehte; äärik, raam, nurk, kolmnurk, seib, lattraud ja lainjas detail koos kogustega.',
    'tour.caption': 'Päris ekraan · sünteetiline näidisjoonis',
    'tour.setEyebrow': '2 · Seadista', 'tour.setTitle': 'Lehed, vahe ja aeg valid ise',
    'tour.setText': 'Kasuta eri suurusega lehti koos, piira nende kogust või jäta lahtiseks. Detailide vahe ja servavaru sisestatakse millimeetrites; ka 0 on lubatud.',
    'tour.s1t': 'Kolm otsinguaega', 'tour.s1': 'Kiire 15 s, Tasakaalus 45 s, Põhjalik 2 min (lehe kohta).',
    'tour.s2t': 'Detailid avadesse', 'tour.s2': 'Väikesed detailid lähevad suurte avadesse, vahe säilib.',
    'tour.s3t': 'Viimase lehe tühjendamine', 'tour.s3': 'Jääk jääb viimase lehe lõppu ühes tükis.',
    'tour.altSettings': 'Seadete ekraan: lehtede tabel, 10 mm vahe ja servavaru, otsinguaja valik, avadesse paigutamise ja viimase lehe tühjendamise valikud, nupp Paiguta.',
    'flow.title': 'Kuidas see töötab',
    'flow.1t': 'Eraldab detailid', 'flow.1': 'Leitakse suletud kontuurid ja avad; ühesugused detailid loendatakse.',
    'flow.2t': 'Kasutab avasid', 'flow.2': 'Avadesse mahtuvad väikesed detailid leitakse FFT-skaneerimisega ja kinnitatakse täpse geomeetriaga.',
    'flow.3t': 'Täidab lehed', 'flow.3': 'Avatud lähtekoodiga <a href="https://github.com/JeroenGar/sparrow">sparrow</a> algoritm pakib detailid vabade pöördenurkadega. Iga protsessorituum teeb ühe katse ja parim valitakse.',
    'flow.4t': 'Kontrollib ja kirjutab', 'flow.4': 'Sõltumatu kontroll mõõdab servavaru ja iga vahe; tulemus laaditakse alla kihilise DXF-failina.',
    'out.eyebrow': 'Väljund', 'out.title': 'Lõikamiseks valmis DXF, kiht kihi haaval',
    'out.text': 'Paigutus laaditakse alla DXF-failina, mille sinu CAM-tarkvara avab otse. Lehe piirjoon, lõikekontuurid, detailide sildid ja jääk on eraldi kihtidel.',
    'out.l1': 'Lehe piirjoon', 'out.l2': 'Detailide ja avade kontuurid', 'out.l3': 'Detailide nimed', 'out.l4': 'Ühes tükis jääk lehe lõpus',
    'out.win': 'Leht 1 · 3000 × 1500 mm',
    'out.alt': 'Paigutatud leht: 46 detaili 3000×1500 mm lehe vasakus osas, väikesed seibid äärikute ja raamide avades, paremal ühes tükis jääk.',
    'out.caption': 'Päris väljund · sünteetiline näidisjoonis, seade „Kiire“: 46/46 detaili, neist 18 avades, kontroll läbitud, 40 s',
    'why.title': 'Miks Nestio',
    'why.1t': 'Tasuta, piiranguteta', 'why.1': 'Maksmist, kontot ega detailide või lehtede piirangut pole. Ka vesimärki pole.',
    'why.2t': 'Joonis jääb sinu juurde', 'why.2': 'Kõik töötab brauseris WebAssembly abil. Leht laadib ainult oma faile; küpsiseid ega jälgijaid pole.',
    'why.3t': 'Avatud lähtekood', 'why.3': 'Kogu kood on GitHubis GPL-3.0 litsentsiga. Soovi korral käivita see oma serveris või käsurealt.',
    'why.4t': 'Paigaldust pole vaja', 'why.4': 'Piisab ajakohasest brauserist. Mitmetuumalises arvutis proovitakse sama tööd paralleelselt.',
    'faq.title': 'Korduma kippuvad küsimused',
    'faq.1q': 'Kas mu joonis saadetakse serverisse?', 'faq.1a': 'Ei. Fail loetakse ja paigutatakse sinu brauseris; pärast lehe avanemist on ainsad võrgupäringud tööriista enda failid (WASM, fondid). Seda saad kontrollida brauseri arendajatööriistade vahekaardil Võrk.',
    'faq.2q': 'Kas see oskab ka DWG-faili kirjutada?', 'faq.2a': 'DWG-faili loetakse, väljund on DXF. Brauseris DWG-d kirjutavat avatud lähtekoodiga teeki veel pole ja DXF-i avab iga CAM-programm.',
    'faq.3q': 'Milliseid lehemõõte saan kasutada?', 'faq.3a': 'Lisa nii palju mõõte kui vaja; igale saab anda koguse või jätta piiramatuks. Ristkülikud kihtidel nagu <code>PLATE</code>, <code>SHEET</code>, <code>PLAKA</code> või <code>LEVHA</code> tuntakse automaatselt lehtedena ära.',
    'faq.4q': 'Kas detaile peegeldatakse?', 'faq.4a': 'Ei. Detailid pöörduvad vabalt, kuid neid ei peegeldata, nii et suunatud detailid ja kattega lehed jäävad korda.',
    'faq.5q': 'Kuidas ma tean, et tulemus on õige?', 'faq.5a': 'Iga tulemus läbib paigutusalgoritmist sõltumatu kontrolli: kas iga detail on servavaru sees ja kas iga vahe on vähemalt nõutud suurusega? Tulemuse ekraanil on kirjas „Kontroll läbitud“ või leitud vead.',
    'faq.6q': 'Miks see tasuta on?', 'faq.6a': 'Nestio on üks Talivio avatud lähtekoodiga tööriistadest. Tasulist versiooni, piiratud paketti ega reklaame pole. Vigadest ja ideedest anna teada GitHubis.',
    'end.title': 'Lohista joonis siia ja saa lõikamiseks valmis DXF',
    'foot.tagline': 'Tasuta avatud lähtekoodiga lehtmaterjali paigutuse tööriist, mis töötab brauseris.',
    'foot.product': 'Toode', 'foot.source': 'Lähtekood', 'foot.blog': 'Tutvustus', 'foot.blogHref': 'https://talivio.com/blog/nestio-free-open-source-nesting-in-your-browser', 'foot.licenses': 'Litsentsid',
    'foot.legal': 'Õiguslik', 'foot.privacy': 'Privaatsus', 'foot.terms': 'Kasutustingimused', 'foot.imprint': 'Impressum',
    'foot.copy': '© 2026 Talivio Technology OÜ · Nestio on GPL-3.0-or-later; Talivio nimi ja märk ei kuulu litsentsi alla.', 'foot.by': 'Valmistanud Talivio',
    'img.parts': 'img/ekran-parcalar-et.webp', 'img.partsDark': 'img/ekran-parcalar-et-dark.webp', 'img.settings': 'img/ekran-ayarlar-et.webp', 'img.settingsDark': 'img/ekran-ayarlar-et-dark.webp',

    'app.workerFail': 'Arvutustöötajat ei õnnestunud käivitada: {e}',
    'app.workerFailHint': 'sinu brauser ei pruugi moodultöötajaid toetada',
    'app.dwgFail': 'DWG-lugejat ei õnnestunud laadida',
    'app.reading': 'Loen joonist…',
    'app.readFail': 'Joonist ei õnnestunud lugeda: {e}',
    'app.extracting': 'Eraldan detaile…',
    'app.noContours': 'Joonisel ei leitud suletud kontuure. Kontrolli kihte ja seda, et joonis oleks suletud.',
    'app.partsum': '{n} tk · {a} m²',
    'app.units': 'Joonise ühik ei ole mm; mõõdud teisendati millimeetriteks teguriga {u}×.',
    'app.warned': '{n} detailil on hoiatus (hüüumärgiga ruudud); vaata, viies kursori peale.',
    'app.manual': 'Muutsid käsitsi: {list}.',
    'app.attention': 'Tähelepanu',
    'app.resetRoles': 'Võta käsitsi muudatused tagasi',
    'role.part': 'Detail', 'role.plate': 'Leht', 'role.ignore': 'Eira',
    'role.partL': 'detail', 'role.plateL': 'leht', 'role.ignoreL': 'eiratud',
    'app.typeOf': '{name} tüüp',
    'app.thickness': 'paksus {t} mm',
    'app.qty': 'Kogus', 'app.type': 'Tüüp',
    'app.lengthMm': 'Pikkus (mm)', 'app.widthMm': 'Laius (mm)', 'app.delete': 'Kustuta',
    'app.noParts': 'Paigutatavaid detaile pole.',
    'app.noSheets': 'Sisesta vähemalt üks lehe mõõt.',
    'app.nesting': 'Paigutan…',
    'app.nestingReady': 'Paigutan… {n} lehte valmis',
    'app.nestFail': 'Paigutamine ebaõnnestus: {e}',
    'app.stopped': 'Peatatud.',
    'app.sheet': 'Leht {k}',
    'app.sheetLine': '{w} × {h} mm · {n} detaili · täituvus {f}% · kasutatud pikkus {u} mm',
    'app.remnant': ' · jääk {r} × {h} mm',
    'stat.sheets': 'lehed', 'stat.fill': 'kogutäituvus', 'stat.lastUsed': 'viimasel lehel kasutatud pikkus',
    'stat.placed': 'paigutatud detaili', 'stat.holes': 'avades', 'stat.time': 'aeg',
    'app.checkOk': 'Kontroll läbitud',
    'app.checkBad': '{n} kontrolliviga',
    'app.unplaced': '{n} detaili jäi paigutamata: ',
    'app.fileSuffix': 'paigutus',
    'app.pct': '{v}%',
  },
};

export function t(key, vars = {}) {
  const s = D[lang][key] ?? D.en[key] ?? key;
  return s.replace(/\{(\w+)\}/g, (_, k) => (vars[k] ?? ''));
}

// ── motor mesajları (Türkçe kalıp → en / et) ────────────────────────────────
const M = [
  [/^Delikler taranıyor$/, { en: 'Scanning holes', et: 'Skaneerin avasid' }],
  [/^Delik taraması (\d+)\/(\d+)$/, { en: 'Hole scan $1/$2', et: 'Avade skaneerimine $1/$2' }],
  [/^(.+) deliğine (.+) yerleşti$/, { en: '$2 placed inside $1', et: '$2 paigutati detaili $1 avasse' }],
  [/^Plaka (\d+): kalan (\d+) parça tek plakaya deneniyor$/, { en: 'Sheet $1: trying to fit the remaining $2 parts on one sheet', et: 'Leht $1: proovin ülejäänud $2 detaili ühele lehele mahutada' }],
  [/^Plaka (\d+): (\d+) parça diziliyor$/, { en: 'Sheet $1: nesting $2 parts', et: 'Leht $1: paigutan $2 detaili' }],
  [/^Çözücü uyarısı: (.*)$/, { en: 'Solver warning: $1', et: 'Lahendaja hoiatus: $1' }],
  [/^Plaka (\d+) hazır$/, { en: 'Sheet $1 ready', et: 'Leht $1 valmis' }],
  [/^Son plakadaki parçalar (\d+)\. plakaya taşınmaya çalışılıyor$/, { en: 'Trying to move the last sheet\'s parts to sheet $1', et: 'Proovin viimase lehe detaile viia lehele $1' }],
  [/^Bir parça (\d+)\. plakaya taşındı$/, { en: 'A part moved to sheet $1', et: 'Üks detail viidi lehele $1' }],
  [/^Son plaka sıkıştırılıyor$/, { en: 'Compacting the last sheet', et: 'Tihendan viimast lehte' }],
  [/^Bitti$/, { en: 'Done', et: 'Valmis' }],
  [/^Yüz çözülemedi, dış sınır \(silüet\) alındı; delikleri kontrol edin\.$/, { en: 'The face could not be resolved; the outer boundary (silhouette) was used. Check the holes.', et: 'Pinda ei õnnestunud lahendada; kasutati välispiiri (siluetti). Kontrolli avasid.' }],
  [/^Parçaları çevreleyen dikdörtgen plaka olarak algılandı; yanlışsa parça yapın\.$/, { en: 'The rectangle around the parts was taken as a sheet; make it a part if that is wrong.', et: 'Detaile ümbritsev ristkülik tuvastati lehena; kui see on vale, muuda see detailiks.' }],
  [/^Kalınlık ([\d.,]+) mm, çoğunluk ([\d.,]+) mm: parça eğik ya da farklı kalınlıkta olabilir\.$/, { en: 'Thickness $1 mm, most parts $2 mm: the part may be tilted or of a different thickness.', et: 'Paksus $1 mm, enamikul $2 mm: detail võib olla kaldu või erineva paksusega.' }],
  [/^Dış sınırı alındı\.$/, { en: 'Outer boundary used.', et: 'Kasutati välispiiri.' }],
  [/^(\d+) büyük dikdörtgen plaka olarak algılandı; yanlışsa parça yapın\.$/, { en: '$1 large rectangles were taken as sheets; make them parts if that is wrong.', et: '$1 suurt ristkülikut tuvastati lehtedena; kui see on vale, muuda need detailideks.' }],
  [/^Plaka (\d+): (.+) kenar payının dışına taşıyor$/, { en: 'Sheet $1: $2 extends beyond the edge margin', et: 'Leht $1: $2 ulatub servavarust välja' }],
  [/^Plaka (\d+): (.+) ile (.+) arası ([\d.,]+) mm \(istenen ([\d.,]+)\)$/, { en: 'Sheet $1: $2 and $3 are $4 mm apart (required $5)', et: 'Leht $1: $2 ja $3 vahe on $4 mm (nõutud $5)' }],
  [/^çözücü hata verdi$/, { en: 'the solver returned an error', et: 'lahendaja andis vea' }],
  [/^DXF okunamadı: (.*)$/, { en: 'DXF could not be read: $1', et: 'DXF-faili ei õnnestunud lugeda: $1' }],
  [/^DWG okunamadı$/, { en: 'DWG could not be read', et: 'DWG-faili ei õnnestunud lugeda' }],
  [/^çözücü işçisi yanıt vermedi$/, { en: 'a solver worker stopped responding', et: 'lahendustöötaja ei vasta' }],
  [/^çözücü işçisi düştü: (.*)$/, { en: 'a solver worker crashed: $1', et: 'lahendustöötaja jooksis kokku: $1' }],
  [/^bilinmeyen hata$/, { en: 'unknown error', et: 'tundmatu viga' }],
  [/^Rust paniği: (.*)$/, { en: 'Engine panic: $1', et: 'Mootori paanika: $1' }],
  [/^Hesap beklenmedik biçimde durdu: (.*)$/, { en: 'The calculation stopped unexpectedly: $1', et: 'Arvutus peatus ootamatult: $1' }],
];

export function tm(msg) {
  if (lang === 'tr' || typeof msg !== 'string') return msg;
  for (const [re, out] of M) if (re.test(msg)) return msg.replace(re, out[lang]);
  return msg;
}

// ── sayfa metni ──────────────────────────────────────────────────────────────
export function applyStatic(root = document) {
  if (lang !== 'en') {
    const tbl = D[lang];
    const set = (sel, fn) => root.querySelectorAll(sel).forEach(fn);
    set('[data-i18n]', (el) => { const v = tbl[el.dataset.i18n]; if (v != null) el.innerHTML = v; });
    set('[data-i18n-alt]', (el) => { const v = tbl[el.dataset.i18nAlt]; if (v != null) el.alt = v; });
    set('[data-i18n-aria]', (el) => { const v = tbl[el.dataset.i18nAria]; if (v != null) el.setAttribute('aria-label', v); });
    set('[data-i18n-content]', (el) => { const v = tbl[el.dataset.i18nContent]; if (v != null) el.setAttribute('content', v); });
    set('[data-i18n-href]', (el) => { const v = tbl[el.dataset.i18nHref]; if (v != null) el.href = v; });
    set('[data-i18n-src]', (el) => { const v = tbl[el.dataset.i18nSrc]; if (v != null) el.src = v; });
    set('[data-i18n-srcset]', (el) => { const v = tbl[el.dataset.i18nSrcset]; if (v != null) el.srcset = v; });
    const c = document.querySelector('link[rel=canonical]');
    if (c) c.href = `https://nestio.talivio.com/?lang=${lang}`;
  }
  document.querySelectorAll('[data-lang]').forEach((a) => {
    if (a.dataset.lang === lang) a.setAttribute('aria-current', 'true');
    a.addEventListener('click', () => { try { localStorage.setItem('nestio-lang', a.dataset.lang); } catch { /* yok say */ } });
  });
  document.documentElement.classList.remove('i18n-pending');
}

applyStatic();
