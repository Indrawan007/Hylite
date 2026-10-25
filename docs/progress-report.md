# Laporan Progress — Hylite (Stage 11a; Stage 9c/9d tetap ditunda)

**Tanggal:** 26 Oktober 2026 · **Branch:** `arena/21a4c00b-hylite` · **Kode terakhir terverifikasi:** `fdf991b`

**Basis verifikasi Stage 11a:** commit `fdf991b` lulus seluruh gate di [CI 40244045608](https://github.com/Indrawan007/Hylite/actions/runs/40244045608): helper Python, format, strict Source core, strict Clippy, build, tes Rust, semua suite GUI X11, dan probe opt-in yang telah ada. Laporan diagnostik Stage 11a hanya menyajikan petunjuk yang diizinkan dan merahasiakan nilai endpoint/path; batas serta hasil ada di [docs/testing-stage11a.md](testing-stage11a.md). CI ini **bukan** bukti Arch + Hyprland/Wayland, portal runtime, atau paket/release build. Stage 9b asli tetap **NO-GO historis**; probe custom-widget terpisah juga **NO-GO terbatas**. Status kerja: **belum di-merge ke `main` dan tidak ada PR**. Kontrak dan hasil probe alternatif: [dokumen terpisah](proposal-alternative-widget-probe.md).

> Persentase di bawah adalah **estimasi terhadap roadmap bertahap yang dideklarasikan**,
> bukan hasil pengukuran otomatis dan bukan klaim kesiapan produksi. Dua angka sengaja
> dipisah agar tidak menyesatkan: progress roadmap ≠ kesiapan rilis.

## 1. Ringkasan persentase

| Sudut pandang | Angka | Arti |
|---|---|---|
| **Roadmap bertahap yang dideklarasikan** (Stage 5a–9b) | **≈ 93 %** | 42 milestone sampai probe 9b asli ditutup; Stage 10a/10b dilaporkan terpisah, bukan ditambahkan ke denominator lama |
| **Kesiapan rilis / produksi** (Arch + Hyprland, dokumen nyata) | **≈ 44 %** | belum ada validasi desktop target, packaging, maupun WYSIWYG umum; probe dan harness benchmark tidak mengubah kesiapan produksi |

Rincian bobot ada di [bagian 6](#6-basis-perhitungan-persentase). Angka 93 %
tinggi karena roadmap tahap yang dideklarasikan sengaja terbatas pada editor
native dan prototipe editing blok. Kenaikan ≈92 % → ≈93 % hanya menandai
penutupan milestone feasibility 9b dengan hasil NO-GO; itu **bukan** lompatan
kemampuan produk atau kesiapan rilis. Stage 10a menambah fast path parser/cache
yang konservatif tanpa bukti percepatan; Stage 10b menambah observasi runner,
bukan klaim performa. Skor kategori 82,3 % pada bagian 3 adalah estimasi berbobot
lain, bukan angka roadmap atau kesiapan rilis. Stage 11a menambahkan diagnostik
pendukung dan tidak mengubah kedua persentase: kesiapan target tetap menunggu
verifikasi manual dan packaging.

## 2. Capaian per stage

### Fondasi editor & renderer (Stage 5a–5e) — selesai 100 %

| Stage | Isi | Bukti |
|---|---|---|
| 5a | Renderer preview native (heading, list, quote, code, rule, tabel GFM, tema) | `docs/testing-stage5a.md` |
| 5b | Tabel GFM native + batas 12 kolom / 128 row / 512 sel + fallback | `docs/testing-stage5b.md` |
| 5c | Cache preview + invalidasi revisi | `docs/testing-stage5c.md` |
| 5d | Navigasi provenance blok Preview → Source | `docs/testing-stage5d.md` |
| 5e | Navigasi provenance sel tabel | `docs/testing-stage5e.md` |

### Prototipe editing blok (Stage 6a–6p) — selesai 100 %

| Stage | Isi |
|---|---|
| 6a / 6b | Checkbox task kanonis + Undo/Redo terbatas di Preview |
| 6c / 6d | Level heading root ATX + Setext |
| 6e | Batch task dalam satu transaksi |
| 6f | Alignment kolom tabel |
| 6g | Duplikasi/hapus body row |
| 6h / 6i | Draft raw sel tabel + Previous/Next sel |
| 6j / 6k | Insert body row / insert kolom (before-after) |
| 6l / 6m | Hapus kolom / pindah body row |
| 6n / 6o | Pindah kolom / duplikasi kolom |
| 6p | Apply & Previous sel |

### Paragraf root & draft multiline/sel (Stage 7a–7n) — selesai 100 %

| Stage | Isi |
|---|---|
| 7a | Draft paragraf root satu baris (raw Markdown) |
| 7b | Draft paragraf multiline native 2–128 baris, ≤16 KiB |
| 7c | History lokal bounded (100 record / 1 MiB / 16 KiB per record) |
| 7d | Previous/Next antar paragraf multiline, khusus draft unchanged |
| 7e | Apply & Next atomik dengan preflight kandidat penuh |
| 7f | Apply & Previous atomik, offset root sebelumnya tetap diverifikasi ulang |
| 7g | Shortcut commit eksplisit draft: Ctrl+Enter, Ctrl+PageUp/PageDown |
| 7h | Hardening properti/diferensial draft multiline (tanpa perubahan produk) |
| 7i | Konteks baris Source pada panel draft + umpan balik aksi yang diblokir |
| 7j | Navigasi dan Apply & Previous/Next paragraf satu baris (atomik, tanpa wrap) |
| 7k | Baris Source terautentikasi pada draft sel tabel (tanpa perubahan layout) |
| 7l | History draft lokal (100 record / 1 MiB) + `Undo draft`/`Redo draft` pada panel paragraf satu baris dan sel tabel |
| 7m | Keyboard `Ctrl+Z`/`Ctrl+Y`/`Ctrl+Shift+Z` untuk kedua stack draft lokal itu, dengan fallback Source/Preview yang tidak berubah |
| 7n | Umpan balik gestur history kosong/terlalu besar di footer + penguatan properti batas history (tanpa perubahan koordinat atau gate GUI) |

### Permukaan dokumen (Stage 8a–8e) — selesai 100 % dari fase yang dijadwalkan

| Stage | Isi | Bukti |
|---|---|---|
| 8a | Tab multi-buffer (park seluruh `Document`, history per tab) | `docs/testing-stage8a.md` |
| 8b | Workspace folder + file tree sekali-scan, tanpa watcher | `docs/testing-stage8b.md` |
| 8c | Gambar read-only di Preview dengan gerbang path/format/ukuran | `docs/testing-stage8c.md` |
| 8d | Ekspor PDF 1.4 yang ditulis sendiri + `hylite export-pdf` headless | `docs/testing-stage8d.md` |
| 8e | Math read-only subset: `$…$` / `$$…$$`, parser std-only, linearization Unicode dan fallback bertanda | `docs/testing-stage8e.md` |

Fase A (8a–8e) selesai. Stage 9a tetap terbatas pada caret keyboard; probe
Stage 9b berikutnya telah ditutup dengan hasil **NO-GO**:

| Stage | Isi | Bukti / batas |
|---|---|---|
| 9a | Opt-in, satu paragraf root satu baris ASCII; projection bold/italic/code; insert/delete satu karakter dengan satu Source transaction | `docs/testing-stage9a.md`; [CI 40039268185](https://github.com/Indrawan007/Hylite/actions/runs/40039268185) |
| Hasil GUI 9a | Caret keyboard 0→7 pada satu posisi di strong run; ketik `X` memberi `left **boXld** right\n`; Undo/Redo byte-exact | X11 + Xvfb + tiny-skia; 7 pemeriksaan; bukan hit-testing per glyph |
| 9b | Probe opt-in read-only pada fixture `ab**CDEFGH**ijk\n`; sampling caret mouse dan directional drag jika endpoint ditemukan | `docs/testing-stage9b.md`; [CI 40058088359](https://github.com/Indrawan007/Hylite/actions/runs/40058088359) |
| Hasil GUI 9b | 325 posisi x yang berulang maju/balik hanya melaporkan caret 0; dense band `x=256..511` juga hanya 0; offset 1–11 tidak teramati | **NO-GO** untuk fixture/jalur X11 + Xvfb + tiny-skia ini |
| Selection | Drag tidak dikirim karena offset 3 dan 7 tidak ditemukan; anchor/arah selection tidak teramati | Bukan bukti bahwa selection gagal pada semua backend; tidak ada implementasi mapping |
| Pengamanan | Source, dirty state, file fixture dan history tetap utuh; jendela keluar natural | 7 pemeriksaan Stage 9b lulus sebagai smoke/safety checks, bukan verdict feasibility GO |
| Probe custom-widget pasca-9b (terpisah, test-only) | 113 x berulang maju/balik mencapai offset 0–6; keduanya non-monotone; offset 7–11 tidak teramati sehingga drag tidak dijalankan | **NO-GO terbatas**; 9 safety/harness checks lulus; [hasil](proposal-alternative-widget-probe.md) |

### Stage 10b — benchmark harness (selesai; test-only)

Harness opt-in di `src/performance_benchmarks.rs` membangun dokumen Markdown
sintetis tepat 100 KB, 1 MB, dan 5 MB, lalu mengukur buka file lokal + inisialisasi
`Document`, satu edit karakter, parse preview dingin, dan konstruksi pohon widget
Preview. Satu warm-up diikuti tiga sampel; angka min/median/maks absolut (ms) dan
batas cakupannya dicatat di [docs/performance.md](performance.md). CI
[40086623422](https://github.com/Indrawan007/Hylite/actions/runs/40086623422)
menjalankan tes yang diabaikan secara eksplisit; test selesai dalam 3,17 detik.
Budget 180 detik untuk harness dan timeout langkah CI 5 menit bersifat pagar
kasar, bukan target tiap operasi. `render_tree` tidak mengukur layout, paint,
GPU, window, atau frame. Hasil satu run tidak dijadikan klaim performa. Stage
9c/9d tetap ditunda sesuai pilihan pengguna A. Harness kini juga mengukur
cache-miss incremental dan fallback untuk Stage 10a; angka dicatat terpisah
sebagai observasi lingkungan.

### Stage 10a — incremental parse Preview (fast path konservatif)

`Document` menyimpan Source dan Snapshot bersama model cache. Setelah edit, cache
revision sebelumnya hanya dipakai sebagai basis bila identitas dokumen cocok dan
revision lebih lama; diff byte exact dihitung dari Source lama dan Source aktif.
Fast path hanya mereparse satu root paragraph satu baris plain yang dikelilingi
batas aman dan tetap menghasilkan satu plain paragraph. Blok sesudahnya disalin
serta seluruh rentang Source absolut dipindah, termasuk nested blocks, task, dan
semua rentang table. Perubahan newline/inline/block syntax, beberapa area yang
berubah, reference syntax, cache tak cocok, atau batas tak terverifikasi memilih
full parse. Full parser tetap oracle/fallback; cache kandidat dari preflight
hanya dipasang bila Source tepat cocok dengan buffer.

Differential tests membandingkan seluruh `Block` terhadap full parse; uji cache
mencakup edit plain Unicode, beberapa mutasi sebelum pembacaan Preview, dan
fallback. Property tests lama, assertion, byte/coordinate oracle, deadline, dan
focus/refocus tetap dipertahankan. Karena dirty range diperoleh dengan scan
Source penuh dan suffix model juga perlu disalin/digeser, implementasi ini belum
membuktikan percepatan; tidak ada klaim performa. Stage 9c/9d tetap terpisah dan
tidak dimulai. Rincian: [proposal/scope Stage 10a](proposal-stage10a-scope.md).

### Stage 11a — diagnostik lingkungan (selesai)

Toolbar **Copy env** membuat laporan atas permintaan pengguna, mengirimkannya ke
clipboard, dan mencatat laporan yang sama ke `stderr`. Laporan berisi tipe
`iced::Renderer` compile-time, scale factor yang ditanyakan dari window Iced
saat dibuka/di-resize, font/ukuran konfigurasi Hylite, serta allowlist petunjuk
locale dan sesi desktop. Endpoint Wayland/X11/D-Bus hanya tampil sebagai
set/unset; nilai dan path tidak ikut. Script [`report-environment.sh`](../scripts/report-environment.sh)
menyajikan OS/kernel/toolchain, hint fontconfig, dan state tiga service portal
melalui pemeriksaan user systemd yang dibatasi timeout.

Tipe renderer bukan identifikasi GPU/backend window runtime; service portal
aktif bukan bukti provider dialog dipilih. Aplikasi tidak mengintrospeksi
provider `rfd`. Diagnostik ini tidak menjalankan atau menggantikan checklist
Stage 11b, dan tidak mengubah estimasi roadmap/kesiapan rilis. Rincian privasi,
tes, dan batas klaim: [catatan Stage 11a](testing-stage11a.md).

### Verifikasi otomatis — semua gate CI lulus

| Metrik | Nilai |
|---|---|
| Rust format, strict core, Clippy, build dan unit/property tests (baseline 10a) | **lulus** pada CI `40094818086` (`16bcb77`); 1.058 tes Cargo lulus, satu benchmark ignored di suite normal, dan 344 strict Source core lulus |
| Full CI Stage 11a | **lulus** pada `fdf991b` / [run `40244045608`](https://github.com/Indrawan007/Hylite/actions/runs/40244045608): helper, format, strict core, Clippy, build, tes Rust, suite GUI X11 dan probe opt-in |
| Helper Python | **98 tes** terdeteksi secara lokal; discovery helper juga lulus di CI `40244045608` |
| Suite GUI X11 | **31 suite / 528 pemeriksaan**; suite custom-widget terpisah menambah 9, semua suite lama lulus |
| Main regression smoke | **116 pemeriksaan** tetap lulus |
| Stage 10b opt-in | **1 test lulus**, selesai 132,40 detik dalam budget 180 detik; tambahan incremental/fallback tercatat hanya sebagai observasi |
| Gate lama | Seluruh gate GUI lama tetap dijalankan; tidak ada assertion, byte/coordinate oracle, deadline atau perilaku focus/refocus yang dilonggarkan |

Satu run terdahulu (`40094340411`) gagal pada pemeriksaan clipboard sentinel di
`scripts/gui-smoke-x11.py:354`; penyebabnya belum diketahui. Kegagalan tersebut
tidak direproduksi oleh CI penuh `40094818086`, yang meluluskan smoke GUI X11.

Run `40240307726` juga gagal pada pemeriksaan clipboard sentinel di GUI smoke;
GitHub menolak rerun karena workflow mungkin rusak, dan penyebabnya belum
teridentifikasi. Kegagalan itu tidak terulang pada pipeline penuh Stage 11a
`40244045608`. Ini hanya catatan non-reproduksi pada run tersebut, bukan
penjelasan akar penyebab atau bukti target desktop.

### Status fitur dan gap yang tersisa

| Area | Kondisi |
|---|---|
| Diagnostik environment Stage 11a | **selesai** — tipe renderer compile-time, skala window, font, locale, redaksi endpoint, serta hint service portal; [catatan](testing-stage11a.md) |
| Validasi desktop Arch + Hyprland/Wayland | **belum** — semua bukti dari Xvfb + tiny-skia |
| Portal/dialog file nyata, IME, RTL, HiDPI, GPU | **belum** — Stage 11a hanya memberi hint, bukan verifikasi runtime |
| Stage 9b asli: caret hit-testing + selection anchor/arah pada widget `text_editor` | **probe selesai, NO-GO historis** — hanya offset 0 teramati pada fixture X11; drag/anchor/arah tidak teruji |
| Probe custom-widget alternatif | **selesai, NO-GO terbatas** — 113 x berulang hanya mencapai offset 0–6/non-monotone; drag tidak dijalankan; tidak ada Source mapping atau editing produksi |
| Selection mapping umum, drag, multi-cell | **belum** |
| Sinkronisasi scroll Preview↔Source | **belum** |
| WYSIWYG inline bebas ala MarkText | **belum** (masih panel draft terbatas) |
| Multi-dokumen (tab + file tree) | **ya, terbatas** — tanpa watcher/split pane/session restore |
| Gambar di Preview | **ya, terbatas** — PNG/GIF/BMP/JPEG lokal ≤4 MiB & ≤8192 px; tanpa oracle piksel, SVG/WebP, atau jaringan |
| Ekspor PDF | **ya, terbatas** — PDF 1.4 ditulis sendiri, palet tema aktif, tabel + code block, tanpa browser; math tetap source inert; tanpa gambar/link/font embedding/kompresi |
| Math subset | **ya, terbatas** — inline/display, `frac`/`sqrt`/sup-sub/Greek/operator eksplisit, Unicode linearized; unsupported source ber-marker; tanpa KaTeX, 2D layout, glyph oracle |
| Harness Stage 10b / incremental parsing / klaim performa | **Stage 10a fast path konservatif diimplementasikan** dengan full-parse oracle/fallback; Stage 10b mengukur cache-miss incremental dan fallback sebagai observasi runner, tanpa klaim performa |
| Packaging/AUR/release build | **belum** |
| Checklist manual Tahap 2–4 | **belum dicentang** |

## 3. Progres per kategori (bobot estimasi)

| Kategori | Bobot | Capaian | Kontribusi |
|---|---|---|---|
| Sumber kebenaran Source, history, file I/O, save/export | 20 % | 100 % | 20,0 |
| Renderer preview + Split + navigasi/refocus | 15 % | 100 % | 15,0 |
| Editing blok tabel/task/heading (6a–6p) | 25 % | 100 % | 25,0 |
| Draft paragraf + multiline + history + Apply & Next/Previous + shortcut commit + keyboard history satu baris (7a–7n) | 15 % | 100 % | 15,0 |
| Verifikasi otomatis & CI | 15 % | 100 % | 15,0 |
| Validasi desktop target + manual checklist | 5 % | 25 % | 1,3 |
| WYSIWYG umum, selection/glyph, scroll sync, performa | 5 % | 10 % | 0,5 |
| **Total** | **100 %** | | **≈ 82,3 %** |

## 4. Risiko & batasan yang masih terbuka

1. **Belum ada bukti di compositor target.** X11 tiny-skia bukan bukti Hyprland/Wayland;
   shortcut bisa diambil compositor, fokus/IME belum diuji nyata.
2. **Belum siap dokumen penting.** History bounded, LF canonical, selection collapse,
   dan draft pane terbatas masih kompromi prototipe.
3. **Kerja O(n) tetap ada**: pencarian diff memindai Source penuh, fast path menyalin
   dan menggeser seluruh suffix model, fallback tetap parse penuh; dua buffer native
   juga ada di plan 7e/7f. Belum ada bukti bahwa fast path mempercepat operasi umum.
4. **Statistik tes bukan cakupan.** Angka GUI membuktikan byte/fokus/urutan, bukan
   piksel atau mapping glyph. Stage 9a membuktikan satu caret keyboard ASCII;
   Stage 9b hanya mengamati caret 0 pada probe klik terbatas dan tidak membuktikan
   mapping glyph atau selection.
5. **Screenshot artefak belum direview visual** dan bukan oracle piksel.

## 5. Saran langkah berikutnya (perkiraan usaha)

| Prioritas | Kandidat | Nilai | Usaha |
|---|---|---|---|
| 1 | Audit manual Arch + Hyprland (checklist Tahap 2–4, IME/RTL/HiDPI) | menutup gap target yang tidak dapat dibuktikan oleh CI X11 | sedang; butuh mesin target dan verifikasi manual |
| 2 | Stage 10a/10b yang telah disetujui dan diverifikasi ditutup pada scope ini; pertahankan full-parse fallback dan jangan perluas produksi tanpa persetujuan terpisah | menjaga kesetaraan hasil dan batas scope | CI lulus; timing satu-run hanya observasi, tanpa klaim atau target latency |
| 3 | Sinkronisasi scroll Preview↔Split dan viewport | kenyamanan dokumen panjang | sedang–besar |
| 4 | Packaging AUR + release build | distribusi | sedang |

## 6. Basis perhitungan persentase

- **≈93 % roadmap Stage 5a–9b**: denominator lama berisi 42 milestone (5a–5e,
  6a–6p, 7a–7n, 8a–8e, 9a–9b). Stage 9b menutup gate feasibility dengan verdict
  NO-GO, bukan implementasi fitur. Probe custom-widget adalah studi test-only di
  luar milestone produk. Stage 10a dan 10b dilaporkan terpisah; angka 93 % tidak
  diperluas diam-diam ke stage berikutnya.
- **≈44 % kesiapan rilis**: taksiran kualitatif yang **tidak berubah**. Stage 10a
  hanya optimasi cache/parser yang konservatif tanpa klaim hasil performa; 10b
  memberi observasi satu-run, bukan validasi target atau kesiapan produksi.
  Tanpa validasi compositor, packaging, dan WYSIWYG umum, aplikasi belum layak
  disebut siap produksi.
- Bobot kategori pada bagian 3 dipilih mengikuti besar permukaan kode dan risiko,
  bukan jumlah baris. Semua angka bisa direvisi bila prioritas proyek berubah.

## 7. Referensi

- Kontrak & bukti terbaru: [keputusan scope Stage 9c/9d](proposal-stage9c-9d-scope.md) · [evaluasi scope Stage 10a](proposal-stage10a-scope.md) · [Stage 10b benchmark](performance.md) · [Stage 9b](testing-stage9b.md) · [kontrak dan hasil probe alternatif](proposal-alternative-widget-probe.md) · [Stage 9a](testing-stage9a.md) · [Stage 8e](testing-stage8e.md) · [8d](testing-stage8d.md) · [8c](testing-stage8c.md) · [8b](testing-stage8b.md) · [8a](testing-stage8a.md) · [7n](testing-stage7n.md) · [7m](testing-stage7m.md) · [7l](testing-stage7l.md) · [7k](testing-stage7k.md) · [7j](testing-stage7j.md) · [7i](testing-stage7i.md) · [7h](testing-stage7h.md) · [7g](testing-stage7g.md) · [7f](testing-stage7f.md) · [7e](testing-stage7e.md) ·
  [7d](testing-stage7d.md) · [bukti GUI](testing-gui-x11.md) ·
  [verifikasi native](verification-native.md)
- CI verifikasi kode Stage 10a: `40094818086` (`16bcb77`) **success** — format, strict core, Clippy,
  build, **1.058 tes Cargo lulus + satu benchmark diabaikan di suite normal**,
  **344 Source core**, 93 tes Python, **31 suite GUI / 528 pemeriksaan**, smoke
  regresi utama 116, dan Stage 10b eksplisit (1 test lulus dalam 132,40 detik).
  Angka mentah Stage 10b serta incremental/fallback ada di
  [docs/performance.md](performance.md), sebagai observasi satu-run tanpa klaim.
  Probe alternatif tetap NO-GO terbatas; Stage 9b asli tetap NO-GO historis.
- CI Stage 9b: `40058088359` (`0599141`) success, **30 suite / 519 pemeriksaan GUI**,
  artifact `12546617100`; hasil historis itu tidak ditimpa oleh probe baru.
- Verifikasi lokal untuk kode Rust: toolchain Cargo/rustc tidak tersedia; format,
  build, Clippy, tes, dan benchmark dibuktikan oleh CI. `git diff --check` lulus.
