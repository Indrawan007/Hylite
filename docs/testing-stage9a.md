# Stage 9a — spike inline terbatas

**Tanggal:** 24 Oktober 2026 · **Branch:** `arena/21a4c00b-hylite` · **Kode:** `7b8f82f` · [CI 40039268185](https://github.com/Indrawan007/Hylite/actions/runs/40039268185) hijau

Stage ini menguji satu prasyarat teknis sebelum keputusan terpisah tentang 9b:
apakah native Iced `text_editor` bisa mempertahankan caret byte/karakter pada
teks yang terlihat tebal, dan apakah satu ketikan di dalam teks itu dapat
menghasilkan splice Source yang tepat. Buktinya **positif untuk gerak keyboard
ASCII satu-baris yang diuji**: caret native berakhir di byte tampak 7, lalu satu
`X` menghasilkan Source delta yang tepat; Undo/Redo memulihkan kedua versi.
Ini **spike opt-in**, bukan pengganti panel draft dan bukan dukungan WYSIWYG umum.
Bukti ini tidak mencakup hit-testing klik per glyph atau mapping selection.

## Kontrak spike

- Diaktifkan hanya dengan `hylite --inline-edit-spike <file-yang-ada>`.
  Default Preview, Source, Split, panel draft, dan gate lama tetap memakai jalur
  biasa. Spike otomatis memilih paragraf root satu-baris pertama yang lolos
  subset; bila tidak ada, Source tetap utuh dan tidak ada editor inline.
- Maksimum 16 KiB ASCII dalam satu baris fisik; paragraf harus dapat diproyeksikan
  tanpa ambigu untuk run `**strong**`, `*emphasis*`, dan `` `code` ``.
  Link, entity, HTML, math, strike, escape, markup bertingkat, gabungan style,
  teks Unicode/control, dan baris struktural lain ditolak.
- Editor native menampilkan teks yang terlihat saja. Range format highlight
  memakai byte range ASCII pada teks tampak; delimiter Source tidak ikut masuk
  ke widget. Selama spike aktif, judul jendela menampilkan `[Stage 9a caret N]`
  sebagai diagnostik posisi byte tampak (bukan koordinat glyph atau oracle piksel).
- Hanya gerakan caret dan satu karakter ASCII aman untuk insert/delete. Paste,
  seleksi, drag, baris baru, delimiters Markdown, atau aksi yang tak terverifikasi
  ditolak. Seleksi yang muncul atau caret tanpa posisi byte membatalkan aksi;
  kandidat kemudian diparse ulang dan wajib tetap menjadi root paragraph dengan
  teks tampak yang sama sebelum satu `Document::change` atomik dilakukan.
- Source adalah sumber kebenaran dan menerima perubahan langsung. Undo/Redo tetap
  melalui history dokumen yang sudah ada. `Esc` atau tombol **Finish inline spike**
  menutup editor eksperimen; semua panel draft lama tetap tersedia tanpa perubahan.

## Reproduksi

Dengan file satu baris seperti fixture smoke di bawah:

```sh
hylite --inline-edit-spike fixture.md
```

Untuk mengulang gate GUI X11/tiny-skia (tidak memerlukan dialog file):

```sh
./scripts/test-gui-x11.sh --suite inline-spike \
  --artifacts /tmp/hylite-stage9a
```

Suite memulai lewat flag opt-in dan memastikan judul menunjukkan caret byte 0.
Untuk fokus keyboard, harness mencoba titik klik aman pada `x=600`, `y=120..400`
(step 8); setelah setiap klik ia menekan `Home` lalu `Right` dan hanya menerima
fokus ketika judul native berubah ke caret byte 1. `Home` berikutnya mereset
posisi klik; klik ini **tidak** dipakai untuk menguji hit-testing glyph. Setelah
itu tujuh `Right` harus membuat judul menunjukkan caret byte 7, lalu suite
mengetik `X`. Setelah `Esc`, suite berpindah ke Split dan membaca Source widget
melalui clipboard. Fixture byte-exact dan oracle-nya:

```text
sebelum: left **bold** right\n
sesudah: left **boXld** right\n
```

Sesudah itu satu Undo harus mengembalikan fixture persis, Redo harus
mengembalikan hasil persis, Undo terakhir membersihkan dokumen, dan jendela harus
keluar alami. Artefak screenshot bernama `inline-spike-before.png`,
`inline-spike-caret-in-bold.png`, `inline-spike-inserted.png`, dan
`inline-spike-split.png`; screenshot adalah catatan visual, **bukan** oracle piksel.

Tes otomatis yang menjadi pagar sebelum suite GUI:

- `src/inline_edit.rs` ikut dikompilasi oleh `scripts/test-source-core.sh`; tes
  memeriksa proyeksi, offset byte, batas style, splice insert/delete tepat
  (termasuk karakter berulang yang ditambatkan ke caret), dan penolakan input
  ambigu.
- `src/ui/inline_spike.rs` memeriksa posisi caret byte `text_editor::Content`
  sepanjang run strong dan memastikan highlighter hanya menandai range teks
  tampak. Tes ini menguji `Content`/aksi native; bukan renderer atau hit-testing.
- `src/app/tests.rs` menjalankan aksi native ke controller dan memeriksa delta
  persis `left **bold** right\n` → `left **boXld** right\n`, marker caret pada
  judul, Undo/Redo, serta rollback Source/history ketika kandidat menghilangkan
  kelayakan paragraf root.
- `scripts/test_gui_smoke.py` mengunci fixture, urutan keyboard, oracle Source,
  dispatch suite eksklusif, dan syarat dokumen untuk harness GUI.

## Cara membaca bukti dan batas klaim

Bukti GUI yang lulus mencakup **keyboard caret motion ASCII pada satu baris** di
X11 + Xvfb + tiny-skia: judul mencatat caret byte 0 → 1 (fokus) → 0 (Home) → 7,
dan satu `X` pada posisi itu mengubah Source ke `left **boXld** right\n`.
Undo dan Redo mengembalikan fixture/hasil secara byte-exact. Suite inline-spike
lulus 7 pemeriksaan; seluruh CI lulus 1041 tes Cargo, 344 tes strict core, 90
tes Python, serta 29 suite GUI / 512 pemeriksaan. Artefak `12538559588`
(4.415.012 bytes) tersedia sampai 2026-10-30T23:36:23Z.

Ini **belum** membuktikan hit-testing klik per glyph—klik harness hanya mencari
fokus, lalu `Home` menghapus pengaruh posisi klik—atau kestabilan pada font,
ukuran/zoom lain, seluruh titik caret yang dirender, seleksi dua arah, IME,
Unicode, RTL, ZWJ, paste, drag, scroll, GPU/HiDPI, portal, maupun
Hyprland/Wayland. Tes `Content` bukan pengganti bukti renderer native; hasil ini
tidak membenarkan klaim WYSIWYG.

**Keputusan setelah Stage 9a:** lanjut hanya ke **feasibility probe Stage 9b**
untuk hit-testing/caret glyph dan anchor/arah selection, bukan implementasi
WYSIWYG umum. Bukti positif saat ini terbatas pada byte-column melalui keyboard;
probe berikutnya harus punya oracle GUI terpisah dan berhenti bila pemetaan tidak
stabil. Stage 9b sendiri belum dimulai. Stage 9a tidak menambah dependency dan
tidak mengubah gate lama.
