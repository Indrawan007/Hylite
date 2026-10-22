# Stage 5b — tabel Markdown native read-only

Melanjutkan [fondasi Stage 5a](testing-stage5a.md), bukan WYSIWYG dan bukan
pernyataan seluruh Tahap 5 selesai. Source tetap satu buffer Iced otoritatif.
Tidak mengubah serializer, save/export, history, konfigurasi, dependensi atau
`Cargo.lock`. Tabel menggunakan extension GFM parser yang sudah diaktifkan.

## Cakupan implementasi

- `preview_table.rs`: model owned table/row/cell, alignment default/left/center/
  right, header vs body, inline runs dan rentang provenance byte UTF-8.
- `preview_table_view.rs`: grid Iced native, header bold + background tema,
  border/rule, alignment teks per kolom, inline bold/italic/strike/code/link.
- Lebar setiap kolom **160 logical pixels termasuk padding**; teks wrap
  word-or-glyph di dalamnya. Tabel lebar memakai scroll horizontal di dalam
  preview vertikal, termasuk pada Split. Belum auto-size, resize kolom, sticky
  header, cell selection, copy table, atau edit sel.
- Tinggi baris mengikuti isi tertinggi. Divider vertikal memakai layer native
  setelah pengukuran row teks; tidak mengubah parent menjadi Fill-height dalam
  scroll vertikal. Sel kosong mendapat spacer layout saja, bukan karakter baru
  dalam model atau Source.
- Table/row/cell menyimpan provenance. Sel kosong yang dipad parser dapat memiliki
  rentang kosong di akhir row; bukan byte index/glyph/caret mapping.

### Semantik parser dan keselamatan

Lebar semantik ditentukan header GFM. Body row yang kurang sel dipad kosong;
body row berlebih mengabaikan sel ekstra **di preview sesuai parser GFM**.
Sel ekstra tetap ada dalam Source dan row provenance, bukan penghapusan dokumen.
Pipe literal perlu escape `\|`, termasuk dalam code span, mengikuti parser.
Baris tanpa delimiter valid tetap Markdown biasa, bukan dipaksa menjadi tabel.

HTML/math/footnote di dalam sel tetap literal, image placeholder tidak membaca
file/jaringan, dan label link hanya diberi gaya tanpa action URL. Semua sel memakai
parser inline dan renderer span inert yang sama dengan paragraph Stage 5a.

### Batas eager preview

Tabel hanya menjadi grid bila memenuhi **semua** batas berikut:

| Batas per tabel | Nilai inklusif |
| --- | ---: |
| Kolom | 12 |
| Row termasuk header | 128 |
| Sel termasuk header dan sel kosong/padded | 512 |

Jika salah satu terlampaui, **seluruh rentang source tabel** ditampilkan dalam
panel `Table preview limit — source`; bukan hanya baris yang sempat dibaca.
Parser mengonsumsi sampai akhir tabel tanpa menelan akhir quote/container atau
block berikutnya. Batas block/inline nesting Stage 5a tetap berlaku.

Ini batas jumlah widget tabel eager, **bukan** batas ukuran source, waktu parser,
memori total, jumlah tabel per dokumen atau proteksi DoS keseluruhan. Belum ada
cache, incremental parse, virtualisasi atau optimasi file besar. Source fallback
sendiri tetap dapat panjang.

## Pengujian

- Model preview kini **35 tes**: 22 non-table Stage 5a, penggantian satu tes
  fallback tabel menjadi tes struktur, serta 12 tes tabel tambahan. Mencakup
  empat alignment, header-only, sel kosong, ragged rows, style isolation,
  Unicode/entity/escaped pipe, quote/list, inert inline content, batas inklusif,
  fallback seluruh source dan tail setelah tabel/quote.
- **2 tes adapter tabel baru**: mapping alignment native dan pembangunan widget
  pada Light/Dark dengan sel kosong/untrusted inline; size hint vertikal harus
  Shrink agar aman di outer scroll. Bukan visual/pixel oracle.
- Tes invariant controller/view yang sudah ada diperluas dengan tabel normal
  dan tabel melebihi batas: text raw/serialized, revision/render key, undo/redo,
  dirty, caret dan selection harus tetap sama setelah view dibangun.
- Runner X11 menambahkan **4 pemeriksaan clipboard** pada fixture 8 kolom,
  alignment/styles/Unicode/escaped pipes, missing/extra cells, tabel di quote,
  dan tabel 13 kolom fallback. Input pada Preview dan wheel horizontal tidak
  boleh mengubah Source. Mode cycle harus byte-exact sampai terminal tab; satu
  Undo kembali ke welcome clean. Copy setelah mode change mengklik Source,
  **bukan klaim auto-focus**. Semua 67 pemeriksaan GUI lama tetap aktif.
- Satu helper Python baru memeriksa fixture byte-exact dan protokol wheel/mode
  cycle; helper bukan pengganti GUI sebenarnya.

`tables-native.png`, `tables-horizontal.png`, dan `tables-split.png` direkam
sebagai artifact runtime. Pengiriman wheel dan screenshot **tidak membuktikan
secara otomatis posisi scrollbar, batas sel, glyph atau layout visual benar**.

## Status eksekusi

Lokal: 35 tes model dengan source parser versi lockfile lulus melalui
`clippy-driver -D warnings`, 150 tes core Source dan 16 helper Python lulus;
format dan `git diff --check` lulus. Gate native Cargo/Iced/Clippy/build dan GUI
CI masih menunggu hasil; hasil akan dicatat setelah benar-benar diamati.

Gate standar tetap memakai `--locked --all-targets --all-features`, Clippy
`-D warnings`, serta Xvfb/XDG privat. Tidak ada test hook aplikasi, pengurangan
assertion atau perluasan waktu gate untuk menutupi kegagalan.

## Belum diverifikasi / berikutnya

- [ ] Review visual Light/Dark: alignment, header, wrap baris terakhir yang paling
  tinggi, border/divider, empty rows dan scrollbar setelah resize/Split.
- [ ] Runtime Arch + Hyprland/Wayland, GPU/HiDPI, IME/RTL dan font fallback.
- [ ] Benchmark tabel/dokumen besar; tentukan cache/incremental/virtualisasi.
- [ ] Source↔selection mapping sebelum prototipe WYSIWYG, bukan menafsirkan
  provenance range sebagai pemetaan caret siap pakai.

Portal file, save-before-close end-to-end, packaging Arch dan kesiapan produksi
tetap di luar bukti renderer ini. Tidak ada merge ke `main`.
