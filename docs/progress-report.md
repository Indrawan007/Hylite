# Laporan Progress — Hylite (snapshot Stage 8e)

**Tanggal:** 23 Oktober 2026 · **Branch:** `arena/21a4c00b-hylite` · **HEAD kode:** `97e586c`
(dokumentasi 8e berada di commit terpisah pada branch yang sama)

**Basis:** commit `97e586c` lulus [CI 39963797627](https://github.com/Indrawan007/Hylite/actions/runs/39963797627)
Status kerja: **belum di-merge ke `main`**; dokumentasi 8e ditulis setelah CI kode lulus.

> Persentase di bawah adalah **estimasi terhadap roadmap bertahap yang dideklarasikan**,
> bukan hasil pengukuran otomatis dan bukan klaim kesiapan produksi. Dua angka sengaja
> dipisah agar tidak menyesatkan: progress roadmap ≠ kesiapan rilis.

## 1. Ringkasan persentase

| Sudut pandang | Angka | Arti |
|---|---|---|
| **Roadmap bertahap yang dideklarasikan** (Stage 5a–8e) | **≈ 91 %** | 40 stage yang dideklarasikan sudah diimplementasi + diverifikasi otomatis |
| **Kesiapan rilis / produksi** (Arch + Hyprland, dokumen nyata) | **≈ 44 %** | belum ada validasi desktop target, packaging, maupun WYSIWYG umum |

Rincian bobot ada di [bagian 6](#6-basis-perhitungan-persentase). Angka 91 % tinggi
karena roadmap tahap yang dideklarasikan sengaja terbatas pada editor native dan
prototipe editing blok. Kenaikan 90 % → 91 % berasal dari satu stage (8e), jadi
jangan dibaca sebagai lompatan kemampuan produk. Skor kategori 82,3 % pada bagian 3
adalah estimasi berbobot lain, bukan angka roadmap atau kesiapan rilis.

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

Fase A (8a–8e) selesai; kandidat berikut sesuai graf adalah Stage 9a spike inline terbatas.

### Verifikasi otomatis — selesai 100 % dari yang dirancang

| Metrik | Nilai |
|---|---|
| Tes Cargo | **1026** lulus |
| Tes core mandiri (subset) | **335** |
| Helper Python | **88** |
| Suite GUI X11 | **28 suite / 505 pemeriksaan** (446 clipboard, 18 title, 28 exit native) |
| Gate lama yang dipertahankan | 27 suite sebelumnya dijalankan ulang utuh + 28 badan exercise lama identik AST |
| Lint/format | `cargo fmt` + strict Clippy `-D warnings` + build debug |

### Yang belum dikerjakan

| Area | Kondisi |
|---|---|
| Validasi desktop Arch + Hyprland/Wayland | **belum** — semua bukti dari Xvfb + tiny-skia |
| Portal/dialog file nyata, IME, RTL, HiDPI, GPU | **belum** |
| Selection arah/anchor, mapping glyph, drag, multi-cell | **belum** |
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
4. **Statistik tes bukan cakupan.** 335 core adalah subset dari 1026 Cargo; angka GUI
   membuktikan byte/fokus/urutan, bukan piksel atau mapping glyph.
5. **Screenshot artefak belum direview visual** dan bukan oracle piksel.

## 5. Saran langkah berikutnya (perkiraan usaha)

| Prioritas | Kandidat | Nilai | Usaha |
|---|---|---|---|
| 1 | Stage 9a spike inline terbatas untuk bold/italic/code; uji apakah `text_editor` memberi caret per-glyph stabil sebelum melangkah ke 9b | bukti prasyarat WYSIWYG tanpa big-bang | besar / risiko tinggi |
| 2 | Audit manual Arch + Hyprland (checklist Tahap 2–4, IME/RTL/HiDPI) | menutup gap target | sedang; butuh mesin target |
| 3 | Stage 9b mapping glyph/selection dua arah bila spike 9a lolos | prasyarat editing inline | besar |
| 4 | Sinkronisasi scroll Preview↔Split dan viewport | kenyamanan dokumen panjang | sedang–besar |
| 5 | Packaging AUR + release build | distribusi | sedang |

## 6. Basis perhitungan persentase

- **91 % roadmap**: 40 stage dideklarasikan (5a–5e, 6a–6p, 7a–7n, 8a–8e); semuanya
  diimplementasi dan lulus CI dengan gate lama utuh. Sisa ~9 % adalah kategori
  desktop/WYSIWYG/performa yang sengaja ditandai di luar scope stage.
- **44 % kesiapan rilis**: taksiran kualitatif. Fitur inti stabil dan teruji otomatis,
  tetapi tanpa validasi compositor, packaging, dan WYSIWYG umum, aplikasi belum layak
  disebut siap produksi.
- Bobot kategori pada bagian 3 dipilih mengikuti besar permukaan kode dan risiko,
  bukan jumlah baris. Semua angka bisa direvisi bila prioritas proyek berubah.

## 7. Referensi

- Kontrak & bukti terbaru: [Stage 8e](testing-stage8e.md) · [8d](testing-stage8d.md) · [8c](testing-stage8c.md) · [8b](testing-stage8b.md) · [8a](testing-stage8a.md) · [7n](testing-stage7n.md) · [7m](testing-stage7m.md) · [7l](testing-stage7l.md) · [7k](testing-stage7k.md) · [7j](testing-stage7j.md) · [7i](testing-stage7i.md) · [7h](testing-stage7h.md) · [7g](testing-stage7g.md) · [7f](testing-stage7f.md) · [7e](testing-stage7e.md) ·
  [7d](testing-stage7d.md) · [bukti GUI](testing-gui-x11.md) ·
  [verifikasi native](verification-native.md)
- Riwayat CI terakhir: `39963797627` (`97e586c`) **success** — 1026 Cargo / 335 strict
  core / 88 Python / 28 suite GUI 505 pemeriksaan (446 clipboard, 18 title,
  28 exit native), artefak `12507250289`, 4.172.220 bytes, expires
  2026-10-30T13:57:35Z.
- Verifikasi lokal: `cargo fmt --all -- --check`, 335 strict core (termasuk Clippy
  driver), 88 Python, dan audit AST 38 exercise-function definitions menunjukkan
  semua body lama selain dispatch `exercise_suite` tetap identik; 28 body gate yang
  dilindungi tetap utuh.
