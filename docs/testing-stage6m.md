# Stage 6m — bounded raw body-row moves

Successor [atomic column deletion6l](testing-stage6l.md). CI dokumentasi6l
[39872209785](https://github.com/Indrawan007/Hylite/actions/runs/39872209785)
pada `69d0ede` kini success. Stage ini menambah row reorder bertetangga eksplisit,
bukan drag-and-drop, implicit draft commit atau general inline WYSIWYG.

## Kontrak produk

- **Table actions… → Row N: Move up / Move down**, body N mulai1. Move up hanya
  jika N>1, Move down hanya jika ada real body row berikutnya. Header/delimiter
  tidak pernah menjadi neighbor; tidak wrap atau berpindah tabel.
- Opsi move di tail **sesudah seluruh alignment/row/cell/insert/column Delete6l**.
  Prefix lama, width168/text11/action-bar height tidak berubah. Boundary actions
  tidak ditawarkan; tidak ada shortcut baru.
- Dua **whole raw LF physical lines** ditukar: selected row + previous/next row.
  Isi/indent/spacing/tab/Unicode/markup/escaped pipe/entity/ragged width ikut baris
  aslinya. Header/delimiter/all alignment, baris lain serta prefix/suffix tetap.
  Tidak membuat ulang sel, tidak mengubah kolom, bukan AST serialization.
- Supported shape mengikuti body-row6g: root canonical pipe-leading LF table,
  indent0–3 spasi, batas128row termasuk header/12kolom/512sel. Ragged body boleh;
  nested/list/quote/fence/HTML/fallback/noncanonical/EOF tanpa LF ditolak. Header-
  only/single-body tidak punya move. Growth cap tidak menghalangi swap.
- Planner memerlukan exact full document/revision/state snapshot, root range,
  row/column dimensions, row index dan physical line target. `neighbor` hanya
  perhitungan index bounded, **bukan** authentication. Document menuntut warm
  current root membership sebelum planning atau preflight.
- Candidate Source diparse penuh **sebelum commit**: root range/dimensions/body
  lines harus tetap cocok. Refusal Document tidak mengubah Source/history/revision/
  caret/redo; tidak ada partial reorder. Satu efektif move = **satu Atomic
  PreviewTable edge**, Undo/Redo memulihkan exact bytes/caret/checkpoint.
- Selected row yang dipindahkan menentukan caret Source pada **awal physical line
  di posisi barunya**. Move down caret relatif = panjang raw next line (UTF-8
  bytes); Move up caret relatif0. Ini bukan mapping glyph/sel/indent payload.
- Adjacent **identical raw lines** no-op, tanpa revision/history/cache/caret/redo
  mutation. Model menu hanya tahu bounded neighbors, jadi pilihan no-op masih
  bisa muncul; controller melaporkan unchanged. Render-equivalent tetapi berbeda
  raw bytes (misalnya indent) tetap effective move, tidak dinormalisasi.
- Source mode, confirmation/pending Open dan active cell draft menolak command;
  draft tidak dibuang. Running Save/Export tetap memakai snapshot awal; delayed
  save ack mengikuti checkpoint identity. Search direfresh dan logical Find/
  Go-to context tetap. Native typing setelah move menjadi edge/barrier terpisah.
- Cache model lama diinvalidasi saat commit; aksi/cell edit berikut memakai target
  fresh. Full parse preflight/reconstruction/history tetap O(n), bukan speedup.

## Implementasi dan tes

`preview_table_row::{Action::MoveUp,Action::MoveDown,neighbor}` memakai bounded
body index dan exact physical lines6g. Planner menukar dua raw line tanpa menyentuh
header/delimiter; identical raw rows return None. Document reuse warm row
provenance, candidate parser preflight hanya untuk move sebelum Atomic
PreviewTable. Jalur controller/file/modal/search/focus6g tetap, status refusal
menyebut unchanged. Tidak ada runtime hooks atau Source refocus workaround.

**22 Rust baru**, total Cargo **716** =694 lama +7 planner +7 Document
+3 picker/model +5 controller:

- Planner exact up/down/UTF-8 byte caret/prefix/suffix/indent/tab/ragged/markup/
  escaped pipes, protected header/bounds/counts/overflow/non-move actions,
  forged snapshot/range/index/dimensions; identical no-op vs raw-different rows;
  growth caps; EOF refusal dan fenced lexical-positive bukan authentication.
- Document atomic bytes/caret/cache/dimensions/history, cold/forged/stale/replace,
  boundary/identical no-op preserving redo/checkpoint, growth caps/branch,
  nested/literal/noncanonical/EOF/fallback rejection termasuk exact inner fenced
  target, fresh moved-cell raw provenance dan explicit Apply history; delayed
  save ack/Source typing barrier.
- Picker full old14-label prefix pada fixture ragged dipastikan persis; new targets
  full snapshot/line/range/action; header/single-body/bounds/unsupported suppression;
  neighbors tetap ditawarkan pada growth caps. Fixture canonical single-body6l
  masih20 pilihan, tidak mengubah assert native picker lama.
- Controller Preview/Split exact one-step history/selected-row caret; Source/
  prompt/Open/modal/Save/Export guards; search offset refresh dan logical Find/
  Go-to context; stale redo/Source barrier; delayed save ack/boundary refusal.

**257 strict core** subset Cargo, bukan257 tes tambahan unik. Python **43**
menambah2 helper independent full Source/line/click/Undo/no-refocus/exit literals
serta exclusive dispatch. Private DISPLAY guard mencakup sepuluh suite sebelum
subprocess. Initial planner test expected body offset21 salah satu byte; literal
header length22 dikoreksi sebelum commit, bukan mengubah production/oracle lama.

## Suite GUI privat kesepuluh

`--suite table-row-move` memiliki private Xvfb/app/XDG/env/clipboard baru,
alarm180s/step5min sendiri, job45min tetap. Sembilan suite lama
**116+9+9+9+14+16+11+11+11** utuh. Semua20 body exercise functions lama (12
regression phases +8 independent suites) AST-identical terhadap `69d0ede`.
Tidak melonggarkan bytes/koordinat/refocus/deadline atau menambah retry/hooks.

Aktual **11 checks** =10 exact Source Copy +1 natural clean client Close exit0:

1. Welcome/fixture raw Unicode/markup/escaped pipe/entity/ragged row/terminal tab.
2. Preview Row1 Move down hanya menukar dua whole raw body lines, header/suffix utuh.
3. Satu Preview Undo memulihkan original ordering, Redo kembali exact moved lines.
4. Undo sebelum Split kembali original table/menu prefix.
5. Split Row2 Move up lalu native X tanpa refocus masuk ke moved row physical
   start: `X| short |`, bukan header/body neighbor atau query.
6. Undo typing menyisakan complete row move, Undo move kembali original fixture,
   Undo paste kembali welcome clean tanpa extra history, native clean Close0.

Empat klik hanya picker/opsi produk. Header/boundary/identical no-op guards diuji
planner/model/Document/controller, bukan runtime pixel/menu-absence assertions.
Evidence `table-row-move/row-move-preview.png` / `row-move-split.png` bukan pixel/
glyph oracle. Total aktual **217 GUI** =194 clipboard (185 Source +9 draft),
12 title,11 natural exits.

```sh
./scripts/test-gui-x11.sh --suite table-row-move --artifacts "$(mktemp -d)"
# Tetap jalankan kesembilan suite lama, bukan substitusinya.
```

## Status eksekusi

Lokal: Rust/harness format, **257 strict core**, **43 Python**, diff check lulus.
[CI 39872957659](https://github.com/Indrawan007/Hylite/actions/runs/39872957659)
pada `27da9ae2fa9238324a65abae930bed1fd819b759` **success**, watch exit0,
job `120151569350`,22 Oktober2026 23:37:02–23:43:37 UTC /23 Oktober Makassar.
Format, strict Clippy, native debug build, **716 Cargo**, **257 strict core**
(subset), **43 Python**, dan **116+9+9+9+14+16+11+11+11+11 GUI** lulus.
Annotation memuat semua11 PASS row move dan totals sepuluh suite; oracle/deadline
lama utuh. Artifact `12467486534`, `gui-x11-smoke`,2.335.915 bytes, belum expired
saat diperiksa; screenshot belum direview visual.

## Batas/manual

- [ ] Middle/3+body/1kolom/12kolom/3spasi/trailing-tab row move UI, no-op physical
  Source focus, active Find/Go-to physical focus, large menu keyboard/scroll,
  header/boundary menu absence/accessibility. Guard unit bukan pixel oracle.
- [ ] Glyph/caret/selection/scroll visuals, IME/RTL/HiDPI, desktop Arch/Hyprland/
  Wayland/portal/GPU. X11 tiny-skia tidak menggantikan verifikasi compositor.

Tidak ada header movement, drag/bulk/multi-row selection/reorder, column move/
resize, last-column/whole-table delete, implicit Apply, draft session Undo,
free Preview typing/rich selection formatting, glyph↔caret mapping, scroll sync
atau incremental parse/virtualisasi/speed-RAM benchmark. Source canonical LF/
gutter yang ditunda dan startup6b exit101 lama yang belum terdiagnosis tetap;
fitur ini bukan klaim memperbaiki startup atau pixel layout.
