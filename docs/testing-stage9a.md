# Stage 9a — spike inline terbatas

**Tanggal:** 23 Oktober 2026 · **Branch:** `arena/21a4c00b-hylite`

Stage ini menguji satu prasyarat teknis sebelum keputusan terpisah tentang 9b:
apakah native Iced `text_editor` bisa mempertahankan caret berbasis karakter saat
bergerak melewati teks yang ditampilkan tebal, dan apakah satu ketikan di dalam
teks itu dapat menghasilkan splice Source yang tepat. Ini **spike opt-in**, bukan
pengganti panel draft dan bukan dukungan WYSIWYG umum.

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
  ke widget.
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

Suite memulai lewat flag opt-in, memfokuskan editor, menekan `Home`, bergerak
kanan tujuh langkah melewati karakter-karakter tampak, lalu mengetik `X`.
Setelah `Esc`, suite berpindah ke Split dan membaca Source widget melalui
clipboard. Fixture byte-exact dan oracle-nya:

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
  tampak.
- `src/app/tests.rs` menjalankan aksi native ke controller dan memeriksa delta
  persis `left **bold** right\n` → `left **boXld** right\n`, Undo/Redo, serta
  rollback Source/history ketika kandidat menghilangkan kelayakan paragraf root.
- `scripts/test_gui_smoke.py` mengunci fixture, urutan keyboard, oracle Source,
  dispatch suite eksklusif, dan syarat dokumen untuk harness GUI.

## Cara membaca bukti dan batas klaim

Bukti GUI yang lulus hanya mencakup **keyboard caret motion ASCII pada satu
baris dan satu posisi di dalam strong run pada X11 + Xvfb + tiny-skia**. Itu
menunjukkan bahwa satu karakter yang diketik di sana masuk ke byte Source yang
diharapkan. Ini belum membuktikan hit-testing klik per glyph, kestabilan pada
font/ukuran/zoom lain, seluruh posisi caret, seleksi dua arah, IME, Unicode,
RTL, ZWJ, paste, drag, scroll, GPU/HiDPI, portal, atau Hyprland/Wayland. Tes
`Content` saja bukan pengganti bukti renderer native; hasil GUI tetap bersifat
sempit dan tidak membenarkan klaim WYSIWYG.

Stage 9b **belum dimulai**. Keputusan tentang mapping glyph/selection dua arah
harus memakai hasil gate ini dan tetap mempertimbangkan keterbatasan di atas;
spike ini tidak menambah dependency dan tidak mengubah gate lama.
