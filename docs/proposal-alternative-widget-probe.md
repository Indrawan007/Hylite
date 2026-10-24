# Rancangan dan hasil probe widget alternatif setelah Stage 9b

**Tanggal:** 24 Oktober 2026 · **Branch:** `arena/21a4c00b-hylite`

**Status:** probe test-only/opt-in telah diimplementasikan dan dijalankan; hasil GUI untuk kandidat alternatif ini **NO-GO terbatas**. Dokumen ini tetap menjadi kontrak eksperimen, bukan spesifikasi fitur editing produksi.

**Keputusan scope:** studi kelayakan terbatas untuk widget Iced alternatif; bukan implementasi editor produksi.

## 1. Pertanyaan yang hendak dijawab

Stage 9b tetap ditutup dengan **NO-GO** pada fixture dan jalur yang diuji:
widget `text_editor` mengamati caret `[0]` pada seluruh koordinat x yang
berulang, sehingga drag selection tidak dijalankan. Hasil itu tidak diulang,
tidak diperluas menjadi kesimpulan umum tentang Iced/backend, dan tidak
membuka kembali sweep lama.

Pertanyaan baru yang dibatasi: **pada fixture satu baris yang sama, apakah
widget kustom berbasis API teks Iced dapat mengubah koordinat pointer native
menjadi offset caret melalui hit-test paragraf secara langsung, dengan hasil
yang berulang?** Ini menguji jalur widget/input alternatif, bukan kelayakan
mengedit dokumen.

## 2. Jalur saat ini dan kandidat alternatif

Jalur Stage 9b yang sudah ada:

1. `src/main.rs` menerima flag opt-in `--inline-feasibility-probe <file>`.
2. `src/app.rs` membuka Preview dan `src/document/preview.rs` mengganti hanya
   paragraf target dengan widget dari `src/ui/inline_spike.rs`.
3. Widget itu memakai `text_editor`; event native diterjemahkan menjadi
   `text_editor::Action`, lalu `src/app/inline_spike.rs` menjalankan aksi
   diagnostik yang diizinkan pada `text_editor::Content`.
4. Marker judul membaca caret/selection dari state editor tersebut. Ini adalah
   keseluruhan jalur yang menghasilkan bukti Stage 9b NO-GO.

Kandidat probe yang disarankan adalah **widget kustom opt-in** melalui
`iced::advanced::widget::Widget`, tanpa `text_editor::Action` atau
`Content::perform`. Widget menerima event mouse, mengubah posisi pointer ke
koordinat lokal paragraf, lalu memanggil API paragraf Iced secara langsung:

```rust
let hit = paragraph.hit_test(local_point).map(|hit| hit.cursor());
let caret_point = paragraph.grapheme_position(0, visible_grapheme_index);
```

`Paragraph` yang sama dipakai untuk `Renderer::fill_paragraph`, hit-test, dan
posisi caret. State probe mencatat `hit`, `anchor`, dan `focus`; **tidak**
menyisipkan teks, membuat peta ke byte Source, atau mengirim aksi ke dokumen.
Untuk fixture ASCII ini, indeks karakter yang dikembalikan dapat dibandingkan
langsung dengan offset yang diharapkan; kesetaraan tersebut tidak boleh
digeneralisasi ke Unicode.

Pemisahan jalur ini penting: bila widget alternatif menemukan offset nonnol,
buktinya hanya menunjukkan bahwa kombinasi event widget kustom dan hit-test
paragraf dapat bekerja pada fixture tersebut. Itu belum membuktikan penyebab
NO-GO Stage 9b, belum membuktikan `text_editor` mustahil, dan belum menjadi
editor atau selection production.

## 3. Kontrak eksperimen yang diusulkan

### Fixture dan tampilan

- Pakai fixture disk yang sama: `ab**CDEFGH**ijk\n`; teks yang terlihat adalah
  `abCDEFGHijk`, 11 byte ASCII dengan batas caret `0..=11`.
- Tampilkan satu baris dengan run regular `ab`, bold `CDEFGH`, lalu regular
  `ijk`, menggunakan ukuran/font yang sama dengan probe lama sedapat mungkin.
  Probe hanya menerima teks tampak dan gaya run; ia tidak menghitung offset
  selection ke Source.
- Pertahankan mode Preview, letak paragraf target, tinggi widget, serta padding
  yang sebanding dengan Stage 9b. Jalur ini hanya hidup di flag/suite baru;
  Preview default dan Stage 9a/9b tidak diganti.
- Tidak ada ketikan, clipboard, keyboard editing, Apply, atau transaksi
  dokumen. Event selain klik/drag diagnostik dan Escape diabaikan.

### State dan diagnostik

Simpan state lokal minimal: `dragging`, `anchor: Option<usize>`, dan
`focus: Option<usize>`. Mouse-down menetapkan anchor serta focus dari hasil
`hit_test`; pointer-move selama tombol kiri ditahan hanya memperbarui focus;
mouse-up mengakhiri drag. Rentang dan arah dihitung dari dua offset itu, bukan
diinferensikan dari teks yang dipilih.

Marker judul baru (berbeda dari marker Stage 9b) sebaiknya menampilkan nomor
event, koordinat lokal, hasil `hit_test`/`none`, bounds widget, anchor, focus,
dan arah. Marker adalah oracle diagnostik; screenshot tetap bukti pendamping
saja dan bukan oracle piksel.

### Sampling native yang terbatas

- Pertahankan input X11 asli, sinkronisasi gerak pointer (`--sync`), dan
  deteksi lokasi berbasis marker yang sudah dipakai helper GUI. Jangan kembali
  ke klik koordinat fokus tetap atau sweep penuh selebar jendela.
- Dapatkan rentang x teks dari posisi grapheme pertama dan terakhir pada
  paragraf (`grapheme_position(0, 0)` dan `(0, 11)`), diterjemahkan melalui
  origin widget yang tercatat. Sampel hanya rentang teks itu ditambah margin
  kecil, dengan **batas keras 256 koordinat x unik per lintasan**. Bila rentang
  yang dibutuhkan melebihi batas, hasilnya *inconclusive*; jangan memperlebar
  pencarian.
- Gunakan y di dalam baris yang sama berdasarkan bounds/layout widget. Jika
  marker tidak membuktikan widget menerima event pada lokasi itu, berhenti
  sebagai *inconclusive*; jangan melakukan scan seluruh layar.
- Klik setiap koordinat pada lintasan maju, lalu ulangi koordinat yang sama
  secara terbalik. Catat hasil untuk tiap x; bila offset 3 dan 7 ditemukan,
  jalankan drag 3→7 dan 7→3 masing-masing dua kali menggunakan koordinat yang
  benar-benar telah diamati.
- Batas berlaku untuk seluruh percobaan: maksimal 256 x unik per lintasan,
  tidak ada fallback sweep yang lebih lebar, dan tetap berada dalam anggaran
  global GUI 180 detik serta batas step CI 5 menit yang sudah ada.

## 4. Verdict dan keselamatan

**GO untuk studi ini saja** bila semua syarat terpenuhi:

1. Lintasan menyentuh seluruh caret offset `0..=11` setidaknya sekali.
2. Pemetaan x→offset monoton non-menurun dan koordinat yang sama memberi
   offset sama pada lintasan maju/balik.
3. Dua drag maju melaporkan `anchor=3, focus=7`; dua drag balik melaporkan
   `anchor=7, focus=3`.
4. Source memory/disk, dirty state, dan history tidak berubah; jendela ditutup
   natural; harness selesai di dalam batas.

GO hanya berarti **hit-testing/gestur widget kustom layak untuk satu fixture
ASCII pada X11 + Xvfb + tiny-skia**. GO tidak mengesahkan mapping Source,
selection umum, editing langsung, WYSIWYG, Stage 9c/9d, atau klaim tentang
backend/renderer lain.

Jika percobaan valid selesai tetapi gagal menemukan offset/arah yang diwajibkan,
catat **NO-GO terbatas** untuk kandidat ini dan hentikan; jangan memperluas
sampling. Jika layout, marker, fokus, build, atau batas waktu tidak
memungkinkan pengukuran yang sah, catat **INCONCLUSIVE**, bukan NO-GO. Kegagalan
immutability tetap kegagalan keselamatan, bukan verdict kelayakan.

## 5. Implementasi terisolasi (sudah diterapkan)

Probe dibuat sebagai modul UI `src/ui/inline_advanced_probe.rs`, controller
`src/app/inline_advanced_probe.rs`, renderer Preview terpisah, dan jalur CLI
opt-in `--inline-alt-widget-probe <file>`. Custom widget memakai
`advanced::Widget`/`Paragraph::hit_test`; hasil hanya offset teks tampak serta
geometri paragraf dan tidak dipetakan ke byte Source. Saat probe aktif, app
menolak aksi dokumen lain; pointer hanya memperbarui diagnostik dan Escape
menutup probe. Preview default, Stage 9a/9b, serta panel editing produksi tidak
diganti.

Suite `inline-alt-probe` ditambahkan sebagai step CI terpisah. Ia menggunakan
fixture yang sama, mencatat scan maju/balik serta drag berulang, dan membatasi
koordinat x unik hingga 256 per lintasan. Timeout/pergerakan native yang tak
terkonfirmasi menghentikan scan dan mengklasifikasikan hasil sebagai
INCONCLUSIVE; tidak ada fallback sweep yang lebih lebar. Safety checks wajib
membandingkan Source memory dan disk, dirty/history, serta natural close.
Implementasi tidak menambah dependency.

`Cargo.toml` sudah mengaktifkan fitur Iced `advanced`; fitur `canvas` belum
aktif. Implementasi memilih `advanced::Widget` + `advanced::text::Paragraph`
agar tidak perlu menambah dependency atau mengubah fitur Cargo. API Iced yang
dipakai menyediakan `Paragraph::hit_test`, `Paragraph::grapheme_position`, dan
renderer paragraph. Signature terhadap versi di `Cargo.lock` (`iced 0.13.1`,
`iced_core 0.13.2`, `iced_widget 0.13.4`) kini terverifikasi melalui lint, build,
tes Rust, dan runtime suite X11 pada CI yang dirujuk di bawah; bukti runtime
tetap hanya berlaku untuk konfigurasi tersebut.

Tetap di luar scope: Source mapping/editing, general selection/glyph mapping,
Unicode, IME, RTL, scroll sync, pixel oracle, klaim performa, Arch + Hyprland /
Wayland, dan perubahan Preview/WYSIWYG produk. **Stage 9b NO-GO tetap
historis dan final untuk probe yang sudah dijalankan.**

## Referensi API yang ditinjau

- Versi proyek: [`Cargo.toml`](../Cargo.toml) mengaktifkan `advanced` tetapi
  bukan `canvas`; [`Cargo.lock`](../Cargo.lock) mengunci Iced `0.13.1`,
  `iced_core 0.13.2`, dan `iced_widget 0.13.4`.
- API paragraf Iced 0.13.1: [`Paragraph::hit_test` dan
  `grapheme_position`](https://github.com/iced-rs/iced/blob/9bfbd7cda79aceef2d115b8bb35e8f3257dcabf2/core/src/text/paragraph.rs),
  [`Hit` dan text renderer](https://github.com/iced-rs/iced/blob/9bfbd7cda79aceef2d115b8bb35e8f3257dcabf2/core/src/text.rs),
  serta [`advanced` re-exports](https://github.com/iced-rs/iced/blob/9bfbd7cda79aceef2d115b8bb35e8f3257dcabf2/src/advanced.rs).
- Widget lifecycle: Iced [`Widget::on_event` / `draw`](https://github.com/iced-rs/iced/blob/9bfbd7cda79aceef2d115b8bb35e8f3257dcabf2/core/src/widget.rs).
- Proyek saat ini: [CLI](../src/main.rs), [Preview renderer](../src/document/preview.rs),
  [app routing](../src/app.rs), [Stage 9b UI](../src/ui/inline_spike.rs), dan
  [Stage 9b controller](../src/app/inline_spike.rs).

## 6. Hasil implementasi dan GUI

**CI:** [run 40070349616](https://github.com/Indrawan007/Hylite/actions/runs/40070349616),
commit `ea2f4de`, job `120802247347` — seluruh workflow **success**. Lint semua
target, build aplikasi, 1.052 tes Cargo, 344 tes Source core, 93 helper Python,
dan 31 suite GUI / 528 pemeriksaan lulus. Suite baru menyumbang 9 pemeriksaan;
run ini juga menjalankan gate Stage 9b yang lama tanpa perubahan.

Pada fixture disk `ab**CDEFGH**ijk\n` dengan visible ASCII
`abCDEFGHijk`, widget melaporkan geometry dan hit pada baris yang dimaksud.
Masing-masing sweep maju dan balik menguji 113 posisi x unik (di bawah cap
256); koordinat yang sama repeatable, tetapi kedua pemetaan x→offset tidak
monoton. Kedua arah hanya mencapai offset `[0, 1, 2, 3, 4, 5, 6]`, bukan
`7..11`. Karena offset 7 tidak ditemukan, drag 3→7 dan 7→3 tidak dijalankan;
anchor/focus/arah selection dari gesture native tidak teramati.

Verdict yang dicatat adalah **NO-GO terbatas**: percobaan sah dan mencapai
sampling berulang, tetapi gagal syarat cakupan caret serta monotonicity pada
kandidat ini. Ini bukan INCONCLUSIVE dan bukan klaim bahwa semua Iced/backend
mustahil; cakupannya hanya fixture, kombinasi widget kustom dan
X11 + Xvfb + tiny-skia yang diuji. Kegagalan mengamati drag tidak dinilai
sebagai kegagalan drag yang teruji.

Keselamatan lolos: 9 pemeriksaan suite mencakup Source byte-exact, dirty state,
file fixture di disk, ketiadaan history Undo baru, penolakan aksi paste/Undo/
mode-switch ketika probe terbuka, dan natural close. Artefak `gui-x11-smoke`
[id `12551795605`](https://github.com/Indrawan007/Hylite/actions/runs/40070349616/artifacts/12551795605)
tersedia sampai `2026-10-31T06:44:22Z`. Screenshot pendamping bukan oracle
piksel.

Stage 9b asli tetap ditutup **NO-GO historis** pada widget `text_editor` dan
hasil [catatan Stage 9b](testing-stage9b.md) tidak ditimpa oleh percobaan
terpisah ini. Tidak ada mapping Source, editing inline produksi, perubahan
Preview default, klaim WYSIWYG umum, atau persetujuan untuk Stage 9c/9d.