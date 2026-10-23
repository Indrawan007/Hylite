# Laporan Progress — Hylite (snapshot Stage 7h)

**Tanggal:** 23 Oktober 2026 · **Branch:** `arena/21a4c00b-hylite` · **HEAD:** `0539b02`
**Basis:** commit `c2af096` lulus [CI 39898595716](https://github.com/Indrawan007/Hylite/actions/runs/39898595716)
Status kerja: **belum di-merge ke `main`**; dokumentasi 7h ditulis setelah CI lulus.

> Persentase di bawah adalah **estimasi terhadap roadmap bertahap yang dideklarasikan**,
> bukan hasil pengukuran otomatis dan bukan klaim kesiapan produksi. Dua angka sengaja
> dipisah agar tidak menyesatkan: progress roadmap ≠ kesiapan rilis.

## 1. Ringkasan persentase

| Sudut pandang | Angka | Arti |
|---|---|---|
| **Roadmap bertahap yang dideklarasikan** (Stage 5a–7h) | **≈ 85 %** | 29 dari 29 stage yang direncanakan sudah diimplementasi + diverifikasi otomatis |
| **Kesiapan rilis / produksi** (Arch + Hyprland, dokumen nyata) | **≈ 42 %** | belum ada validasi desktop target, packaging, maupun WYSIWYG umum |

Rincian bobot ada di [bagian 6](#6-basis-perhitungan-persentase). Angka 85 % tinggi
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

### Paragraf root & draft multiline (Stage 7a–7h) — selesai 100 %

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

### Verifikasi otomatis — selesai 100 % dari yang dirancang

| Metrik | Nilai |
|---|---|
| Tes Cargo | **910** lulus |
| Tes core mandiri (subset) | **294** |
| Helper Python | **63** |
| Suite GUI X11 | **20 suite / 394 pemeriksaan** (361 clipboard, 12 title, 21 exit native) |
| Gate lama yang dipertahankan | 19 suite sebelum 7g, 28 badan exercise identik AST |
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
| Draft paragraf + multiline + history + Apply & Next/Previous + shortcut commit (7a–7g) | 15 % | 100 % | 15,0 |
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

- **85 % roadmap**: 29 stage dideklarasikan (5a–5e, 6a–6p, 7a–7h); semuanya
  diimplementasi dan lulus CI dengan gate lama utuh. Sisa ~16 % adalah kategori
  desktop/WYSIWYG/performa yang sengaja ditandai di luar scope stage.
- **42 % kesiapan rilis**: taksiran kualitatif. Fitur inti stabil dan teruji otomatis,
  tetapi tanpa validasi compositor, packaging, dan WYSIWYG umum, aplikasi belum layak
  disebut siap produksi.
- Bobot kategori pada bagian 3 dipilih mengikuti besar permukaan kode dan risiko,
  bukan jumlah baris. Semua angka bisa direvisi bila prioritas proyek berubah.

## 7. Referensi

- Kontrak & bukti terbaru: [Stage 7h](testing-stage7h.md) · [7g](testing-stage7g.md) · [7f](testing-stage7f.md) · [7e](testing-stage7e.md) ·
  [7d](testing-stage7d.md) · [bukti GUI](testing-gui-x11.md) ·
  [verifikasi native](verification-native.md)
- Riwayat CI terakhir: `39898595716` (`c2af096`) **success** — 910 Cargo / 394 GUI,
  artefak `12477769679`, 3.253.627 bytes, SHA256
  `11c3fd76f5e200e24e196271113e81a580f6ff842a238ee72e5b60cff9143934`.
