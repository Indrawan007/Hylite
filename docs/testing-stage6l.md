# Stage 6l — explicit atomic column deletion

Successor [blank columns 6k](testing-stage6k.md). CI dokumentasi6k
[39868311414](https://github.com/Indrawan007/Hylite/actions/runs/39868311414)
pada `36c137c` kini success. Stage ini menambah Delete eksplisit, bukan implicit
mutation dari navigasi/draft atau general inline WYSIWYG.

## Kontrak produk

- **Table actions… → Column N: Delete**, N mulai1. Menghapus seluruh raw header
  segment, delimiter segment dan body segments kolom yang dipilih. Ini aksi
  destruktif **eksplisit**, seperti Row Delete6g, tanpa prompt tambahan; gunakan
  Undo untuk memulihkan. Tidak ada shortcut atau auto-delete saat mengosongkan sel.
- Menu Delete menjadi tail **sesudah semua opsi6k**, termasuk seluruh Insert
  before/after columns. Semua label/index alignment/row/cell/insert lama utuh,
  width168/text11 dan action-bar height tetap.
- Hanya canonical rectangular root tables, semua physical line (termasuk
  delimiter) pipe-leading/pipe-trailing LF, raw cells valid6h. Ragged/padding/
  extra cells/noncanonical/missing outer pipe/EOF tanpa LF/nested/fence/HTML/
  fallback ditolak. Payload16KiB/canonical scanner6h tetap, bukan general tables.
- **Kolom terakhir tidak boleh dihapus**, termasuk empty/header-only table.
  Tabel header-only dengan >=2kolom boleh. Deletion boleh pada growth cap12kolom
  atau512sel karena jumlah sel berkurang; renderer128row/12kolom/512sel tetap.
- Scanner memakai backslash parity, tidak menganggap escaped pipe sebagai batas.
  Pada setiap line, raw segment setelah left separator sampai **termasuk right
  separator** dipotong. Left pipe dipertahankan; semua byte retained neighbors,
  old alignment/dash counts, indent/trailing tab/whitespace dan prefix/suffix utuh.
  Tidak ada AST serialization atau normalisasi sel yang tidak dihapus.
- Source contoh `| a | b | c |` delete kolom2 → `| a | c |`; delimiter dan body
  dipotong serentak. **Satu whole-table TextChange / Atomic PreviewTable edge**,
  bukan satu transaksi per line. Undo/Redo memulihkan exact bytes/caret/checkpoint.
- Caret Source di **awal raw payload header neighbor yang bertahan**: kolom kanan
  pada index lama yang sama jika ada, atau kolom kiri jika yang terakhir dihapus.
  Empty payload memakai canonical empty range. Split mengembalikan Source focus
  bila query/Go-to tidak aktif. Native typing berikutnya transaksi/barrier sendiri.
- Target full document/revision/state snapshot, root range/dimensions/column dan
  header payload/marker ranges wajib cocok warm current model. `deletable_columns`
  derived metadata terpisah dari growth eligibility, bukan authoritative buffer.
- Candidate full Source diparse **sebelum commit**: full root range, same row count,
  one-fewer columns dan canonical real cells/delimiters harus tetap cocok. Refusal
  Document tidak mengubah Source/revision/history/caret/redo; tidak ada prefix
  deletion parsial. Commit menginvalidasi cache lama; target berikut harus fresh.
- Mode Source, confirmation, pending Open dan active cell draft menolak command
  lama/barunya, tidak membuang draft. Save/Export snapshot awal tetap; delayed save
  ack mengikuti checkpoint identity. Search direfresh, logical Find/Go-to context
  tetap. Physical query/Go-to focus sesudah Delete masih manual.

## Implementasi dan tes

`preview_table_column::Action::Delete` dan `can_delete` menggunakan bounded
canonical scanner6k. Planner hanya menghapus selected segments, lalu resolve
neighbor caret dari canonical replacement cells. Table projection mendapat
`deletable_columns`; Document memilih eligibility sesuai action dan candidate
preflight columns count. Existing controller/message/modal/file/search/focus
jalur sama; tidak menambah runtime hook atau fallback refocus.

**22 Rust baru**, total Cargo **694** =672 lama +7 planner +7 Document
+3 picker/model +5 controller:

- Planner exact first/middle/last segments, retained Unicode/markup/escaped pipes/
  alignment/trailing tabs/prefix/suffix/indent, empty neighbor/header caret;
  last-column/invalid counts/overflow; deletion di growth caps; forged snapshot/
  dimensions/ranges; malformed lines dan fenced lexical-positive bukan auth.
- Document complete atomic raw source/alignment/caret/cache/history, cold/forged/
  stale/replace; last-column refusal preserves redo/checkpoint; growth caps/branch,
  nested/literal/noncanonical/EOF/fallback rejection; header-only/indent/middle dan
  fresh retained body-cell edit; save ack/Source typing barrier.
- Picker semua18 labels prefix6k persis, delete target snapshots/ranges/action;
  last-column/unsupported suppression dan deletion independent of growth caps.
  Total-length fixture berubah18→20, insertion-only helper tetap menegaskan
  jumlah insertions saja. Old prefix/cell/row/insert target asserts tetap utuh;
  bukan relaksasi oracle GUI lama.
- Controller Preview/Split one-step history/retained header caret; Source/prompt/
  Open/modal guards, immutable Save/Export tokens; Find offset refresh dan logical
  query/Go-to contexts, stale redo/Source barrier; save ack dan last-column refusal.

**250 strict core** subset Cargo, bukan250 tes tambahan unik. Python **41**
menambah2 helper independent full Source literals/click/Undo/no-refocus/exit dan
exclusive dispatch. Private DISPLAY guard mencakup sembilan suite sebelum subprocess.

## Suite GUI privat kesembilan

`--suite table-column-delete` memiliki private Xvfb/app/XDG/env/clipboard baru,
alarm180s/step5min sendiri, job45min tetap. Delapan suite lama
**116+9+9+9+14+16+11+11** utuh. Semua19 body exercise functions lama (12 regression
phases +7 independent suites) AST-identical terhadap `36c137c`. Tidak mengubah
oracle bytes/koordinat/refocus/deadline lama atau menambah retry/runtime hooks.

Aktual **11 checks** =10 exact Source Copy +1 natural clean client Close exit0:

1. Welcome/fixture raw UTF-8/markup/escaped pipe/entity/terminal tab.
2. Preview Delete first column memotong header/delimiter/body serentak, retained
   Count/right-alignment/entity/tab tepat. Satu Undo mengembalikan seluruh fixture.
3. Redo kembali exact one-column Source, Undo sebelum Split kembali full fixture.
4. Split Delete last column lalu native X tanpa Source refocus masuk ke retained
   header payload: `| XName |`, bukan body/query atau kolom yang sudah dihapus.
5. Undo typing menyisakan complete deletion, Undo deletion mengembalikan fixture,
   Undo paste kembali welcome clean tanpa extra steps; natural client Close0.

Empat klik hanya picker/opsi produk. Last-column protection diuji model/Document/
controller, bukan klaim menu absence pixel/runtime. Evidence
`table-column-delete/column-delete-preview.png` / `column-delete-split.png` bukan
pixel/glyph oracle. Total aktual **206 GUI** =184 clipboard (175 Source +9 draft),
12 title,10 natural exits.

```sh
./scripts/test-gui-x11.sh --suite table-column-delete --artifacts "$(mktemp -d)"
# Tetap jalankan kedelapan suite lama; ini bukan substitusi.
```

## Status eksekusi

Lokal: Rust/harness format, **250 strict core**, **41 Python**, diff check lulus.
[CI 39871210748](https://github.com/Indrawan007/Hylite/actions/runs/39871210748)
pada `492669883a071d59258c94e8f329c3f273148eea` **success**, watch exit0,
job `120145818542`,22 Oktober2026 23:21:18–23:28:51 UTC /23 Oktober Makassar.
Format, strict Clippy, native debug build, **694 Cargo**, **250 strict core**
(subset), **41 Python**, dan **116+9+9+9+14+16+11+11+11 GUI** lulus. Annotation
memuat semua11 PASS Delete dan totals sembilan suite; oracle/deadline lama utuh.
Artifact `12466407895`, `gui-x11-smoke`,2.237.302 bytes, belum expired saat
diperiksa; screenshot belum direview visual.

## Batas/manual

- [ ] Middle/header-only/empty/12kolom deletion UI, trailing tab/indented header
  visual, large menu keyboard/scroll, active Find/Go-to physical focus, native
  last-column menu absence/accessibility (guard diuji unit, bukan pixel oracle).
- [ ] Glyph/caret/selection/scroll visuals, IME/RTL/HiDPI, desktop Arch/Hyprland/
  Wayland/portal/GPU. X11 tiny-skia bukan bukti target compositor.

Tidak ada delete whole table/last column, column reorder/resize, bulk deletion,
header-row insert, multi-cell selection/draft session Undo, free Preview typing,
rich selection formatting, glyph↔caret mapping, scroll sync atau incremental
parse/virtualisasi/speed-RAM benchmark. Reconstruction/history +preflight tetap
O(n). Source canonical LF/gutter yang ditunda serta startup6b exit101 lama yang
belum terdiagnosis tetap; fitur ini bukan klaim memperbaiki startup/pixel layout.
