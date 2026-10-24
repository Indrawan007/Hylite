# Rancangan scope — Stage 10a incremental parse

**Status: Stage 10a produksi diimplementasikan pada branch `arena/21a4c00b-hylite`;**
scope tetap terbatas pada cache/parser Preview yang ada. Stage 9c/9d tetap
ditunda; tidak ada marker hiding, pemetaan per glyph, editing inline, atau
pensiun panel draft.

## 1. Temuan dari jalur kode saat ini

- `Document::preview_blocks()` menyimpan satu `OnceCell<PreviewCache>` untuk
  revision aktif: snapshot, Source tepat, dan model block. Saat cache miss,
  cache revision sebelumnya boleh menjadi basis incremental hanya jika
  `document_id` sama dan revision-nya lebih lama.
- Setelah mutasi, cache aktif dipindah ke slot basis sebelumnya. Saat proyeksi
  diminta, kode membandingkan Source tersimpan dengan Source saat ini untuk
  memperoleh diff byte yang tepat; tanpa basis/cache yang valid, hasilnya full
  parse. Penggantian dokumen menginisialisasi ulang kedua slot.
- Model `Block` menyimpan rentang Source absolut; anak, inline runs, marker
  heading, payload paragraf, task, dan sel tabel juga membawa provenance/rentang.
  Aksi Preview dan validasi target saat ini bergantung pada rentang serta
  snapshot yang autentik.
- Mutasi tidak datang dari satu jalur: ada ketikan native/Undo/Redo, `TextChange`,
  operasi blok, tabel/task, penggantian buffer, dan rencana kandidat. Invalidation
  lokal tidak boleh menghidupkan ulang target dari revision lama.

Stage 9d (editing per glyph di Preview) **bukan prasyarat teknis** agar parser
membantu Preview yang sudah ada: Preview dan operasi block-level sekarang tetap
meminta model yang sama. Jadi, 10a dapat dipertimbangkan sebagai optimasi
independen untuk produk saat ini. Pernyataan roadmap lama bahwa 10a “baru
bermakna setelah 9d” adalah keputusan urutan/proyek yang perlu direvisi, bukan
kendala API yang terlihat pada jalur parser ini.

## 2. Batas implementasi produksi

- Perubahan hanya menyentuh parser/cache Preview. Grammar Markdown, bentuk
  `Block`, widget, alur editing, Source of truth, focus/refocus, dan scope 9c/9d
  tidak diubah.
- Fast path sengaja sempit: diff Source harus berada di satu root paragraph
  satu baris yang telah diparse sebagai satu run plain tanpa style; batas blok
  harus terisolasi oleh awal/akhir dokumen atau blank line; konten baru harus
  tetap satu paragraph plain saat di-parse tersendiri. Baris dengan bracket
  ditolak karena reference link/footnote bisa bergantung pada definisi global.
- Hanya blok itu yang di-parse ulang. Semua blok sesudahnya disalin dan seluruh
  rentang absolut dipindahkan—termasuk anak, runs, marker heading/Setext, marker
  task, row/cell/delimiter/body-line/editable-cell tabel. Prefix tidak berubah.
  Overflow, rentang tidak konsisten, cache tanpa identitas dokumen/revision yang
  valid, beberapa blok berubah, newline, inline/block syntax, atau konteks lain
  yang tidak terbukti aman memilih full parse.
- Dirty range berasal dari diff byte atas Source sebelumnya yang disimpan dan
  Source aktif, sehingga tetap exact untuk jalur ketikan, transaksi, Undo/Redo,
  dan mutasi lain. Diff ini men-scan Source penuh; karena itu implementasi ini
  belum membuktikan bahwa incremental lebih cepat. Jalur preflight yang sudah
  melakukan full parse tetap menjadi validasi kanonis; cache kandidat hanya
  dipasang bila Source persis cocok dengan buffer aktif.
- Tidak ada klaim performa. Harness Stage 10b sekarang mencatat dua observasi
  tambahan: cache-miss setelah satu edit plain yang memenuhi fast path, dan
  cache-miss setelah newline yang memaksa fallback. Keduanya tetap satu-run
  environment observations, bukan threshold atau perbandingan lintas mesin.

## 3. Oracle, pengujian, dan gate yang dipertahankan

1. Fast-path helper membandingkan hasil splice incremental secara `Block`-exact
   terhadap `preview_model::parse` atas Source lengkap, termasuk equality semua
   nesting, run/style, marker, rentang Source absolut, task, dan tabel. Pengujian
   juga memastikan fallback untuk newline/syntax tak aman serta reference
   context lintas dokumen.
2. Uji cache mencakup warm parse → edit plain Unicode → equality oracle, beberapa
   edit sebelum Preview dibaca, dan invalidasi lama. Property tests yang sudah
   ada tidak diubah atau dilemahkan.
3. Mutasi tetap terpusat pada revision cache: basis lama membawa `Snapshot`
   dan Source; basis ditolak bila `document_id` tidak cocok atau revision tidak
   lebih lama. Cache preflight dipasang untuk revision aktif hanya setelah
   Source kandidat dicocokkan lagi dengan buffer. Target stale tetap ditolak
   oleh snapshot dan autentikasi yang sudah ada.
4. CI menjalankan gate format, strict Source core, strict Clippy, build, seluruh
   unit/property tests, GUI smoke lama, dan harness Stage 10b tanpa mengurangi
   assertion, oracle byte/coordinate, deadline, atau focus/refocus.
5. Ukuran 100 KB/1 MB/5 MB untuk incremental yang eligible dan fallback dicatat
   terpisah sebagai observasi runner, tanpa klaim performa atau threshold.

## 4. Keputusan dan status

- **Kesimpulan teknis:** 10a dapat berdiri sendiri dari 9c/9d untuk Preview
  saat ini.
- **Implementasi:** produksi fast path yang konservatif berada di
  `src/document/preview_incremental.rs`; cache/revision provenance berada di
  `src/document/state.rs`. Full parse tetap oracle dan fallback.
- **Scope:** hanya paragraph satu baris plain yang terisolasi dapat di-reparse.
  Perubahan lain sengaja full parse. Diff scan Source penuh dan cloning/range
  shifting seluruh suffix berarti status ini bukan klaim percepatan.
- **Stage 9c/9d:** tetap ditunda sesuai pilihan pengguna A. Tidak ada marker
  hiding, mapping per glyph, editing inline, atau pensiun panel draft.
- **Estimasi:** progress roadmap ≈93% (denominator Stage 5a–9b) dan kesiapan
  rilis ≈44% tetap terpisah; Stage 10a tidak mengubah kedua denominator itu.
