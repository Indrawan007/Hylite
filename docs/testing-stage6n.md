# Stage 6n — bounded raw column moves with alignment

Successor [raw body-row movement6m](testing-stage6m.md). Menambah adjacent column
reorder eksplisit, bukan drag-and-drop/resize/implicit draft commit atau general
inline WYSIWYG. CI docs6m39873812558 cancelled saat push6n supersede; bukti fitur6m
39872957659 tetap success dan seluruh gate6m dijalankan lagi pada6n.

## Kontrak produk

- **Table actions… → Column N: Move left / Move right**, N mulai1. Hanya neighbor
  kolom real pada tabel yang sama; tidak wrap. Single-column tidak punya move,
  first-column tidak bisa ke kiri, last-column tidak bisa ke kanan.
- Opsi move kolom di tail **sesudah seluruh menu6m**, termasuk semua row moves.
  Prefix/index/label/width168/text11/action-bar height lama tetap; tanpa shortcut.
- Menukar **dua raw cell segments beserta right separator** pada setiap physical
  line: header, delimiter dan seluruh body. Delimiter alignment/dash counts ikut
  kolom asal, bukan menempel di slot lama. Raw spaces/tab/markup/Unicode/escaped
  pipe/entity ikut cell segment; row indent dan trailing bytes sesudah outer pipe
  tetap pada physical row. Prefix/suffix/kolom lain tidak diubah.
- Contoh `| Name | Count |` Move right kolom1 → `| Count | Name |`;
  `| --- | --: |` → `| --: | --- |`. Tidak membangun ulang AST/normalize sel lama.
- Satu whole-table TextChange = **satu Atomic PreviewTable edge**, bukan edit per
  row/cell. Candidate full Source parser preflight dilakukan sebelum commit:
  complete root range, same row/column dimensions dan canonical real cells/
  delimiters harus tetap cocok. Refusal Document tidak mengubah Source/revision/
  history/caret/redo; tidak ada partial column movement.
- Caret Source di **awal payload header selected column pada posisi barunya**,
  bukan neighbor yang ditukar atau sel body. Canonical empty range berlaku untuk
  empty header. Split Source focus kembali bila Find/Go-to tidak aktif; native
  typing berikutnya transaksi/barrier sendiri.
- Hanya canonical rectangular root table6k: seluruh physical lines termasuk
  delimiter outer-piped LF, raw cells valid6h/16KiB. Ragged/padding/extra cells,
  missing outer pipe/EOF tanpa LF/nested/fence/HTML/fallback ditolak. Header-only
  boleh jika >=2kolom. Renderer128row/12kolom/512sel tetap; growth cap tidak
  menghalangi swap karena dimensions tidak bertambah.
- Target membawa full document/revision/state snapshot, table range/dimensions,
  selected column dan exact header payload/marker ranges. `neighbor` hanya index
  bounded, bukan authentication. Document memerlukan warm current root membership
  dan canonical metadata (same eligibility as Delete, bukan growth flag).
- **Entire raw columns identik** (header, delimiter/alignment dan semua body) no-op:
  tidak menambah revision/history/cache/caret mutation atau membuang redo. Menu
  bounded neighbor masih bisa muncul; controller melaporkan unchanged. Sama secara
  render tetapi beda spacing/body/alignment tetap effective move.
- Source mode, confirmation/pending Open dan active cell draft menolak command;
  draft tidak dibuang/di-Apply implicit. Running Save/Export memakai snapshot awal;
  save ack tetap checkpoint identity. Search direfresh dan logical Find/Go-to
  context tetap; physical query/Go-to/no-op focus masih manual.
- Commit menginvalidasi projection lama, target berikut harus fresh. Full parse
  preflight/reconstruction/history tetap O(n), bukan speed/RAM improvement.

## Implementasi dan tes

`preview_table_column::{Action::MoveLeft,Action::MoveRight,neighbor}` reuse bounded
backslash-parity scanner6k. Swap two raw adjacent segments per line; resolve caret
from canonical replacement header cells. Identical whole-table replacement return
None before mutation. Document memakai deletable eligibility untuk moves dan
preflight same dimensions; no new authority buffer/AST serialization/runtime hook.
Existing controller/modal/file/search/focus jalur sama, refusal mencakup unchanged.

**22 Rust baru**, total Cargo **738** =716 lama +7 planner +7 Document
+3 picker/model +5 controller:

- Planner exact left/right/middle/raw alignment/Unicode/escaped pipes/indent/tab/
  prefix/suffix/caret, no-wrap/bounds/counts/overflow/non-move actions; forged
  snapshot/ranges/header/marker; identical raw no-op vs body/alignment/spacing
  differences; header-only/empty/growth caps; malformed/EOF/fenced lexical-positive.
- Document atomic bytes/alignment/dimensions/caret/cache/history, cold/forged/stale/
  replace, no-op/boundary preserving redo/checkpoint/cache, single-column refusal,
  header-only/empty/indent/growth caps, ragged/nested/literal/noncanonical/EOF/
  fallback rejection termasuk exact inner fenced target, fresh moved body-cell
  provenance/Apply history, delayed save ack/branch/Source barrier.
- Picker seluruh20 labels prefix6m pada single-body fixture persis, new targets
  full snapshot/ranges/action, unsupported/single-column/no-wrap suppression,
  header-only/growth caps. Total-length fixture20→22, existing prefix/cell/row/
  insert/Delete assertions tetap; bukan relaksasi oracle GUI.
- Controller Preview/Split exact one-step history/selected header caret; Source/
  prompt/Open/modal/immutable Save/Export guard; Find offset refresh dan logical
  Go-to context; stale redo/Source barrier; delayed save ack/boundary refusal.

**264 strict core** subset Cargo, bukan264 tes tambahan unik. Python **45**
menambah2 helper independent full Source/alignment/click/Undo/no-refocus/exit
literals serta exclusive dispatch. Private DISPLAY guard mencakup sebelas suite
sebelum subprocess.

## Suite GUI privat kesebelas

`--suite table-column-move` mempunyai private Xvfb/app/XDG/env/clipboard baru,
alarm180s/step5min sendiri, job45min tetap. Sepuluh suite lama
**116+9+9+9+14+16+11+11+11+11** utuh. Semua21 old exercise bodies (12 regression
phases +9 independent suites) AST-identical terhadap `8fbc789`. Tidak mengubah
bytes/koordinat/refocus/deadline lama, tanpa retry/runtime hooks.

Aktual **11 checks** =10 exact Source Copy +1 natural clean client Close exit0:

1. Welcome/fixture raw Unicode/markup/spacing/escaped pipe/entity/terminal tab.
2. Preview Column1 Move right menukar header/delimiter alignment/sel body serentak.
3. Satu Preview Undo memulihkan original order; Redo kembali exact moved columns.
4. Undo sebelum Split mengembalikan original Source dan prefix menu lama.
5. Split Column2 Move left lalu native X tanpa refocus masuk ke selected moved
   header payload: `| XCount | Name |`, bukan body atau header neighbor.
6. Undo typing menyisakan complete move, Undo move kembali original fixture,
   Undo paste kembali welcome clean tanpa extra history, natural client Close0.

Empat klik hanya picker/opsi produk. Boundary/single-column/identical guards diuji
planner/model/Document/controller, bukan runtime pixel/menu-absence assertions.
Evidence `table-column-move/column-move-preview.png` / `column-move-split.png`
bukan pixel/glyph oracle. Total aktual **228 GUI** =204 clipboard (195 Source
+9 draft),12 title,12 natural exits.

```sh
./scripts/test-gui-x11.sh --suite table-column-move --artifacts "$(mktemp -d)"
# Tetap jalankan kesepuluh suite lama; ini bukan substitusi.
```

## Status eksekusi

Lokal: Rust/harness format, **264 strict core**, **45 Python**, diff check lulus.
[CI 39874500418](https://github.com/Indrawan007/Hylite/actions/runs/39874500418)
pada `46c89c632bef97018ca8633a4877569fe1528408` **success**, watch exit0,
job `120156694687`,22 Oktober2026 23:51:38–23:59:38 UTC /23 Oktober Makassar.
Format, strict Clippy, native debug build, **738 Cargo**, **264 strict core**
(subset), **45 Python**, dan **116+9+9+9+14+16+11+11+11+11+11 GUI** lulus.
Annotation memuat semua11 PASS column move dan totals sebelas suite; oracle/
deadline lama utuh. Artifact `12468122247`, `gui-x11-smoke`,2.433.374 bytes,
belum expired saat diperiksa; screenshot belum direview visual.

## Batas/manual

- [ ] Middle/3+kolom/header-only/empty/12kolom/3spasi/trailing-tab column move UI,
  no-op/active Find/Go-to physical focus, large menu keyboard/scroll/accessibility,
  boundary/single-column menu absence. Guard unit bukan pixel proof.
- [ ] Glyph/caret/selection/scroll visuals, IME/RTL/HiDPI, desktop Arch/Hyprland/
  Wayland/portal/GPU. X11 tiny-skia tidak menggantikan verifikasi compositor.

Tidak ada drag/bulk reorder/multi-column selection, column resize, header-row
movement, last-column/whole-table delete, implicit Apply/draft session Undo,
free Preview typing/rich selection formatting, glyph↔caret mapping, scroll sync,
incremental parse/virtualisasi/speed-RAM benchmark. Source canonical LF/gutter
tertunda/startup6b exit101 lama yang belum terdiagnosis tetap; fitur ini bukan
klaim memperbaiki startup atau pixel layout.
