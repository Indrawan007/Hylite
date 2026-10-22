# Stage 5d — navigasi provenance blok preview ke Source

Langkah pertama navigasi posisi dari preview native, **bukan WYSIWYG, glyph hit
testing, source↔selection mapping atau synchronized scrolling**. Source tetap
satu buffer otoritatif; tidak ada perubahan serializer/save/export/dependensi.

## Perilaku

- Leaf block menampilkan tombol eksplisit **Show block source**. Berlaku untuk
  heading, paragraph, code, rule, tabel sebagai satu blok, dan source fallback.
- Quote/list/item adalah container; tombol berada pada leaf anaknya. Sel tabel
  belum menjadi target individual. Empty document tidak mempunyai target.
- Klik dari Preview membuka Split; dari Split tetap di Split. Caret Source
  ditempatkan pada **awal rentang byte provenance blok**, bukan posisi huruf yang
  diklik. Rentang paragraph sintetis pada tight list bisa mulai pada isi run,
  bukan marker bullet/container/inline Markdown.
- Find/Go to Line ditutup setelah sukses, current Find match dibersihkan tetapi
  query dipertahankan; task scoped Source memulihkan fokus. Tidak ada edit teks,
  checkpoint/dirty/revisi/history baru. Navigasi menutup typing group dan
  mengkolapskan selection seperti Go to Line sebelumnya.
- Inline URL tetap inert, gambar tidak dibaca/dimuat, HTML/math tetap source
  literal. Tombol provenance bukan link browser atau edit langsung pada preview.

## Validasi dan race

`preview_navigation::Target` membawa full Snapshot (document ID, revision,
state ID) dan range UTF-8. Resolver core hanya memvalidasi snapshot, range
non-empty dan batas UTF-8; `Document::preview_source_offset` juga menuntut model
cache sudah ada dan range **identik dengan leaf block yang tersedia saat ini**.
Range valid tetapi sembarang di dalam blok, range container/sel, stale snapshot,
atau cache cold ditolak. Validasi tidak mem-parse model cold demi message.

Controller menolak aksi di mode Source, saat konfirmasi atau Open in-flight.
Save/export biasa tidak memblokir navigasi dan token operasinya tidak dikonsumsi.
Setelah edit/undo/redo, target lama stale walau bytes kembali identik. New/Open
mengganti document ID, jadi target dokumen sebelumnya juga stale. Caret/tema/
layout dan save acknowledgment tidak mengubah snapshot konten sehingga target
saat ini tetap valid. Tidak ada offset rebase otomatis atau heuristik glyph.

Caret direkonstruksi melalui `set_caret` yang memverifikasi byte source dan
posisi native sebelum swap. Mode/panel baru diubah setelah rekonstruksi sukses;
invalid target tidak mengganti caret/selection/mode/query. Render key Source
bisa berubah, sementara model preview tetap valid karena teks tidak berubah.

## Tes

- **8 tes core baru**: offset byte vs Unicode scalar, UTF-8 endpoints, empty/
  reversed/out-of-bounds ranges, EOF, full snapshot equality, stale revision
  walau source identik, dan no source/target mutation.
- **7 tes Document baru**: seluruh leaf termasuk nested quote/list/Unicode/code/
  tabel/fallback; cache cold, forged/container/cell ranges, edit+undo stale,
  replacement same text, caret/save metadata, dan whole-table limit fallback.
- **6 tes controller baru**: Preview→Split; Split/query/redo invariants;
  Source/prompt/Open guards; invalid message mempertahankan selection/query;
  navigasi ketika Save/Export aktif; stale message setelah edit/replace.
- **1 tes renderer baru** memastikan hanya leaf mengirim action dengan snapshot
  dan range yang tepat. Tes inert inline span lama tetap berlaku.
- GUI menambah **4 pemeriksaan clipboard**: Source fixture Unicode/trailing tab;
  klik tombol preview pertama lalu ketik `X` tanpa klik Source harus menghasilkan
  `X# café🙂...`; Undo sentinel mengembalikan source byte-exact; satu Undo
  berikutnya mengembalikan welcome clean (navigasi tidak menambah history).
  Klik tombol adalah interaksi yang diuji, **bukan** workaround fokus Source.
  Semua Copy baru memakai `refocus=False`. Screenshot `reveal-preview.png` dan
  `reveal-split.png` bukan pixel/glyph oracle.
- Satu helper Python baru menjaga fixture, urutan expected bytes dan hanya
  satu klik tombol preview, tanpa click/windowfocus tambahan ke Source.

## Status eksekusi

Lokal: **158 core** (150 baseline + 8 navigator) dan **18 helper Python** lulus;
format harness/aplikasi dan diff diperiksa. Gate Cargo/Iced/Clippy/build serta
klik GUI native baru masih menunggu CI; tidak diasumsikan lulus dari core saja.

## Batas dan checklist manual — belum dinyatakan lulus

- [ ] Periksa tombol/tooltip, warna, wrap dan kepadatan kontrol pada Light/Dark.
- [ ] Periksa Source viewport untuk blok jauh, nested container, resize/HiDPI;
  logical caret benar tidak membuktikan scroll/glyph terlihat tepat.
- [ ] Runtime Arch + Hyprland/Wayland, GPU, IME/RTL dan input desktop target.
- [ ] Desain mapping glyph/run/selection dua arah sebelum prototype WYSIWYG.

Setiap leaf menambah widget kontrol; belum virtualisasi. Lookup membership dan
rekonstruksi Source masih O(n), bukan operasi posisi konstanta atau benchmark
latensi. Model cache single-entry dan batas tabel/nesting sebelumnya tetap
berlaku. Tidak membuka URL, portal, file atau worker baru; tidak ada app test hook,
pixel assertions, pelonggaran byte-exact checks, timeout baru atau merge `main`.
