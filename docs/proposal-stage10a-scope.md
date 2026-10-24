# Rancangan scope — Stage 10a incremental parse

**Status: evaluasi teknis saja; implementasi belum disetujui dan belum dimulai.**
Keputusan pengguna untuk menunda Stage 9c/9d mengizinkan penilaian ulang
ketergantungan, bukan perubahan kode parser/editor produksi.

## 1. Temuan dari jalur kode saat ini

- `Document::preview_blocks()` menyimpan satu `OnceCell<Vec<Block>>` untuk
  seluruh proyeksi dokumen. Saat cache miss, ia membuat teks dari native buffer
  lalu memanggil `preview_model::parse` atas **seluruh** teks.
- `Document::advance_revision()` menaikkan revision dan menghapus proyeksi itu
  setelah edit efektif. Tidak ada dirty range atau identitas cache per blok saat
  ini.
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

## 2. Batas scope yang disarankan

- Tujuan hanya mengurangi kerja parse ulang Preview pada edit tertentu; tidak
  mengubah grammar Markdown, bentuk model, perilaku widget, Source sebagai
  sumber kebenaran, atau alur editing.
- Cache penuh tetap oracle dan fallback. Jika batas aman reparse tidak dapat
  dibuktikan—misalnya perubahan mengubah fence, list nesting, table, quote,
  heading Setext, atau struktur lintas blok—gunakan full parse; jangan mencoba
  “menebak” rentang.
- Jangan mengubah seluruh operasi sekaligus. Mulai dengan **prototype/test-only**
  yang membandingkan kandidat incremental terhadap hasil full parse; baru ajukan
  implementasi produksi sebagai keputusan terpisah.
- Tahap implementasi harus menerima dirty range yang benar dari setiap jalur
  edit. Menemukan diff dengan menyalin dan membandingkan seluruh Source dapat
  menghapus manfaat yang hendak diukur dan bukan pengganti provenance edit.
- Tidak ada klaim bahwa incremental lebih cepat sampai metrik pasca-edit yang
  setara diukur pada harness 100 KB/1 MB/5 MB dan dicatat sebagai observasi
  lingkungan. Baseline Stage 10b saat ini hanya mengukur buka, satu ketikan,
  parse dingin, dan konstruksi pohon Preview.

## 3. Oracle dan gate sebelum integrasi

1. Untuk setiap kandidat hasil, `incremental(source, edit)` harus sama persis
   dengan `preview_model::parse(source_after_edit)`—termasuk seluruh `Block`,
   nesting, run/style, marker, offset Source absolut, cell/task target, dan batas
   fallback. Full parse menjadi pembanding independen.
2. Tambahkan property/differential coverage terpisah untuk insert, delete,
   replace, Undo/Redo, edit di awal/tengah/akhir, newline/paragraph split-merge,
   perubahan delimiter Markdown, UTF-8, tabel, task, code fence, serta suffix yang
   rentangnya bergeser. Jangan mengurangi atau menulis ulang badan property test
   yang sudah ada.
3. Semua jalur mutasi harus mengirim dirty provenance yang terverifikasi atau
   memilih full-parse fallback. Cache entries tidak boleh bertahan melewati
   `document_id`/revision yang salah; aksi stale tetap ditolak.
4. Lakukan benchmark sebelum/sesudah untuk edit biasa dan edit yang memaksa
   fallback, tanpa membandingkan lintas mesin dan tanpa mengubah batas byte,
   coordinate, focus/refocus, atau deadline gate lama.
5. Jika prototype gagal pada satu kelas Markdown, hasilnya mempersempit scope
   atau memicu fallback; bukan alasan melemahkan equality oracle.

## 4. Keputusan dan langkah berikutnya

- **Kesimpulan teknis:** 10a dapat dipisahkan dari 9c/9d pada tingkat tujuan;
  masih ada risiko implementasi besar di invalidation, absolute ranges, dan
  banyak jalur mutasi.
- **Keputusan roadmap:** belum dicatat; proposal ini tidak menghapus atau
  mengubah dependency lama secara otomatis.
- **Izin implementasi:** belum diberikan. Prototype test-only dan terutama
  integrasi produksi memerlukan persetujuan eksplisit terpisah.
- **Stage 9c/9d:** ditunda sesuai keputusan scope terpisah. Tidak ada marker
  hiding, mapping per glyph, atau pensiun panel draft.
- **Estimasi:** tidak ada perubahan pada progress roadmap atau kesiapan rilis;
  keduanya tetap dilaporkan terpisah.
