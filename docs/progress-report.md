# Laporan Progress — Hylite (snapshot Stage 9b)

**Tanggal:** 24 Oktober 2026 · **Branch:** `arena/21a4c00b-hylite` · **HEAD kode terverifikasi:** `0599141`

**Basis:** commit `0599141` lulus [CI 40058088359](https://github.com/Indrawan007/Hylite/actions/runs/40058088359); semua gate workflow sukses, sementara verdict feasibility Stage 9b adalah **NO-GO**. Status kerja: **belum di-merge ke `main`**.

> Persentase di bawah adalah **estimasi terhadap roadmap bertahap yang dideklarasikan**,
> bukan hasil pengukuran otomatis dan bukan klaim kesiapan produksi. Dua angka sengaja
> dipisah agar tidak menyesatkan: progress roadmap ≠ kesiapan rilis.

## 1. Ringkasan persentase

| Sudut pandang | Angka | Arti |
|---|---|---|
| **Roadmap bertahap yang dideklarasikan** (Stage 5a–9b) | **≈ 93 %** | 42 milestone sampai feasibility probe 9b ditutup; 9b berakhir NO-GO dan tidak menambah fitur produk |
| **Kesiapan rilis / produksi** (Arch + Hyprland, dokumen nyata) | **≈ 44 %** | belum ada validasi desktop target, packaging, maupun WYSIWYG umum; probe NO-GO tidak mengubah kesiapan produksi |

Rincian bobot ada di [bagian 6](#6-basis-perhitungan-persentase). Angka 93 %
tinggi karena roadmap tahap yang dideklarasikan sengaja terbatas pada editor
native dan prototipe editing blok. Kenaikan ≈92 % → ≈93 % hanya menandai
penutupan milestone feasibility 9b dengan hasil NO-GO; itu **bukan** lompatan
kemampuan produk atau kesiapan rilis. Skor kategori 82,3 % pada bagian 3 adalah
estimasi berbobot lain, bukan angka roadmap atau kesiapan rilis.

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

### Verifikasi otomatis — semua gate CI lulus

| Metrik | Nilai |
|---|---|
| Rust format, strict core, Clippy, build dan unit tests | **lulus** pada CI `40058088359` |
| Helper Python | **93 tes**; kompilasi helper lulus |
| Suite GUI X11 | **30 suite / 519 pemeriksaan**; suite 9b menambah 7 tanpa mengganti gate lama |
| Main regression smoke | **116 pemeriksaan** tetap lulus |
| Gate GUI terdahulu | 29 suite yang sudah ada tetap dijalankan dan lulus; tidak ada assertion atau deadline yang dilonggarkan |

### Status fitur dan gap yang tersisa

| Area | Kondisi |
|---|---|
| Validasi desktop Arch + Hyprland/Wayland | **belum** — semua bukti dari Xvfb + tiny-skia |
| Portal/dialog file nyata, IME, RTL, HiDPI, GPU | **belum** |
| Stage 9b: hit-testing caret per glyph + selection anchor/arah | **probe selesai, NO-GO** — hanya offset 0 teramati pada fixture X11; drag/anchor/arah tidak teruji |
| Selection mapping umum, drag, multi-cell | **belum** |
| Sinkronisasi scroll Preview↔Source | **belum** |
| WYSIWYG inline bebas ala MarkText | **belum** (masih panel draft terbatas) |
| Multi-dokumen (tab + file tree) | **ya, terbatas** — tanpa watcher/split pane/session restore |
| Gambar di Preview | **ya, terbatas** — PNG/GIF/BMP/JPEG lokal ≤4 MiB & ≤8192 px; tanpa oracle piksel, SVG/WebP, atau jaringan |
| Ekspor PDF | **ya, terbatas** — PDF 1.4 ditulis sendiri, palet tema aktif, tabel + code block, tanpa browser; math tetap source inert; tanpa gambar/link/font embedding/kompresi |
| Math subset | **ya, terbatas** — inline/display, `frac`/`sqrt`/sup-sub/Greek/operator eksplisit, Unicode linearized; unsupported source ber-marker; tanpa KaTeX, 2D layout, glyph oracle |
| Incremental parsing / klaim performa | **belum** — semua jalur O(n) salin+parse |
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
3. **O(n) di banyak jalur** (parse penuh, salinan Source, dua buffer native di plan
   7e/7f) — memori ekstra, belum ada optimasi.
4. **Statistik tes bukan cakupan.** Angka GUI membuktikan byte/fokus/urutan, bukan
   piksel atau mapping glyph. Stage 9a membuktikan satu caret keyboard ASCII;
   Stage 9b hanya mengamati caret 0 pada probe klik terbatas dan tidak membuktikan
   mapping glyph atau selection.
5. **Screenshot artefak belum direview visual** dan bukan oracle piksel.

## 5. Saran langkah berikutnya (perkiraan usaha)

| Prioritas | Kandidat | Nilai | Usaha |
|---|---|---|---|
| 1 | Keputusan scope terpisah setelah Stage 9b NO-GO; jangan mulai 9c atau mapping secara otomatis | menentukan apakah tetap pada panel draft atau merancang studi kelayakan lain yang disetujui | belum ada implementasi yang diotorisasi |
| 2 | Audit manual Arch + Hyprland (checklist Tahap 2–4, IME/RTL/HiDPI) | menutup gap target | sedang; butuh mesin target |
| 3 | Sinkronisasi scroll Preview↔Split dan viewport | kenyamanan dokumen panjang | sedang–besar |
| 4 | Packaging AUR + release build | distribusi | sedang |

## 6. Basis perhitungan persentase

- **≈93 % roadmap**: 42 milestone yang dideklarasikan sampai Stage 9b (5a–5e,
  6a–6p, 7a–7n, 8a–8e, 9a–9b) telah ditutup dengan bukti CI. Stage 9b dihitung
  sebagai penutupan gate feasibility dengan verdict NO-GO, bukan implementasi
  fitur. Kenaikan ≈92 % → ≈93 % hanya progres rencana/keputusan.
- **≈44 % kesiapan rilis**: taksiran kualitatif yang **tidak berubah**; probe
  9b tidak menambah kemampuan produk. Tanpa validasi compositor, packaging, dan
  WYSIWYG umum, aplikasi belum layak disebut siap produksi.
- Bobot kategori pada bagian 3 dipilih mengikuti besar permukaan kode dan risiko,
  bukan jumlah baris. Semua angka bisa direvisi bila prioritas proyek berubah.

## 7. Referensi

- Kontrak & bukti terbaru: [Stage 9b](testing-stage9b.md) · [Stage 9a](testing-stage9a.md) · [Stage 8e](testing-stage8e.md) · [8d](testing-stage8d.md) · [8c](testing-stage8c.md) · [8b](testing-stage8b.md) · [8a](testing-stage8a.md) · [7n](testing-stage7n.md) · [7m](testing-stage7m.md) · [7l](testing-stage7l.md) · [7k](testing-stage7k.md) · [7j](testing-stage7j.md) · [7i](testing-stage7i.md) · [7h](testing-stage7h.md) · [7g](testing-stage7g.md) · [7f](testing-stage7f.md) · [7e](testing-stage7e.md) ·
  [7d](testing-stage7d.md) · [bukti GUI](testing-gui-x11.md) ·
  [verifikasi native](verification-native.md)
- CI terkini: `40058088359` (`0599141`) **success** — format, strict core, Clippy,
  build, unit tests dan seluruh 30 suite GUI lulus; **93 tes Python / 519
  pemeriksaan GUI**. Stage 9b menghasilkan NO-GO tanpa mengganti gate lama.
  Artifact `12546617100`, 4.779.552 byte, expires 2026-10-31T03:37:53Z.
- Verifikasi lokal: 93 tes Python, kompilasi helper dan `git diff --check` lulus.
  Cargo/rustc tidak tersedia lokal; format/build/Clippy/tes Rust terbaru dibuktikan
  oleh CI, bukan oleh pemeriksaan lokal.
