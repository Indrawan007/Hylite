# Stage 6k — explicit atomic blank table columns

Successor [blank body-row insertion 6j](testing-stage6j.md). Menambah kolom kosong
pada tabel root kanonis rectangular, bukan AST serialization/spreadsheet/inline
WYSIWYG. Semua gate6j terdahulu tetap.

## Kontrak produk

- **Table actions… → Column N: Insert before / Insert after**. N mulai1, wajib
  existing real column. Opsi baru menjadi tail **sesudah seluruh alignment,
  duplicate/delete row, cell edit dan insert row6j**; indeks/label/picker width/
  top bar lama tidak berubah. Tidak ada shortcut/implicit row atau column creation.
- Satu aksi menyisipkan **dua spasi + pipe** pada header dan setiap body line,
  serta **spasi + tiga dash + spasi + pipe** pada delimiter. New alignment Default.
  Anchor separator sebelum/sesudah kolom dipindai dengan backslash parity;
  escaped pipe tidak dianggap batas kolom. Raw code/markup/entity tidak diubah.
- Contoh insert sebelum kolom1: `| a | b |` → `|  | a | b |`;
  delimiter `| :-- | --: |` → `| --- | :-- | --: |`. Semua byte raw segment,
  indent, outer/trailing whitespace/tab, dash count/alignment lama, prefix/suffix
  tetap, hanya insertion pada separator per line. Tidak menyalin/normalize sel lama.
- Seluruh tabel menjadi **satu TextChange dan satu Atomic PreviewTable edge**,
  bukan satu edit per line atau per sel. Source tetap dokumen tunggal. Undo/Redo
  memulihkan seluruh kolom/source/caret/checkpoint, bukan header saja.
- Source caret sesudah insert berada **di antara dua spasi sel header baru**,
  bukan pada body atau ujung tabel. Split mengembalikan Source focus bila Find/
  Go-to tidak aktif. Native typing berikutnya menjadi transaksi terpisah.
- Tabel wajib root, pipe-leading/pipe-trailing **di seluruh physical lines termasuk
  delimiter**, LF-terminated, rectangular dan selnya lolos validasi raw6h (termasuk
  payload16KiB). Header-only didukung jika canonical LF; ragged/padded/extra sel,
  missing outer pipes, EOF tanpa LF, nested/fence/HTML/fallback ditolak.
- Growth harus <=12kolom, <=128row termasuk header, <=512sel sesudah insert.
  Tidak ada prefix parsial pada refusal. Empty header/body cells boleh; full
  source parser projection harus mengonfirmasi root lengkap setelah preflight.
- Target membawa full document/revision/state snapshot, table range, row/column
  count, column index dan exact header payload + delimiter marker ranges.
  Document memerlukan warm current root membership dan metadata eligible baru;
  lexical planner sendiri **tidak** mengautentikasi provenance.
- Sebelum commit, candidate Source diparse penuh, root range/dimensions dan
  canonical editable cells/delimiters hasil dicek. Jika preflight gagal, Source/
  revision/history/caret/redo tidak berubah. Old projection diinvalidasi setelah
  commit; target baru harus berasal dari fresh parse. Ini O(n), bukan speedup.
- Source mode, confirmation, pending Open dan active cell draft menolak command.
  Save/Export tetap memakai snapshot awalnya; delayed save ack mengikuti identity
  checkpoint. Search direfresh dan logical Find/Go-to context tidak ditutup;
  physical query/Go-to focus sesudah insert masih manual.

## Implementasi dan tes

`preview_table_column::{Target,Action,can_insert,full_lines,plan}` standalone;
scanner bounded pada column cap dan berhenti jika line count tidak cocok. Table
model memiliki `insertable_columns` derived eligibility, bukan authority buffer.
`Document::edit_preview_table_column` mengautentikasi root metadata, membangun
candidate dan preflight sebelum `commit_kind(PreviewTable)`. Controller reuse
navigation/file guard, edited/search dan input focus task lama. Tidak menambah
runtime test hook atau refocus workaround.

**23 Rust baru**, total Cargo **672** =649 lama +8 planner +7 Document
+3 picker/model +5 controller:

- Planner exact before/after/middle/Unicode/escaped pipes/raw markup/alignment/
  trailing tab/prefix/suffix/indent; empty/header-only cells dan header caret;
  forged snapshots/ranges/counts/overflow; growth cap/noncanonical/EOF; fenced
  lexical-positive membuktikan kenapa Document root authentication tetap wajib.
- Document exact atomic source/alignment/caret/history, cold/forged/stale/replace,
  ragged/nested/HTML/fence/EOF/fallback rejection, caps preserve redo, canonical
  header-only/indented preflight, fresh empty body-cell edit dan separate history,
  save ack/branch checkpoint/Source typing barrier.
- Picker seluruh14 label prefix6j dipastikan persis; new action/target snapshots/
  ranges/index; eligibility/caps/noncanonical/nested; header-only dan indent.
  Dua total-length assert fixture lama menjadi18 (14lama+4baru); prefix/old-cell/
  row assertions utuh. Ini bukan relaksasi oracle GUI.
- Controller Preview/Split one-step history/header caret, modal/Source/prompt/Open
  guards dan immutable Save/Export tokens; Find offset refresh dan logical query/
  Go-to contexts; stale redo/Source barrier dan delayed save ack.

**243 strict core** subset Cargo, bukan243 tes tambahan unik. Python **39**
menambah2 helper independent literal expected full Source/click/Undo/no-refocus/
exit dan exclusive suite dispatch. Private DISPLAY guard mencakup delapan suite
sebelum subprocess.

## Suite GUI privat kedelapan

`--suite table-columns` memiliki Xvfb/app/XDG/env/clipboard baru, alarm180s,
step5min sendiri; job45min tetap. Ketujuh suite lama **116+9+9+9+14+16+11** utuh.
Semua18 body exercise functions lama (12 regression phases +6 suite independen)
sama secara AST terhadap `ddd5afa`. Tidak mengubah oracle bytes/koordinat/refocus/
deadline lama, tanpa retry/runtime hook.

Aktual **11 checks** =10 exact Source Copy +1 natural clean client Close exit0:

1. Welcome dan fixture raw UTF-8/spacing/escaped pipe/entity/terminal tab.
2. Preview Insert before kolom1 menambah blank header/delimiter/body sekaligus.
3. Satu Preview Undo memulihkan seluruh fixture; Redo menjaga original alignment.
4. Undo sebelum Split mengembalikan original table/menu prefix.
5. Split Insert after kolom terakhir lalu native X tanpa Source refocus masuk
   ke **header cell baru**: `| Name | Count | X |`, bukan body/query.
6. Undo typing menyisakan complete new column, Undo insert kembali fixture,
   Undo paste kembali welcome clean tanpa extra steps; natural Close0.

Empat klik hanya picker/opsi produk. Evidence `table-columns/columns-preview.png`
/`columns-split.png` bukan pixel/glyph oracle. Total aktual **195 GUI** =174
clipboard (165 Source +9 draft),12 title,9 natural exits.

```sh
./scripts/test-gui-x11.sh --suite table-columns --artifacts "$(mktemp -d)"
# Tetap jalankan ketujuh suite lama; ini bukan substitusi.
```

## Status eksekusi

Lokal: Rust/harness format, **243 strict core**, **39 Python**, diff check lulus.
[CI 39867392194](https://github.com/Indrawan007/Hylite/actions/runs/39867392194)
pada `9b1e931a2293120bcc50b3655424e90c7f4cfe18` **success**, watch exit0,
job `120133186140`,22 Oktober2026 22:47:35–22:53:50 UTC /23 Oktober Makassar.
Format, strict Clippy, native debug build, **672 Cargo**, **243 strict core**
(subset), **39 Python**, dan **116+9+9+9+14+16+11+11 GUI** lulus. Annotation
memuat semua11 PASS new column dan totals kedelapan suite; oracle/deadline lama
utuh. Artifact `12464787718`, `gui-x11-smoke`,2.143.016 bytes, belum expired saat
diperiksa; screenshot belum direview visual. CI docs6j `39866562690` dibatalkan
ketika push6k supersede; bukti fitur6j `39865519334` tetap success dan gate6j
kembali lulus di run6k ini.

## Batas/manual

- [ ] Middle-column/header-only/1kolom/11→12kolom UI, indented headers/trailing tab
  visual, large menu scroll/keyboard traversal dan active Find/Go-to input focus.
- [ ] Multi-line glyph/caret/scroll/selection visuals, IME/RTL/accessibility/HiDPI,
  desktop Arch/Hyprland/Wayland/portal/GPU. X11 tiny-skia bukan bukti compositor.

Tidak ada column delete/reorder/resize, bulk columns, header-row insertion,
selection formatting, multi-cell selection/draft session Undo, free Preview typing,
glyph↔caret mapping, scroll sync, incremental parse/virtualisasi atau speed/RAM
benchmark. Reconstruction/history + preflight tetap O(n), Source canonical LF/
gutter yang ditunda dan startup6b exit101 lama yang belum terdiagnosis tetap.
Gate baru tidak diklaim memperbaiki startup atau membuktikan pixel layout.
