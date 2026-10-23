# Laporan Progress — Hylite (snapshot Stage 7f)

**Tanggal:** 23 Oktober 2026 · **Branch:** `arena/21a4c00b-hylite` · **HEAD:** `e8b20f3`
**Basis:** commit `0353f5e` lulus [CI 39892699395](https://github.com/Indrawan007/Hylite/actions/runs/39892699395)
Status kerja: **belum di-merge ke `main`**, working tree bersih, dokumentasi 7f sudah dipush.

> Persentase di bawah adalah **estimasi terhadap roadmap bertahap yang dideklarasikan**,
> bukan hasil pengukuran otomatis dan bukan klaim kesiapan produksi. Dua angka sengaja
> dipisah agar tidak menyesatkan: progress roadmap ≠ kesiapan rilis.

## 1. Ringkasan persentase

| Sudut pandang | Angka | Arti |
|---|---|---|
| **Roadmap bertahap yang dideklarasikan** (Stage 5a–7f) | **≈ 82 %** | 27 dari 27 stage yang direncanakan sudah diimplementasi + diverifikasi otomatis |
| **Kesiapan rilis / produksi** (Arch + Hyprland, dokumen nyata) | **≈ 40 %** | belum ada validasi desktop target, packaging, maupun WYSIWYG umum |

Rincian bobot ada di [bagian 6](#6-basis-perhitungan-persentase). Angka 82 % tinggi
karena scope yang dideklarasikan memang sengaja sempit: editor native + prototipe
editing blok terbatas. Gap terbesar ada pada dua baris terakhir tabel di atas.

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

### Paragraf root & draft multiline (Stage 7a–7f) — selesai 100 %

| Stage | Isi |
|---|---|
| 7a | Draft paragraf root satu baris (raw Markdown) |
| 7b | Draft paragraf multiline native 2–128 baris, ≤16 KiB |
| 7c | History lokal bounded (100 record / 1 MiB / 16 KiB per record) |
| 7d | Previous/Next antar paragraf multiline, khusus draft unchanged |
| 7e | Apply & Next atomik dengan preflight kandidat penuh |
| 7f | Apply & Previous atomik, offset root sebelumnya tetap diverifikasi ulang |

### Verifikasi otomatis — selesai 100 % dari yang dirancang

| Metrik | Nilai |
|---|---|
| Tes Cargo | **903** lulus |
| Tes core mandiri (subset) | **294** |
| Helper Python | **61** |
| Suite GUI X11 | **19 suite / 378 pemeriksaan** (346 clipboard, 12 title, 20 exit native) |
| Gate lama yang dipertahankan | 18 suite sebelum 7f, 28 badan exercise identik AST |
| Lint/format | `cargo fmt` + strict Clippy `-D warnings` + build debug |

### Yang belum dikerjakan

| Area | Kondisi |
|---|---|
| Validasi desktop Arch + Hyprland/Wayland | **belum** — semua bukti dari Xvfb + tiny-skia |
| Portal/dialog file nyata, IME, RTL, HiDPI, GPU | **belum** |
| Selection arah/anchor, mapping glyph, drag, multi-cell | **belum** |
| Sinkronisasi scroll Preview↔Source | **belum** |
| WYSIWYG inline bebas ala MarkText | **belum** (masih panel draft terbatas) |
| Incremental parsing / klaim performa | **belum** — semua jalur O(n) salin+parse |
| Packaging/AUR/release build | **belum** |
| Checklist manual Tahap 2–4 | **belum dicentang** |

## 3. Progres per kategori (bobot estimasi)

| Kategori | Bobot | Capaian | Kontribusi |
|---|---|---|---|
| Sumber kebenaran Source, history, file I/O, save/export | 20 % | 100 % | 20,0 |
| Renderer preview + Split + navigasi/refocus | 15 % | 100 % | 15,0 |
| Editing blok tabel/task/heading (6a–6p) | 25 % | 100 % | 25,0 |
| Draft paragraf + multiline + history + Apply & Next/Previous (7a–7f) | 15 % | 100 % | 15,0 |
| Verifikasi otomatis & CI | 15 % | 100 % | 15,0 |
| Validasi desktop target + manual checklist | 5 % | 20 % | 1,0 |
| WYSIWYG umum, selection/glyph, scroll sync, performa | 5 % | 10 % | 0,5 |
| **Total** | **100 %** | | **≈ 81,5 %** |

## 4. Risiko & batasan yang masih terbuka

1. **Belum ada bukti di compositor target.** X11 tiny-skia bukan bukti Hyprland/Wayland;
   shortcut bisa diambil compositor, fokus/IME belum diuji nyata.
2. **Belum siap dokumen penting.** History bounded, LF canonical, selection collapse,
   dan draft pane terbatas masih kompromi prototipe.
3. **O(n) di banyak jalur** (parse penuh, salinan Source, dua buffer native di plan
   7e/7f) — memori ekstra, belum ada optimasi.
4. **Statistik tes bukan cakupan.** 294 core adalah subset dari 903 Cargo; angka GUI
   membuktikan byte/fokus/urutan, bukan piksel atau mapping glyph.
5. **Screenshot artefak belum direview visual** dan bukan oracle piksel.

## 5. Saran langkah berikutnya (perkiraan usaha)

| Prioritas | Kandidat | Nilai | Usaha |
|---|---|---|---|
| 1 | Audit manual Arch + Hyprland (checklist Tahap 2–4, IME/RTL/HiDPI) | menutup gap terbesar | sedang |
| 2 | Shortcut Apply/Cancel konsisten untuk semua panel draft | ergonomi nyata | kecil |
| 3 | Navigasi Apply & Next lintas jenis blok terkontrol | memperluas alur editing | sedang |
| 4 | Sinkronisasi scroll Preview↔Split | kenyamanan editing | sedang |
| 5 | Selection arah/mapping glyph | prasyarat WYSIWYG | besar |
| 6 | Packaging AUR + release build | distribusi | sedang |

## 6. Basis perhitungan persentase

- **82 % roadmap**: 27 stage dideklarasikan (5a–5e, 6a–6p, 7a–7f); semuanya
  diimplementasi dan lulus CI dengan gate lama utuh. Sisa ~18 % adalah kategori
  desktop/WYSIWYG/performa yang sengaja ditandai di luar scope stage.
- **40 % kesiapan rilis**: taksiran kualitatif. Fitur inti stabil dan teruji otomatis,
  tetapi tanpa validasi compositor, packaging, dan WYSIWYG umum, aplikasi belum layak
  disebut siap produksi.
- Bobot kategori pada bagian 3 dipilih mengikuti besar permukaan kode dan risiko,
  bukan jumlah baris. Semua angka bisa direvisi bila prioritas proyek berubah.

## 7. Referensi

- Kontrak & bukti terbaru: [Stage 7f](testing-stage7f.md) · [7e](testing-stage7e.md) ·
  [7d](testing-stage7d.md) · [bukti GUI](testing-gui-x11.md) ·
  [verifikasi native](verification-native.md)
- Riwayat CI terakhir: `39892699395` (`0353f5e`) **success** — 903 Cargo / 378 GUI,
  artefak `12475384207`, 3.279.313 bytes, SHA256
  `92636145659d325378e7c3073665a601d74c93bb1dcfa1bc6b1f95f4574b06a2`.
- CI dokumentasi `e8b20f3` (`39893714451`) hanya memvalidasi dokumen, bukan hasil fitur.
