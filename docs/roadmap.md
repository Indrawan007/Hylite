# Roadmap Hylite — dari Stage 8c sampai rilis 1.0

**Tanggal:** 23 Oktober 2026 · **Branch:** `arena/21a4c00b-hylite` · **Basis:** `3652184`
(**988 Cargo / 314 strict core / 79 Python / 26 suite GUI 488 pemeriksaan**, CI
[39947223034](https://github.com/Indrawan007/Hylite/actions/runs/39947223034) hijau)

> Dokumen ini adalah **rencana**, bukan janji tanggal. Semua persentase adalah
> **estimasi**, bukan pengukuran. Stage yang sudah selesai (5a–8c, 38 stage) tidak
> diulang di sini; rinciannya ada di [`progress-report.md`](progress-report.md).
> Stage 8c juga sudah lulus CI (988 Cargo / 26 suite / 488 pemeriksaan).

---

## 1. Definisi "akhir" (yang saya kejar)

Hylite 1.0 dinyatakan selesai bila **semua** ini benar:

| # | Kriteria | Bukti yang harus ada |
|---|---|---|
| 1 | **Mengetik langsung di teks ter-render** untuk minimal: paragraf, heading, list, quote, tabel sel — caret, seleksi, dan drag bekerja di dalam teks berformat | Suite GUI byte+caret per skenario; test properti mapping glyph O(1) |
| 2 | **Dokumen nyata** ≥ 1 MB / 5000 baris tetap responsif | Harness benchmark dengan angka absolut (bukan klaim relatif), batas waktu di CI |
| 3 | **Multi-dokumen**: tab, file tree, workspace folder | Suite GUI perpindahan buffer tanpa kehilangan history/dirty state |
| 4 | **Ekspor PDF + HTML** dengan tema | Byte-compare HTML; PDF tervalidasi struktur, bukan piksel |
| 5 | **Math + gambar** tampil di Preview | Golden test render; gambar tidak pernah dieksekusi/di-upload otomatis |
| 6 | **Berjalan dan divalidasi di Arch + Hyprland/Wayland** (portal, IME, HiDPI, RTL) | Checklist manual tercentang di mesin target, bukan Xvfb |
| 7 | **Terpasang**: PKGBUILD/AUR, build release reproducible | `makepkg` sukses + checksum |
| 8 | Semua gate lama (26 suite, 28 badan exercise) tetap utuh | CI hijau tanpa satu pun gate dilonggarkan |

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

Tujuannya: keluar dari "satu dokumen, satu jendela, tanpa gambar, tanpa PDF".

| Stage | Isi | Kenapa sekarang | Usaha | Risiko |
|---|---|---|---|---|
| **8a** ✅ | **Multi-buffer + tab**: tab memarkir seluruh `Document` (bukan `Vec<Buffer>` terpisah); strip di dalam baris toolbar; perpindahan mempertahankan teks, caret, seleksi, snapshot, path, dirty, dan **stack history masing-masing**; `Ctrl+T`/`Ctrl+W`/`Alt+1..9` + `＋`/`×`/`›` | Prasyarat teknis untuk 8b; menyentuh `Hylite` jadi lebih baik sebelum lapisan WYSIWYG menempel | **selesai** — [catatan](testing-stage8a.md) | sedang — refactor state luas (dipilih: park `Document`, bukan buffer kedua) |
| **8b** ✅ | **File tree + workspace folder**: `hylite <folder>`, `Ctrl+Shift+O`, scan sekali tanpa watcher (F5/Refresh), batas 512 entri & kedalaman 8, symlink tidak diikuti, buka file dari tree lewat jalur Open yang sudah ada, `•` per file kotor, `Alt+J/K/Enter` | Melengkapi 8a menjadi alur kerja nyata | **selesai** — [catatan](testing-stage8b.md) | rendah — batas scan dideklarasikan; tanpa watcher |
| **8c** ✅ | **Gambar read-only di Preview**: paragraf berisi satu gambar dirender (PNG/GIF/BMP/JPEG), path relatif terhadap folder dokumen, gerbang 4 MiB / 8192 px / 40 MP dari header, skema apa pun ditolak sebelum I/O, marker inert per alasan | `iced` sudah mengaktifkan fitur `image`, jadi biayanya rendah dan dampak visualnya besar | **selesai** — [catatan](testing-stage8c.md) | rendah — tanpa decoder sendiri, tanpa jaringan |
| **8d** ✅ | **Ekspor PDF**: dokumen → PDF dengan tema aktif, tabel dan code block ter-render | MarkText punya PDF; ini gap ekspor terbesar | **selesai** — [catatan](testing-stage8d.md) | sedang — ditulis sendiri (PDF 1.4, base-14, WinAnsi), tanpa browser dan tanpa crate |
| **8e** | **Math read-only (subset)**: `$…$` dan `$$…$$` dikenali sebagai blok math dan dirender dengan **subset** operator/struktur yang dideklarasikan (pecahan, sup/sub, akar, operator umum), sisanya jatuh ke teks mentah yang ditandai | Menutup gap yang paling terlihat tanpa berpura-pura jadi KaTeX penuh | sedang–besar | tinggi — jangan mengklaim paritas KaTeX |

**Gate Fase A:** 5 suite GUI baru (tab, file tree, gambar, PDF, math), semua suite
lama hijau, 0 regresi pada oracle byte/koordinat.

### Fase B — WYSIWYG inline (Stage 9a–9d) — gunung esnya

Ini satu-satunya fase yang mengubah sifat produk. Dikerjakan sebagai **spike →
fondasi → perluasan**, bukan big bang.

| Stage | Isi | Usaha | Risiko |
|---|---|---|---|
| **9a** | **Spike terbatas**: satu baris paragraf, edit inline untuk `**bold**`/`*italic*`/`` `code` `` dengan model "teks terlihat + peta ke byte Source". Bukti: caret bergerak benar, mengetik di dalam teks tebal menghasilkan delta Source yang tepat. **Belum** menggantikan panel draft mana pun | besar | tinggi |
| **9b** | **Mapping glyph/selection dua arah** yang benar: caret ↔ byte, seleksi arah/anchor, IME-safe offset, tab/emoji/ZWJ. Diuji dengan properti deterministik (round-trip caret untuk ribuan posisi acak) | besar | tinggi |
| **9c** | **Marker hiding**: `**`, `_`, `` ` ``, `#` disembunyikan saat caret di luar span, muncul saat caret masuk; satu Source tetap sumber kebenaran; undo tetap satu edge per edit efektif | sedang–besar | sedang |
| **9d** | **Perluas ke blok lain + pensiun bertahap panel draft**: heading, list, quote, dan sel tabel langsung editable di Preview. Panel draft 6h/7a/7b tetap ada sebagai **jalur "raw mode"** yang eksplisit, bukan jalur utama | besar | tinggi |

**Gate Fase B:** properti round-trip caret (≥10k posisi acak, 0 mismatch),
suite GUI untuk edit inline per konstruksi, dan **tidak ada satu pun oracle lama
yang dilonggarkan** — bila perilaku lama berubah, itu diumumkan sebagai perubahan
kontrak, bukan patch diam-diam.

**Aturan berhenti:** kalau 9a gagal secara fundamental (mis. `text_editor` Iced tidak
bisa memberi posisi caret per-glyph yang stabil), saya **tidak** akan memaksakan
lapisan hack. Opsi mundurnya: (a) editor inline kustom berbasis `canvas`/`advanced`
Iced, atau (b) tetap di model draft-panel dan mengumumkan bahwa Hylite bukan
WYSIWYG — keputusan itu saya serahkan ke kamu dengan data dari spike.

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
  └──► 9a spike ──► 9b mapping ──► 9c marker hiding ──► 9d perluasan
                                                        │
                                       10a incremental ◄┘ (baru bermakna setelah 9d)
                                       10b benchmark (boleh paralel kapan saja)
                                       10c culling + scroll sync
                                                        │
                                       11a instrumentasi ──► 11b manual (kamu)
                                       11c PKGBUILD ──► 11d 0.2.0 ──► 11e 1.0.0
```

**Paralel yang aman:** 8c/8d/8e bisa dikerjakan kapan pun tanpa menyentuh 9x;
10b (benchmark) sebaiknya **sebelum** 10a supaya regresi terukur sejak awal.

---

## 4. Estimasi kasar (bukan janji)

| Fase | Stage | Estimasi usaha | Sisa roadmap (bobot) |
|---|---|---|---|
| A — Permukaan dokumen | 8a–8e | ~25 % | 15 % |
| B — WYSIWYG inline | 9a–9d | ~45 % | 60 % |
| C — Skala & performa | 10a–10c | ~20 % | 15 % |
| D — Target & rilis | 11a–11e | ~10 % | 10 % |

Artinya: kalau fase A selesai semua, **kesiapan rilis naik ke ≈ 55–60 %** (dari ≈ 43 %),
tetapi **kemiripan pengalaman MarkText baru naik sedikit** — karena inti pengalaman
ada di Fase B. Ini konsekuensi yang saya sampaikan di awal, bukan kejutan di akhir.

---

## 5. Yang saya butuh darimu (keputusan)

Empat hal ini mengubah isi roadmap, jadi saya tanyakan sebelum masuk Fase A:

1. **Prioritas:** Fase A dulu (cepat kelihatan hasilnya, dokumen nyata bisa dipakai)
   atau langsung Fase B (intinya, tapi berbulan-bulan dan berisiko)?
2. **Math/diagram:** subset math buatan sendiri, atau tunda total (tetap inert)?
3. **Dependency baru:** apakah boleh menambah crate untuk PDF/decode gambar, atau
   harus tetap murni memakai yang sudah ada?
4. **Verifikasi target:** kamu punya mesin Arch + Hyprland untuk menjalankan
   checklist 11b, atau tetap di Xvfb sampai ada akses?

Tanpa jawaban, default saya: **Fase A → B**, math **ditunda**, dependency **boleh
minimal**, dan semua klaim tetap dibatasi ke Xvfb + tiny-skia.

---

## 6. Aturan kerja yang tetap berlaku

- Satu stage = implementasi + CI hijau + dokumen + update `progress-report.md`.
- Tidak pernah melonggarkan bytes/koordinat/oracle/deadline/refocus atau menurunkan
  jumlah pemeriksaan; kalau perilaku lama harus berubah, itu **perubahan kontrak
  yang diumumkan**, bukan patch diam-diam.
- Setiap klaim di dokumen harus punya bukti yang bisa dijalankan ulang.
- Browser/WebView tidak pernah masuk: kalau sebuah fitur hanya bisa dicapai dengan
  menanam Chromium, fitur itu keluar dari scope.
