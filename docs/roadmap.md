# Roadmap Hylite — dari Stage 9b sampai rilis 1.0

**Tanggal:** 24 Oktober 2026 · **Branch:** `arena/21a4c00b-hylite` · **Basis kode terverifikasi:** `0599141`
(CI [40058088359](https://github.com/Indrawan007/Hylite/actions/runs/40058088359) hijau;
93 tes Python / 30 suite GUI / 519 pemeriksaan. Verdict feasibility 9b: **NO-GO**.)

> Dokumen ini adalah **rencana**, bukan janji tanggal. Semua persentase adalah
> **estimasi**, bukan pengukuran. Stage 9a membuktikan hanya caret keyboard
> ASCII/X11; probe read-only 9b selesai dengan NO-GO karena sweep klik hanya
> mengamati offset 0. Tidak ada bukti mapping per glyph, anchor/arah selection,
> atau WYSIWYG umum.

---

## 1. Definisi "akhir" (yang saya kejar)

Hylite 1.0 dinyatakan selesai bila **semua** ini benar:

| # | Kriteria | Bukti yang harus ada |
|---|---|---|
| 1 | **Mengetik langsung di teks ter-render** untuk minimal: paragraf, heading, list, quote, tabel sel — caret, seleksi, dan drag bekerja di dalam teks berformat | Suite GUI byte+caret per skenario; test properti mapping glyph O(1) |
| 2 | **Dokumen nyata** ≥ 1 MB / 5000 baris tetap responsif | Harness benchmark dengan angka absolut (bukan klaim relatif), batas waktu di CI |
| 3 | **Multi-dokumen**: tab, file tree, workspace folder | Suite GUI perpindahan buffer tanpa kehilangan history/dirty state |
| 4 | **Ekspor PDF + HTML** dengan tema | Byte-compare HTML; PDF tervalidasi struktur, bukan piksel |
| 5 | **Math subset + gambar** tampil read-only di Preview | Parser/model test; gambar tidak pernah dieksekusi/di-upload otomatis; bukan oracle piksel |
| 6 | **Berjalan dan divalidasi di Arch + Hyprland/Wayland** (portal, IME, HiDPI, RTL) | Checklist manual tercentang di mesin target, bukan Xvfb |
| 7 | **Terpasang**: PKGBUILD/AUR, build release reproducible | `makepkg` sukses + checksum |
| 8 | Semua gate lama (27 suite, 28 badan exercise) tetap utuh | CI hijau tanpa satu pun gate dilonggarkan |

**Yang eksplisit TIDAK termasuk 1.0** (supaya tidak ada klaim berlebih):
kolaborasi/CRDT, cloud sync, plugin system, mobile, WYSIWYG penuh untuk semua
konstruksi GFM (nested table, HTML block), diagram Mermaid/PlantUML, dan paritas
33-tema ala MarkText.

---

## 2. Fase dan stage

Empat fase, 17 stage. Urutan dipilih supaya **nilai per usaha tertinggi dulu**, dan
supaya pekerjaan yang berisiko membatalkan pekerjaan lain (WYSIWYG) datang setelah
permukaan dokumen stabil.

### Fase A — Permukaan dokumen (Stage 8a–8e)

Target Fase A: keluar dari "satu dokumen, satu jendela, tanpa gambar, tanpa PDF"; semua stage 8a–8e kini selesai, dengan math yang tetap terbatas read-only.

| Stage | Isi | Kenapa sekarang | Usaha | Risiko |
|---|---|---|---|---|
| **8a** ✅ | **Multi-buffer + tab**: tab memarkir seluruh `Document` (bukan `Vec<Buffer>` terpisah); strip di dalam baris toolbar; perpindahan mempertahankan teks, caret, seleksi, snapshot, path, dirty, dan **stack history masing-masing**; `Ctrl+T`/`Ctrl+W`/`Alt+1..9` + `＋`/`×`/`›` | Prasyarat teknis untuk 8b; menyentuh `Hylite` jadi lebih baik sebelum lapisan WYSIWYG menempel | **selesai** — [catatan](testing-stage8a.md) | sedang — refactor state luas (dipilih: park `Document`, bukan buffer kedua) |
| **8b** ✅ | **File tree + workspace folder**: `hylite <folder>`, `Ctrl+Shift+O`, scan sekali tanpa watcher (F5/Refresh), batas 512 entri & kedalaman 8, symlink tidak diikuti, buka file dari tree lewat jalur Open yang sudah ada, `•` per file kotor, `Alt+J/K/Enter` | Melengkapi 8a menjadi alur kerja nyata | **selesai** — [catatan](testing-stage8b.md) | rendah — batas scan dideklarasikan; tanpa watcher |
| **8c** ✅ | **Gambar read-only di Preview**: paragraf berisi satu gambar dirender (PNG/GIF/BMP/JPEG), path relatif terhadap folder dokumen, gerbang 4 MiB / 8192 px / 40 MP dari header, skema apa pun ditolak sebelum I/O, marker inert per alasan | `iced` sudah mengaktifkan fitur `image`, jadi biayanya rendah dan dampak visualnya besar | **selesai** — [catatan](testing-stage8c.md) | rendah — tanpa decoder sendiri, tanpa jaringan |
| **8d** ✅ | **Ekspor PDF**: dokumen → PDF dengan tema aktif, tabel dan code block ter-render | MarkText punya PDF; ini gap ekspor terbesar | **selesai** — [catatan](testing-stage8d.md) | sedang — ditulis sendiri (PDF 1.4, base-14, WinAnsi), tanpa browser dan tanpa crate |
| **8e** ✅ | **Math read-only (subset)**: `$…$` inline dan `$$…$$` display; parser std-only untuk `frac`, `sqrt`, sup/sub, delimiter, Greek/operator yang dideklarasikan; hasil dilinearkan ke Unicode, sintaks unsupported/malformed/batas 2048 byte atau 32 group menjadi source delimited ber-marker | Menutup gap yang terlihat tanpa berpura-pura jadi KaTeX penuh | **selesai** — [catatan](testing-stage8e.md) | tinggi — tidak ada KaTeX/MathJax, layout 2D, mapping glyph, atau oracle piksel |

**Gate Fase A terpenuhi saat Stage 8e ditutup:** 5 suite GUI baru (tab, file tree,
gambar, PDF, math), semua 27 suite sebelumnya hijau, dan tidak ada oracle
byte/koordinat yang dikendurkan. Snapshot penutupan Fase A: [CI 39963797627](https://github.com/Indrawan007/Hylite/actions/runs/39963797627),
28 suite / 505 pemeriksaan GUI; suite math memberi 7 pemeriksaan. Ini metrik historis
pada CI 8e, bukan verifikasi terkini setelah Stage 9b. Screenshot bukan oracle piksel.

### Fase B — WYSIWYG inline (Stage 9a–9d) — gunung esnya

Ini satu-satunya fase yang mengubah sifat produk. Dikerjakan sebagai **spike →
fondasi → perluasan**, bukan big bang.

| Stage | Isi | Usaha | Risiko |
|---|---|---|---|
| **9a** ✅ | **Spike terbatas**: satu baris paragraf, edit inline untuk `**bold**`/`*italic*`/`` `code` `` dengan model "teks terlihat + peta ke byte Source". X11 membuktikan caret keyboard ASCII 0→7 dan delta `left **bold** right\n` → `left **boXld** right\n`, Undo/Redo byte-exact. **Belum** menggantikan panel draft mana pun — [catatan](testing-stage9a.md) | selesai | tinggi untuk perluasan; hit-testing/selection tetap belum terbukti setelah probe 9b NO-GO |
| **9b** ⚠️ NO-GO (probe selesai) | **Feasibility terbatas**: satu fixture ASCII, hit-testing caret dan selection pada widget native, tanpa Source edit atau implementasi mapping. Hasil: hanya caret 0 yang teramati; drag tidak dijalankan karena endpoint 3/7 tidak ditemukan — [bukti lengkap](testing-stage9b.md) | kecil–sedang | probe tertutup; hit-testing/selection belum layak dibuktikan |
| **9c** | **HOLD — belum disetujui**. Marker hiding (`**`, `_`, `` ` ``, `#`) tidak dimulai setelah verdict 9b NO-GO. Rancangan probe alternatif pasca-9b tidak mengizinkan implementasi 9c; tetap perlu keputusan scope produk terpisah | sedang–besar | sedang |
| **9d** | **HOLD — belum disetujui**. Perluasan editing langsung ke heading/list/quote/sel tabel dan pensiun panel draft tidak dimulai; perlu gate dan scope produk baru yang eksplisit | besar | tinggi |

**Hasil gate 9a:** CI 40039268185 lulus 1041 Cargo / 344 strict core / 90
Python / 29 suite GUI / 512 pemeriksaan. Tes `Content` mencakup perjalanan caret
byte ASCII melintasi teks strong; smoke X11 mengonfirmasi caret keyboard di byte
7 dan splice Source/Undo/Redo yang byte-exact. Itu hanya mendukung kelanjutan ke
probe 9b, bukan WYSIWYG umum.

**Hasil gate 9b — probe ditutup NO-GO:** CI [40058088359](https://github.com/Indrawan007/Hylite/actions/runs/40058088359)
lulus seluruh langkah workflow (93 Python / 30 suite GUI / 519 pemeriksaan),
tetapi feasibility runner hanya menemukan caret `[0]` pada 325 posisi x yang
berulang maju/balik; dense band `x=256..511` juga tidak mencapai offset 1–11.
Karena titik 3 dan 7 tidak ditemukan, drag tidak dijalankan dan anchor/arah
selection tidak teramati. Source, dirty state, file fixture dan history tidak
berubah. Lihat [catatan hasil 9b](testing-stage9b.md).

**Aturan berhenti:** verdict NO-GO menutup probe Stage 9b; jangan tambahkan
mapping/hack, jangan mengubahnya menjadi editing production, dan jangan mulai
9c/9d. Scope terpisah yang dipilih sesudahnya hanya **rancangan** probe widget
alternatif berbasis `advanced::Widget` + `Paragraph::hit_test`
([rancangan probe alternatif](proposal-alternative-widget-probe.md)); belum ada
implementasi atau hasil GUI. Rancangan itu tidak mengulang scan Stage 9b, tidak
mengubah kesimpulan historis, dan tidak mengizinkan production mapping.
Tidak ada klaim bahwa semua Iced/backend mustahil; bukti Stage 9b hanya berlaku
pada fixture dan konfigurasi X11 + Xvfb + tiny-skia ini. Gate lama tidak diubah.

### Fase C — Skala dan performa (Stage 10a–10c)

| Stage | Isi | Usaha | Risiko |
|---|---|---|---|
| **10a** | **Incremental parse**: parse hanya blok yang berubah (block-level cache + dirty range), bukan seluruh dokumen | besar | sedang |
| **10b** | **Harness benchmark**: dokumen sintetis 100 KB/1 MB/5 MB, metrik absolut (ms) untuk buka, ketik, parse, render; batas waktu di CI supaya regresi tertangkap | kecil–sedang | rendah |
| **10c** | **Viewport culling** untuk dokumen sangat besar (render hanya blok yang terlihat) + sinkronisasi scroll Preview↔Source | sedang–besar | sedang |

**Gate Fase C:** angka absolut terdokumentasi di `docs/performance.md`, plus
**larangan klaim**: tidak ada kata "cepat" tanpa angka yang dihasilkan harness ini.

### Fase D — Target, paket, rilis (Stage 11a–11e)

| Stage | Isi | Usaha | Bisa saya verifikasi? |
|---|---|---|---|
| **11a** | **Instrumentasi target**: log diagnostik (backend Iced, skala, font, locale, portal), tombol salin laporan, skrip `scripts/report-environment.sh` | kecil | ya (kompilasi + tes) |
| **11b** | **Checklist manual Arch + Hyprland/Wayland**: portal file dialog, IME (`fcitx5`/`ibus`), HiDPI, RTL, GPU; dokumen hasilnya terisi olehmu | kecil (kode) + manual | **tidak** — butuh mesinmu |
| **11c** | **PKGBUILD + build release** (`--release`, LTO, strip), cek `makepkg` | kecil–sedang | sebagian (lint PKGBUILD) |
| **11d** | **Rilis 0.2.0**: tag, changelog, catatan batas yang jujur | kecil | ya |
| **11e** | **Rilis 1.0.0**: hanya setelah definisi di bagian 1 terpenuhi | kecil | ya |

---

## 3. Ketergantungan dan urutan

```
8a tab ──► 8b file tree
  │
  ├──► 8c gambar ──► 8d PDF ──► 8e math
  │
  └──► 9a spike ──► 9b feasibility probe (NO-GO)
                         │
                         └──► [rancangan probe widget alternatif selesai; implementasi/uji GUI terpisah]
                                      │
                                      └──► 9c/9d tetap HOLD; tidak ada production mapping
                                       10a incremental ◄┘ (baru bermakna setelah 9d)
                                       10b benchmark (boleh paralel kapan saja)
                                       10c culling + scroll sync
                                                        │
                                       11a instrumentasi ──► 11b manual (kamu)
                                       11c PKGBUILD ──► 11d 0.2.0 ──► 11e 1.0.0
```

**Paralel yang aman:** 8c/8d/8e selesai di cabang ini tanpa menyentuh 9x;
10b (benchmark) sebaiknya **sebelum** 10a supaya regresi terukur sejak awal.

---

## 4. Estimasi kasar (bukan janji)

| Fase | Stage | Estimasi usaha | Sisa roadmap (bobot) |
|---|---|---|---|
| A — Permukaan dokumen | 8a–8e | ~25 % | 0 % (selesai) |
| B — WYSIWYG inline | 9a–9d | ~45 % | ≈58 % |
| C — Skala & performa | 10a–10c | ~20 % | 15 % |
| D — Target & rilis | 11a–11e | ~10 % | 10 % |

Fase A, spike 9a, dan gate feasibility 9b kini ditutup. Estimasi roadmap naik
**≈92 % → ≈93 %** untuk satu milestone bukti kecil; 9b berakhir NO-GO dan tidak
menambah kapabilitas produk. **Kesiapan rilis tetap ≈44 %**: Stage 9a hanya bukti
keyboard ASCII dan Stage 9b tidak membuktikan mapping glyph/selection. Arch +
Hyprland belum diuji, math belum punya oracle visual, packaging belum ada, dan
WYSIWYG umum belum dibangun. Rancangan probe alternatif pasca-9b adalah studi
dokumentasi saja, bukan milestone/fitur, sehingga tidak mengubah salah satu
angka. Progress roadmap dan kesiapan rilis adalah dua angka berbeda; lihat
snapshot [`progress-report.md`](progress-report.md).

---

## 5. Keputusan yang sudah ditetapkan

- Urutan **Fase A → B** dipertahankan. Stage 9b telah diuji dan ditutup dengan
  verdict NO-GO; tidak ada izin otomatis untuk Stage 9c/9d, production mapping,
  atau WYSIWYG umum. Scope terpisah yang dipilih adalah rancangan probe widget
  alternatif saja ([rancangan](proposal-alternative-widget-probe.md)); implementasi
  atau bukti GUI belum dilakukan dan harus tetap terisolasi dari produk.
- Math read-only subset yang sebelumnya ditunda kini dilaksanakan sebagai Stage 8e.
  Diagram tetap di luar scope; tidak ada klaim KaTeX parity.
- Crate baru boleh minimal; implementasi 8e tidak menambah dependency.
- Sampai Stage 11b, klaim desktop tetap dibatasi ke Xvfb + tiny-skia, bukan
  Arch + Hyprland/Wayland.

---

## 6. Aturan kerja yang tetap berlaku

- Satu stage = implementasi + CI hijau + dokumen + update `progress-report.md`.
- Tidak pernah melonggarkan bytes/koordinat/oracle/deadline/refocus atau menurunkan
  jumlah pemeriksaan; kalau perilaku lama harus berubah, itu **perubahan kontrak
  yang diumumkan**, bukan patch diam-diam.
- Setiap klaim di dokumen harus punya bukti yang bisa dijalankan ulang.
- Browser/WebView tidak pernah masuk: kalau sebuah fitur hanya bisa dicapai dengan
  menanam Chromium, fitur itu keluar dari scope.
