# Stage 6d — level heading Setext root tanpa konversi ATX

Successor [heading ATX 6c](testing-stage6c.md). Bukan WYSIWYG umum atau editing
teks bebas. Kontrol H−/H+ yang sama kini juga mendukung heading Setext root.

## Kontrak

- Setext H1 (`===`) dapat H+ menjadi H2 (`---`); Setext H2 dapat H− menjadi H1.
  H− pada H1 dan H+ pada H2 disabled. Tidak konversi ke ATX/H3/paragraph.
- Hanya run underline ASCII kanonis yang diganti seluruhnya, satu transaksi
  Source Atomic. Panjang run, indent 0–3 spasi, whitespace suffix, body raw
  (termasuk beberapa baris, markup, entities, Unicode), dan byte lain tetap.
  Panjang run dapat satu atau lebih karakter; tidak dinormalisasi menjadi tiga.
- Target full Snapshot + exact block/underline range + expected level. Metadata
  `Block::setext_marker` hanya ada pada root Heading yang tervalidasi; ATX marker
  tetap field terpisah. Cold/stale/forged/list/quote/code/HTML/rule ditolak oleh
  exact root membership di Document sebelum planner memeriksa source bytes.
- Planner hanya lexical: dapat mengenali underline di dalam source code/list.
  Itu **bukan** izin edit tanpa model membership. Tidak mencoba menebak konteks
  container dari body string. Parser grammar/options/dependensi tidak berubah.
- ATX adapter/planner tetap punya kontrak lama dan tidak menerima Setext.
  Renderer memakai action Setext tersendiri sebagai fallback untuk kontrol yang
  sama; test penolakan ATX→Setext lama tetap utuh, bukan dihapus/dilonggarkan.
- Guard Source/prompt/Open tetap menolak; Save/Export biasa mengizinkan dan tidak
  mengonsumsi token. Mode tetap; caret ke akhir run underline, selection collapse,
  cache invalidated/revision naik/search refresh. Split memulihkan logical
  Source/query focus; Preview tidak memfokuskan Source.
- Menggunakan `EditKind::PreviewHeading` Atomic/non-coalescing yang sudah ada.
  Preview Undo/Redo hanya dapat menyeberangi checkbox/heading edge tervalidasi
  di puncak stack, tidak mencari melewati Source edit (termasuk underline edit
  identik). Source/Split tetap history umum. Batas 200 entries/8 MiB teks,
  branch/trim/reset, Ignored-only shortcut dan saved-checkpoint semantics tetap.

## Tes

24 Rust baru, total Cargo 510 (lulus pada CI di bawah):

- **7 core planner**: multiline raw bytes/round-trip, panjang run/indent/EOF,
  arah H1/H2, unsupported/malformed, bounds UTF-8/range, full snapshot/level,
  dan lexical-only counterexample untuk code/list.
- **2 model**: metadata exact root untuk kedua level/indent/multiline;
  ATX/nested/code/HTML/rule/malformed tidak mendapat underline target.
- **8 Document**: exact Atomic/cache/caret/dirty/EOF, cold/forged/stale, batas
  level tidak merusak redo, Source barrier, stale meski bytes sama, saved ack/
  branch, serta tiga transaksi Setext/ATX/checkbox yang tetap terpisah.
- **2 adapter**: snapshot/marker/block/level dan endpoint, penolakan non-Setext.
- **5 controller**: mode/Undo/Redo, Source/prompt/Open/Save/Export guards, Split
  query/search, stale/general typing refusal, dan save ack saat Preview Undo.

Tes lama tidak dihapus atau expected bytes/budget/oracle dilonggarkan. Fixture
negatif planner baru yang menganggap `- title` otomatis ditolak scanner underline
sempat gagal lokal; dipindah menjadi counterexample lexical-only yang eksplisit.
Penolakan container tetap dibuktikan di model dan Document, bukan diklaim dari
scanner underline saja. Ini koreksi fixture baru, bukan pelemahan auth produksi.

GUI menambah **7 clipboard checks** tanpa refocus: fixture multiline Unicode/
entity/raw markup/underline indent/suffix/kapital X lain/code literal/terminal
tab; Preview H+; Undo; Redo; Split H− + sentinel X tepat di akhir underline;
dua Undo memisahkan native typing dan Setext; terakhir Undo ke welcome clean.
Hanya dua klik kontrol produk, tidak klik Source. Screenshot `setext-preview.png`
dan `setext-split.png` bukan pixel oracle. Helper Python mengunci bytes/urutan/
shortcut/klik. Tidak mengubah timeout GUI 180 detik, native exit/lifecycle gates,
runtime hook, retry startup atau refocus workaround.

## Status eksekusi

[CI 39765116882](https://github.com/Indrawan007/Hylite/actions/runs/39765116882)
**success** pada `cac30e8797712746c3ca82d60c7fe6d0a56f3445`. Job
`119778526781`, 2026-10-22 10:42:33–10:46:34 UTC: **510 Cargo**, **194 strict core**
(subset Cargo), **24 Python**, **116 GUI** (102 clipboard, 12 title, 2 exit native
normal), format/build debug/Clippy `-D warnings` lulus. Step GUI
10:43:46–10:46:30 UTC, dalam budget 180 detik tanpa relaksasi gate.
Artifact API `12419393417`, `gui-x11-smoke`, 1.367.061 bytes, belum expired saat
diperiksa; screenshot belum direview visual. Ini bukan pixel/layout oracle
atau verifikasi Wayland/Hyprland. [Detail GUI](testing-gui-x11.md).

Repo sempat kembali menunjuk base `70d2a49` sementara seluruh worktree persis
sama dengan remote final 6c `877d9d5` (semua tracked blob dibandingkan). Ref dan
index dipulihkan ke commit remote yang sama tanpa mengubah/menghapus file atau
switch branch; status clean sebelum implementasi 6d.

Catatan lama startup kedua 6b exit 101 tetap belum terdiagnosis; tidak dianggap
diperbaiki oleh fitur ini. Run final 6b dan kedua run 6c lulus lengkap.

## Batas/manual — belum dinyatakan lulus

- [ ] Disabled state, Light/Dark/HiDPI, keyboard accessibility.
- [ ] Query-focus Setext runtime, viewport/caret visual dan scroll retention.
- [ ] Arch/Hyprland/Wayland, GPU/portal, IME/RTL dan compositor shortcuts.

Tidak ada nested heading edit, Setext↔ATX conversion, body editing, editable AST,
glyph/caret reverse map, selection/scroll sync atau benchmark speed/RAM. Konvensi
LF buffer dan rekonstruksi/full parse O(n) tetap; tidak ada klaim round-trip CRLF.
