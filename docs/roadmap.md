# Roadmap Hylite — dari Stage 9a sampai rilis 1.0

**Tanggal:** 24 Oktober 2026 · **Branch:** `arena/21a4c00b-hylite` · **Basis kode:** `7b8f82f`
(**1041 Cargo / 344 strict core / 90 Python / 29 suite GUI / 512 pemeriksaan**, CI
[40039268185](https://github.com/Indrawan007/Hylite/actions/runs/40039268185) hijau)

> Dokumen ini adalah **rencana**, bukan janji tanggal. Semua persentase adalah
> **estimasi**, bukan pengukuran. Stage yang sudah selesai (5a–9a, 41 stage) tidak
> diulang di sini; rinciannya ada di [`progress-report.md`](progress-report.md).
> Stage 9a lulus hanya sebagai spike keyboard ASCII/X11 yang sempit; tidak ada
> bukti hit-testing per glyph atau dukungan WYSIWYG umum.

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
pada CI 8e, bukan verifikasi terkini setelah Stage 9a. Screenshot bukan oracle piksel.

### Fase B — WYSIWYG inline (Stage 9a–9d) — gunung esnya

Ini satu-satunya fase yang mengubah sifat produk. Dikerjakan sebagai **spike →
fondasi → perluasan**, bukan big bang.

| Stage | Isi | Usaha | Risiko |
|---|---|---|---|
| **9a** ✅ | **Spike terbatas**: satu baris paragraf, edit inline untuk `**bold**`/`*italic*`/`` `code` `` dengan model "teks terlihat + peta ke byte Source". X11 membuktikan caret keyboard ASCII 0→7 dan delta `left **bold** right\n` → `left **boXld** right\n`, Undo/Redo byte-exact. **Belum** menggantikan panel draft mana pun — [catatan](testing-stage9a.md) | selesai | tinggi untuk perluasan; klik per glyph/selection belum diuji |
| **9b** | **Probe kelayakan terbatas saja**: pada satu fixture ASCII, ukur hit-testing caret per glyph dan selection anchor/arah pada widget native; tanpa mengedit Source atau mengimplementasikan mapping/WYSIWYG. Catat oracle GUI minimal dan hasil go/no-go; Unicode/IME di luar probe ini | kecil–sedang | tinggi; belum dimulai |
| **9c** | **(Bersyarat, belum disetujui)** Marker hiding: `**`, `_`, `` ` ``, `#` disembunyikan saat caret di luar span, muncul saat caret masuk; satu Source tetap sumber kebenaran; undo tetap satu edge per edit efektif. Hanya dipertimbangkan setelah hasil 9b dan keputusan scope terpisah | sedang–besar | sedang |
| **9d** | **(Bersyarat, belum disetujui)** Perluas ke blok lain + pensiun bertahap panel draft: heading, list, quote, dan sel tabel langsung editable di Preview. Panel draft 6h/7a/7b tetap ada sebagai **jalur "raw mode"** yang eksplisit, bukan jalur utama. Tidak dimulai tanpa gate eksplisit | besar | tinggi |

**Hasil gate 9a / keputusan:** CI 40039268185 lulus 1041 Cargo / 344 strict
core / 90 Python / 29 suite GUI / 512 pemeriksaan. Tes `Content` mencakup
perjalanan caret byte ASCII melintasi teks strong; smoke X11 mengonfirmasi
caret keyboard di byte 7 dan splice Source/Undo/Redo yang byte-exact. Ini cukup
untuk **melanjutkan hanya ke feasibility probe 9b**. Klik probe 9a dipakai untuk
fokus saja dan diikuti `Home`; hit-testing per glyph, selection, IME, UTF-8,
RTL, dan Hyprland/Wayland tetap belum terbukti. Stage 9b bukan izin untuk
mengaktifkan WYSIWYG umum.

**Gate probe 9b:** hasil GUI harus menunjukkan posisi caret per glyph dan anchor/arah
selection yang dapat dibedakan secara repeatable pada fixture ASCII yang dibatasi;
jika tidak, probe berhenti dengan no-go. Probe tidak menulis ke Source dan tidak
membuka gate implementasi. Properti round-trip caret (≥10k posisi acak, 0 mismatch),
suite GUI edit inline per konstruksi, dan **tidak ada satu pun oracle lama yang
dilonggarkan** tetap menjadi gate implementasi lanjutan, hanya jika ada keputusan
terpisah untuk melanjutkan setelah 9b.

**Aturan berhenti:** bila feasibility probe 9b menunjukkan caret/hit-testing atau
selection tidak stabil, jangan tambahkan lapisan hack. Opsi mundurnya: (a) editor
inline kustom berbasis `canvas`/`advanced` Iced, atau (b) tetap di model draft-panel
dan mengumumkan bahwa Hylite bukan WYSIWYG. Keputusan itu harus memakai bukti
probe; hasil 9a sendiri hanya mendukung gerak caret keyboard ASCII yang diuji.

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
  └──► 9a spike ──► 9b feasibility probe ──► [keputusan terpisah] ──► 9c marker hiding ──► 9d perluasan
                                                        │
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

Fase A dan spike opt-in 9a kini selesai. Estimasi roadmap naik **≈91 % → ≈92 %**
karena satu stage kecil yang baru dinyatakan selesai; angka itu **bukan** pengukuran
cakupan fitur. Kesiapan rilis tetap **≈44 %**: spike 9a hanya bukti keyboard ASCII
pada Xvfb + tiny-skia, bukan fitur WYSIWYG produksi. Arch + Hyprland belum diuji,
math belum punya oracle visual, packaging belum ada, dan mapping glyph/selection
masih belum dibangun. Progress roadmap dan kesiapan rilis adalah dua angka berbeda;
lihat snapshot [`progress-report.md`](progress-report.md).

---

## 5. Keputusan yang sudah ditetapkan

- Urutan **Fase A → B** dipertahankan. Stage 9a selesai dengan bukti keyboard ASCII
  yang sempit; kandidat berikut adalah feasibility probe Stage 9b untuk hit-testing
  caret dan selection, bukan implementasi WYSIWYG umum.
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
