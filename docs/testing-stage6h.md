# Stage 6h — panel draft raw Markdown sel tabel

Successor [whole row 6g](testing-stage6g.md). Ini langkah awal editing teks sel
melalui panel eksplisit, **bukan inline/glyph-based WYSIWYG**.

## Kontrak produk

- `Table actions…` mempertahankan semua opsi alignment dan row actions di urutan
  lama, lalu menambahkan **Cell R:C: Edit**. Row **0 adalah header**, body dimulai
  1; kolom tampil mulai 1. Klik kiri sel tetap read-only dan klik kanan tetap
  navigasi Source, tidak diberi arti editing baru.
- Hanya **root projected table kanonis, rectangular, edge-pipe header/body**,
  indent 0–3 spasi, jumlah row/cell persis parser, dan delimiter 6f terautentikasi.
  Ragged/padded/extra cells, nested/quoted/literal/fallback, missing edge pipes
  tidak memperoleh cell edit. EOF tanpa LF boleh karena Apply tidak menambah LF.
- Sel genuine empty boleh diedit; padding virtual tidak pernah ditargetkan.
  Raw cell payload adalah rentang di antara pipes **tanpa outer spasi/tab**.
  Semua whitespace di luar payload dipertahankan verbatim, termasuk untuk cell
  kosong (insertion point setelah seluruh whitespace dalam segmen kosong).
- Draft adalah string UI terpisah, **bukan buffer dokumen kedua**. Mengetik,
  paste, Cancel/Escape tidak mengubah Source/checkpoint/revision/dirty atau
  membuat langkah history. Panel memuat raw Markdown, bukan display rich text.
- **Enter/Apply** mengganti hanya payload terpilih. Header/delimiter/pipes/outer
  spacing, raw sel lain, Unicode/entity/escaped pipe, suffix dan terminal tab utuh.
  Tidak ada serialisasi AST. Caret collapse tepat sesudah payload baru.
- **Single physical line, maksimal 16 KiB byte UTF-8**. Control chars ditolak
  kecuali tab. Literal pipe harus escaped `\|`; jumlah backslash ganjil/genap
  diperiksa. Odd trailing backslash pada payload setelah trim spasi/tab ditolak
  secara konservatif agar separator tidak ter-escape dan payload tetap dapat
  diproyeksikan lagi. Backtick tidak membuat unescaped pipe aman dalam GFM.
- Semua payload existing tabel harus memenuhi validator/cap agar tabel mendapat
  cell targets. Batas tabel lama **12 kolom / 128 row / 512 sel** tetap. Cap draft
  bukan batas ukuran file/RAM; alokasi clipboard/native input belum dibatasi oleh
  cap string state aplikasi. Input over-cap tidak disimpan/truncate lalu Apply
  diam-diam: value sebelumnya dipertahankan, error aktif, Apply ditolak sampai
  input valid lagi.
- Target membawa full **Snapshot + exact table range + row/column + total
  rows/columns + raw payload range**. Model memeriksa setiap payload ada dalam
  real parsed cell source range, dimensi persis, selain lexical canonical ranges.
  Document menuntut exact membership pada **cache warm root Table**, baik saat
  membuka maupun Apply. Lexical planner bukan bukti provenance.
- Cold/stale/forged/partial/index/dimension/marker ditolak tanpa rebase. Stale
  Apply mempertahankan draft dengan error, tidak memindahkannya ke teks baru.
  No-op Apply menutup panel tanpa normalisasi/source mutation/redo clearing.
- Satu Apply efektif adalah satu Atomic trusted **`PreviewTable`** edge yang
  sudah dipakai 6f/6g, tidak coalesce. Preview Undo/Redo top-only, Source barrier,
  budget history 200 entries/8 MiB teks, branch/trim/checkpoint tetap.
- Source mode/prompt/Open menolak pembukaan; Save/Export yang sudah berjalan
  boleh. Selama draft aktif, **mutasi/navigasi/history/mode/file commands serta
  New/Open/Close/Save/Export baru dipause**; selesaikan Apply atau Cancel dulu.
  Draft bukan konten Save. Balasan worker async tetap ditangani dengan guard token
  lama; save ack hanya menandai Source snapshot yang benar, bukan draft.
- Panel modal mengganti widget tree dan memberi native focus ke input draft.
  Apply/Cancel memulihkan logical Source/Find/Go-to focus pada Split, mode tetap.
  Preview tidak memfokuskan Source. Search/cache/revision diperbarui hanya Apply
  efektif. Tidak ada preview free typing atau click-to-glyph caret mapping.

## Tes

**30 Rust baru**, total Cargo **611**, lulus pada CI di bawah:

- **8 portable planner**: exact raw payload/outer bytes/caret, header/genuine
  empty, escaped pipe/backslash parity/trailing whitespace, canonical rectangle/
  edge pipes/EOF, full snapshot/range/index/dimension, 16 KiB/no-op/UTF-8/caps,
  empty payload insertion dan lexical fence counterexample.
- **9 Document**: raw-byte Atomic/caret/checkpoint/cache/UndoRedo; header/empty vs
  delimiter; cold/forged auth; no-op/invalid/redo; nested/code/HTML/ragged/oversize
  refusal; EOF/escaped pipe/empty/padded/even-slash repeated re-editability;
  stale Undo/reset; ordinary Source barrier; save ack/branch.
- **9 controller**: draft isolation/Cancel, Apply/history/mode, no-op/redo,
  invalid/oversize recovery, modal command/close/file guard, stale draft retention,
  Source/prompt/Open vs inflight Save/Export, Split Find refresh/context, save ack
  during draft then Apply/Undo checkpoint semantics.
- **2 model + 2 adapter**: trimmed raw real-cell ranges vs normalized cells,
  nested/ragged/extra/missing-edge/fallback inert, old action prefix unchanged,
  stamped cell labels/targets/message and no padded-cell actions.

Core **226** (218 lama + 8 planner) adalah subset Cargo, bukan tambahan tes unik.
Python **33** (31 lama + 2 cell helper) mengunci literal expected Source line,
raw Unicode draft/clipboard/input focus, action ordering, clicks/shortcuts,
Cancel/no-op/invalid Apply, scoped history/exit dan exclusive suite dispatch.
Private DISPLAY guard mencakup kelima suite.

## Suite GUI privat kelima

Gate lama **116 regression + 9 task-batch + 9 table-alignment + 9 table-rows**
tetap utuh. `--suite table-cells` mendapat app/Xvfb/XDG/env/clipboard baru dan
step workflow sendiri. Lima suite tetap masing-masing alarm **180s** / step
**5 menit**, job budget **45 menit**. Tidak ada retry/runtime hooks, Source click/
refocus workaround atau perubahan expected bytes/koordinat gate lama.

Expected **14 checks** = 13 exact Copy + 1 natural clean exit:

1. Fresh welcome dan fixture canonical table, UTF-8/raw markup/escaped pipe/tab.
2. Draft initial raw payload Copy membuktikan native input focus, lalu Escape
   membuang perubahan dan Source Copy tetap persis.
3. Clipboard paste raw Unicode Markdown ke draft, Copy mengakui konsumsi paste,
   Enter Apply lalu Source Copy hanya payload berubah.
4. Preview Undo/Redo memulihkan byte persis dalam satu langkah.
5. Split unchanged Apply menutup tanpa extra history; unescaped pipe Apply
   ditolak, panel tetap terbuka, Escape lalu Source tetap sama.
6. Split Apply cell kedua dan ketik X tanpa Source refocus: caret setelah payload.
7. Dua Source Undo memisahkan typing dan Apply; dua Undo berikutnya mengembalikan
   welcome clean, membuktikan Cancel/no-op/invalid draft tidak menambah langkah.
8. Client Close keluar alami 0, bukan kill sebagai oracle.

Sepuluh klik hanya picker/opsi produk, Copy semuanya `refocus=False`. Title
`[cell draft]` digunakan sebagai acknowledgement panel, bukan hook aplikasi.
Artefak di `table-cells/`, screenshot preview/invalid/split bukan pixel oracle.
Picker width168/font11/padding2/6 dan Source button24px lama tetap.

```sh
./scripts/test-gui-x11.sh --artifacts "$(mktemp -d)"
./scripts/test-gui-x11.sh --suite task-batch --artifacts "$(mktemp -d)"
./scripts/test-gui-x11.sh --suite table-alignment --artifacts "$(mktemp -d)"
./scripts/test-gui-x11.sh --suite table-rows --artifacts "$(mktemp -d)"
./scripts/test-gui-x11.sh --suite table-cells --artifacts "$(mktemp -d)"
```

## Status eksekusi

Lokal: format Rust, **226 strict core**, **33 Python**, `git diff --check` lulus.
[CI 39792827763](https://github.com/Indrawan007/Hylite/actions/runs/39792827763)
**success** pada `38595306de76410432bf7b24e5b788282658bf19`, `gh run watch
--exit-status` keluar 0. Native build debug, format, strict Clippy `-D warnings`,
**611 Cargo**, **226 core** (subset Cargo), **33 Python**, dan
**116 regression + 9 batch + 9 alignment + 9 rows + 14 cells GUI** lulus.
Total **157 checks di lima suite**, bukan deadline gabungan diperbesar:
139 clipboard (137 Source + 2 draft input), 12 title, 6 exit alami.

Job `119873032327`, 22 Oktober 2026: regresi 14:11:21–14:14:00 UTC, batch
14:14:00–14:14:13, alignment 14:14:13–14:14:27, rows 14:14:27–14:14:40,
cells **14:14:40–14:15:03**. Semua step termasuk Complete job success pada
14:15:06; run final success teramati dengan updated_at 14:22:22. Endpoint job/
check-run sempat tertinggal publikasi status/annotation setelah semua step hijau;
ini tidak dipakai untuk mengklaim diagnosis/perbaikan startup lama.

Seluruh 12 body phase regresi serta body batch/alignment/rows diperiksa sama
secara AST dengan Stage 6g, tanpa perubahan byte/klik/refocus oracle lama.
Artifact API `12432681330`, `gui-x11-smoke`, 1.866.592 bytes, belum expired saat
diperiksa. Screenshot belum direview visual; bukan pixel/glyph/layout oracle atau
bukti Wayland/Hyprland. Kelulusan berdasarkan clipboard byte-exact, native input/
focus/history/modal lifecycle serta clean client exit X11.

## Batas/manual

- [ ] Query/Go-to focus runtime setelah modal; header/genuine empty/EOF cell UI.
- [ ] 16 KiB input/paste/Tab runtime, semua menu/keyboard/scroll, accessibility.
- [ ] Light/Dark/HiDPI, glyph/caret/scroll visual, Arch/Hyprland/Wayland/portal/GPU/IME/RTL.

Belum ada inline editing bebas paragraph/list/cell, rich-text selection formatting,
cell selection/navigation, glyph↔caret map, selection/scroll sync, incremental
parse/virtualisasi atau speed/RAM benchmark. Full reconstruction/projection O(n),
LF canonical dan gutter yang ditunda tetap. Startup 6b exit101 lama masih belum
terdiagnosis; modal/suite baru tidak diklaim memperbaikinya.
