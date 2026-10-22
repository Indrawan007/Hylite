# Stage 6c — kontrol level heading ATX tervalidasi

Successor [checkbox 6a](testing-stage6a.md) / [history task 6b](testing-stage6b.md).
Ini **bukan WYSIWYG umum**: hanya root ATX heading yang mempunyai kontrol level.

## Kontrak produk

- Pada bar provenance heading Preview/Split, **H−** promote level (misalnya H3
  ke H2), **H+** demote (H2 ke H3). Tidak mengubah heading menjadi paragraph.
  H− di H1 dan H+ di H6 disabled; pemeriksaan yang sama ada di planner.
- Hanya ATX root dengan 0–3 spasi indent dan pembuka `#` kanonis. Setext,
  heading dalam quote/list, code/HTML dan blok lain tidak bisa diedit lewat
  kontrol ini. Heading unsupported tetap dirender; kontrol heading disabled.
- Target berisi full Snapshot, range blok, range marker, expected level. Model
  hanya memberi metadata editable untuk root heading. Document menuntut exact
  membership pada model warm saat ini, lalu planner memvalidasi source bytes.
  Cold/stale/forged/level/range mismatch ditolak tanpa edit atau rebase.
- Edit hanya menyisipkan/menghapus **satu byte `#` pembuka**. Body inline raw,
  entities/escapes, closing hashes, indent, separator dan whitespace lain tidak
  di-serialize/normalize. Konvensi buffer LF yang sudah ada tidak diubah.
- Commit memakai jalur Source Atomic: satu checkpoint/history, revision naik,
  cache invalidated, caret sesudah marker baru, selection collapse. Mode tetap;
  Split memulihkan logical Source/query focus, Preview tidak memfokuskan Source.
- Guard prompt/Open/Source menolak command; Save/Export biasa tetap diizinkan,
  token operasi dan saved-snapshot dirty semantics tetap utuh. Find di Split
  di-refresh tanpa menutup query.

## Kebijakan history successor

Stage 6b hanya mengizinkan checkbox. Stage 6c **secara eksplisit memperluas**
PreviewHistory ke checkbox **dan root ATX level edit tervalidasi**, bukan semua
edit Source. `EditKind::PreviewHeading` Atomic/non-coalescing; `PreviewTask`
mempertahankan kontraknya. Tag edge privat sekarang bernama `preview_edit`;
metadata itu ikut bergerak dengan record, bukan set/stack paralel.

Keyboard Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y dan toolbar hanya boleh melewati edge
eligible di puncak arah terkait; tidak mencari lebih dalam melewati Source edit,
termasuk jika Source edit menambah byte `#` yang sama. Source/Split tetap Undo/
Redo umum. Captured-event filter, prompt/Open/query history guards, budget
200 entries/8 MiB teks, trim/branch/reset, revision/cache dan saved checkpoint
menggunakan jalur 6b. Native typing/formatting umum Preview tetap ditolak.

## Tes

25 tes Rust baru (expected total Cargo 486, belum bukti sampai CI):

- 7 planner core: one-hash raw byte preservation, semua level/bounds/round-trip,
  empty/EOF/indent/tab, unsupported/container/Setext, malformed UTF-8/ranges,
  full snapshot/marker/level, dan batas lexical-only tanpa klaim membership.
- 1 core history: heading/task edge berurutan, transfer tag dan barrier Source.
- 2 model: metadata exact pada root ATX (indent 0–3 × level 1–6), metadata absen
  untuk Setext/nested/fence/HTML.
- 8 Document: Atomic/cache/caret/dirty/EOF, cold/stale/forged, batas level tidak
  menghapus redo, Source hash edit bukan preview edge, restore/replace stale,
  saved checkpoint/branch dan mixed checkbox/heading history.
- 2 adapter: stamp snapshot/blok/marker/level, endpoint disable, unsupported
  tidak menghasilkan message edit.
- 5 controller: mode/source/undo, prompt/Open/Save/Export guards, Split search
  context, stale dan ordinary typing rejection, save ack setelah Preview Undo.

Oracle lama tidak dilonggarkan. Pemanggilan history getter privat pada tes lama
hanya berganti nama dari `preview_task_available` ke `preview_edit_available`.

GUI menambah 8 Copy checks tanpa refocus: fixture UTF-8/raw inline/closing hashes/
terminal tab; Preview H+; Undo; Redo; Preview H−; Split H+ lalu sentinel `X` tepat
sesudah marker (bukti Source keyboard focus); dua Undo membedakan transaksi
heading dan native typing; terakhir Undo ke welcome clean tanpa history tambahan.
Tiga klik hanya kontrol produk (bukan Source). `heading-preview.png` dan
`heading-split.png` bukan pixel assertions. Mock helper mengunci bytes/urutan/
shortcut dan jumlah klik. Budget GUI tetap 180 detik; tidak ada runtime hook,
retry startup, refocus workaround, atau gate/expected bytes yang diperlonggar.

## Status eksekusi

Lokal: **187 strict core** dan **23 Python** lulus; format/diff diperiksa.
Native Cargo/Clippy/build dan GUI belum dinyatakan lulus sampai CI diamati.
Expected GUI total 109 (95 clipboard, 12 title, 2 exit native normal).

Catatan lama: percobaan awal 6b `39736591093` startup kedua exit 101, akar belum
diketahui. Dua run berikutnya (`39737235985`, `39737900733`) lulus lengkap. Tidak
ada diagnosis/perbaikan akar failure itu yang diklaim oleh Stage 6c.

## Belum diverifikasi / di luar cakupan

- [ ] Visual disabled/hint/light/dark/HiDPI dan keyboard accessibility.
- [ ] Query-focus runtime untuk heading, viewport/scroll/caret visual.
- [ ] Arch/Hyprland/Wayland, portal/GPU, IME/RTL dan compositor shortcuts.

Tidak ada Setext conversion, nested heading editing, direct text editing,
editable AST, glyph/caret reverse map atau synchronized scroll. Full parse dan
rekonstruksi buffer tetap O(n); tidak ada benchmark speed/RAM. Tidak mengubah
parser/dependensi/lockfile/file worker atau lifecycle assertions.
