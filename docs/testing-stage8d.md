# Stage 8d — ekspor PDF tanpa browser

Sampai Stage 8c satu-satunya ekspor adalah HTML dengan CSS inline. Stage ini
menambahkan **PDF** yang ditulis sendiri: tanpa browser, tanpa `wkhtmltopdf`,
tanpa crate PDF, dan tanpa font yang diunduh.

Yang dikerjakan:

- `src/export/pdf.rs` — penulis PDF 1.4 (objek, xref, trailer, stream) plus
  tata letak: heading H1–H4, paragraf dengan wrap, quote (garis aksen), list
  berbutir/nomor, rule, tabel dengan border dan header berulang, dan code block
  monospace.
- `src/export/pdf_build.rs` — satu-satunya tempat yang memetakan proyeksi
  Preview (`preview_model`) ke item tata letak. Lapisan PDF tidak tahu apa itu
  Markdown; pemetaan bisa diuji tanpa renderer, dan renderer tanpa parser.
- `src/export/mod.rs` — `export_pdf(markdown, palette, path)` memakai
  `files::atomic_write`, jadi file tujuan ditulis utuh atau tidak sama sekali.
- `hylite export-pdf <input.md> <output.pdf>` — jalur tanpa jendela dan tanpa
  dialog simpan, untuk skrip dan untuk gerbang CI.
- Toolbar **PDF** dan `Ctrl+Shift+P` — dialog simpan native, palet dari tema
  yang sedang aktif (`Ctrl+Shift+E` tetap HTML).

## Pemetaan dokumen → halaman

| Preview | PDF |
|---|---|
| `Heading(1..=4)` | tebal, 21 / 17 / 14,5 / 12,5 pt, warna aksen |
| `Paragraph` | 11 pt, leading 15,6 pt, wrap greedy + hard-split untuk kata panjang |
| `Code` | Courier 8,5 pt, latar `surface`, baris dipertahankan apa adanya |
| `Quote` | indentasi 14 pt + garis aksen 2 pt |
| `List(start)` | penanda `•` atau `N.` dan indentasi 18 pt per level |
| `Table` | kolom sama lebar, border 0,5 pt, padding 4 pt, selaraskan kiri/tengah/kanan |
| `Rule` | garis 0,5 pt selebar area isi |
| `Literal` (HTML mentah, math, batas preview) | teks verbatim di blok kode |

Halaman A4 (595 × 842 pt), margin 56 pt. Tabel mengulang baris header setiap kali
pindah halaman, dan satu baris tabel yang lebih tinggi dari sisa halaman
dipindahkan utuh, bukan dipotong.

## Font, teks, dan kejujuran encoding

- Font base-14 `/Helvetica`, `/Helvetica-Bold`, `/Helvetica-Oblique`, `/Courier`.
  **Tidak ada font yang ditanam**: pembaca PDF wajib punya font standar itu.
- Encoding `/WinAnsiEncoding`. Karakter WinAnsi dikirim sebagai **escape oktal**
  (mis. `é` → `\351`) supaya byte-nya tepat dan stream tetap ASCII; karakter di
  luar WinAnsi menjadi `?`. Ini satu-satunya langkah lossy dan dinyatakan di sini.
- Lebar teks memakai tabel lebar Helvetica (95 glyph ASCII) dan 600/1000 untuk
  Courier — perkiraan, bukan metrik font sebenarnya. Wrap bisa sedikit berbeda
  dari pembaca PDF yang memakai metrik asli.
- Tanpa kompresi: stream disimpan apa adanya sehingga byte-nya deterministik dan
  mudah diaudit, dengan konsekuensi file lebih besar dari yang seharusnya.

## Gerbang otomatis

| Lapisan | Bukti |
|---|---|
| `pdf.rs` (12 uji) | xref menunjuk tepat ke header setiap objek, `/Length` sama dengan panjang stream, paginasi, margin, header tabel berulang, escape, clamp warna, hard-split |
| `pdf_build.rs` (6 uji) | heading/paragraf/rule, code byte-exact, tabel (header, baris, selaras), quote + list (termasuk nested), gambar dibuang, math verbatim, dokumen kosong |
| `export/mod.rs` (3 uji) | menulis PDF valid, dokumen kosong tetap satu halaman, error I/O diteruskan |
| `export_cli.rs` (5 uji) | subcommand dikenali, jumlah argumen salah = exit 2, sukses menulis PDF, input hilang = exit 1 tanpa file, menolak menimpa Markdown |
| GUI suite `pdf` (10 pemeriksaan) | header `%PDF-1.4`, xref resolve setiap objek, `/Length` cocok, heading/bold/code/tabel di stream, `é` bertahan dan emoji jadi `?`, Markdown byte-identik, penolakan timpa + input hilang tanpa file, usage exit 2, dokumen GUI tetap utuh dan klien keluar natural |

Suite GUI dijalankan dengan binary yang sama; 4 pemeriksaan pertama memakai
`export-pdf`, sisanya memastikan jendela yang sedang terbuka tidak berubah.

## Batas cakupan (disengaja)

- **Tidak ada dialog simpan yang diotomasi**: aksi toolbar dan `Ctrl+Shift+P`
  memakai dialog native yang tidak bisa didorong lewat Xvfb. Yang diuji adalah
  jalur CLI-nya; perilaku dialog sama persis dengan ekspor HTML sebelumnya, dan
  itu dinyatakan, bukan disembunyikan.
- Gambar di Preview **tidak** masuk PDF; hanya teksnya. Link menjadi teks, tidak
  menjadi anotasi. Math tetap teks sumber.
- Tidak ada outline/bookmark, nomor halaman, header/footer, margin kustom,
  ukuran halaman selain A4, atau kompresi.
- Tanpa font embedding berarti dokumen beraksara non-Latin tidak bisa tampil
  benar; karakter tersebut menjadi `?` dan itu terlihat langsung.
- Tidak ada oracle piksel: "tampilannya benar" tidak diklaim. Yang diuji adalah
  struktur PDF, isi stream, dan byte sumber yang tidak berubah.

## Angka

- Cargo: **1014** uji (sebelumnya 988).
- Core harness: **326** uji (sebelumnya 314).
- Python: **85** uji (sebelumnya 79).
- GUI: **27 suite / 498 pemeriksaan** (sebelumnya 26 / 488).
- CI: [`39956545830`](https://github.com/Indrawan007/Hylite/actions/runs/39956545830)
  pada `f9d160b` — **success**, artefak `12503736448` (3.951.707 bytes).
