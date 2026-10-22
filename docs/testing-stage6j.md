# Stage 6j — explicit blank body-row insertion

Successor [cell draft navigation 6i](testing-stage6i.md). CI dokumentasi akhir6i
[39864686040](https://github.com/Indrawan007/Hylite/actions/runs/39864686040)
pada `8440147` kini success. Stage ini menambah dua aksi eksplisit, bukan implicit
row creation saat menavigasi draft dan bukan general inline WYSIWYG.

## Kontrak produk

- **Table actions… → Row N: Insert blank before / Insert blank after**.
  Anchor harus real body row (N mulai1), bukan header/delimiter/padding. Tidak ada
  insert lewat tabel header-only; tidak ada shortcut atau auto-insert pada Next.
- Baris baru berbentuk indent anchor0–3 spasi, `|`, **dua spasi + `|` per kolom
  header**, kemudian LF. Contoh dua kolom tanpa indent: `|  |  |\n`. Bahkan jika
  anchor ragged, jumlah sel baru mengikuti header, tidak menyalin isi anchor.
- Source hanya berubah pada zero-length insertion range di awal/akhir physical
  LF line anchor. Byte header/delimiter/anchor/sel lain/prefix/suffix/terminal tab
  tidak diserialisasi atau dinormalisasi; UTF-8/markup/escaped pipes/entity utuh.
- Caret Source sesudah insert berada **di antara dua spasi sel pertama baru**
  (`range.start + indent + 2`), bukan awal baris lama atau ujung baris baru.
  Split mengembalikan Source focus bila Find/Go-to tidak aktif.
- Satu insert efektif = satu Atomic `PreviewTable` edge. Preview scoped Undo/Redo
  dan Source Undo memulihkan exact bytes/caret/checkpoint. Refusal tidak memutus
  grup/redo atau menambah revision/history; Source typing tetap history barrier.
- Target memakai full snapshot document/revision/state, root table range,
  dimensions, row index dan physical line range. Document mengautentikasi warm
  current root projection sebelum planner; lexical pipe scan bukan provenance.
- Hanya canonical root pipe-leading LF tables yang sudah mendukung6g body rows.
  Nested quote/list/fence/HTML, noncanonical lines, EOF tanpa LF dan fallback
  ditolak. Growth harus tetap <=128 row termasuk header, <=12 kolom dan <=512 sel.
- Source mode, confirmation/pending Open dan active cell-draft modal menolak aksi;
  Save/Export yang berjalan tetap memakai snapshot saat dimulai. Save ack dan
  branch redo mengikuti identity checkpoint lama, bukan equality byte saja.
- Cache projection lama diinvalidasi setelah insert; target baru harus didapatkan
  dari parse baru. Search direfresh, logical Find/Go-to context tidak ditutup.
- Opsi baru **ditambahkan setelah seluruh alignment/duplicate/delete/cell edit**.
  Label/prefix/ukuran picker/top bar lama tetap; tidak menggeser indeks opsi lama.

## Implementasi dan tes

`preview_table_row::Action` mendapat InsertBefore/InsertAfter; planner reuse exact
line/snapshot verification dan growth cap Duplicate sebelum bounded allocation.
`Document::edit_preview_table_row`/controller yang sama mempertahankan authentication,
file guards, transaction type, cache invalidation, search dan focus restoration.
Tidak ada authoritative buffer/AST serialization baru.

**17 tes Rust baru**, total Cargo **649** (632 lama +4 planner +6 Document
+2 picker/model +5 controller):

- Planner exact insertion/prefix/suffix/caret/indent/Unicode; canonical1–12 kolom
  dari ragged anchor; stale/forged/header/dimensions/overflow; growth caps dan EOF.
- Document fresh parser real blank cells, one-step history/caret/checkpoint;
  cold/forged/stale/replace; caps preserve redo; nested/literal/EOF rejection;
  explicit fresh cell edit sesudah insert; save ack/branch/Source barrier.
- Picker exact seluruh12 label prefix lama di fixture2x2, append target full
  snapshot/range/action; suppression header-only/EOF/nested/fallback/caps.
  Dua assert count picker lama disesuaikan untuk tail baru, prefix/old cells
  tetap diuji; ini bukan relaksasi oracle GUI lama.
- Controller Preview/Split history/caret, Source/prompt/Open/modal guards dan
  Save/Export token immutable, Find offset refresh/context, stale redo/barrier,
  delayed save ack checkpoint identity.

**235 strict core** adalah subset Cargo, bukan235 tes tambahan unik. Python
**37** menambah2 helper untuk literal expected Source/click/undo sequence,
no-refocus/natural exit dan exclusive dispatch. Private DISPLAY guard mencakup
semua tujuh suite sebelum subprocess.

## Suite GUI privat ketujuh

Enam suite lama **116+9+9+9+14+16** tetap utuh. Semua17 body exercise functions
lama (12 regression phases +5 suite independen) sama secara AST terhadap8440147.
Runner baru `--suite table-insert`: Xvfb/app/XDG/env/clipboard baru, alarm180s,
step5min sendiri, job45min tetap. Tidak ada retry/hook/Source refocus workaround,
perubahan oracle bytes/koordinat atau kenaikan deadline gate lama.

Aktual **11 checks** =10 exact Source Copy +1 natural clean client Close exit0:

1. Welcome clean dan fixture raw Unicode/markup/escaped pipe/entity/terminal tab.
2. Preview Insert before hanya menambah satu baris blank berindent anchor.
3. Satu Preview Undo mengembalikan fixture, Redo mengembalikan exact inserted row.
4. Undo sebelum Split mengembalikan original menu prefix/table.
5. Split Insert after lalu native X tanpa Source refocus menghasilkan first-cell
   ` | X |  |\n` setelah raw anchor yang tidak berubah.
6. Undo typing menyisakan insert, Undo berikut menghapus insert, Undo paste kembali
   welcome clean tanpa hidden transactions. Client Close exit alami, bukan kill.

Empat klik hanya picker/opsi produk. Dua screenshot `table-insert/insert-preview.png`
dan `insert-split.png` adalah evidence, bukan pixel/glyph oracle. Total aktual
**184 GUI** =164 clipboard (155 Source +9 draft input),12 title,8 natural exits.

```sh
./scripts/test-gui-x11.sh --suite table-insert --artifacts "$(mktemp -d)"
# Tetap jalankan keenam suite lama; suite ini bukan substitusinya.
```

## Status eksekusi

Lokal: Rust format/harness format, **235 strict core**, **37 Python**, diff check
lulus. [CI 39865519334](https://github.com/Indrawan007/Hylite/actions/runs/39865519334)
pada `d5e009b99bff9ee72e1317bd66c2809a033894e3` **success**, job
`120126611644`, 22 Oktober2026 22:30:58–22:38:34 UTC /23 Oktober Makassar.
Native debug build, Clippy strict, format, **649 Cargo**, **235 strict core**
(subset), **37 Python**, serta **116+9+9+9+14+16+11 GUI** lulus. Annotation
memuat semua11 PASS insert dan totals tujuh suite lama/baru; snapshot/caret/history
oracle tidak dilonggarkan. Artifact `12463529269`, `gui-x11-smoke`,2.049.263 bytes,
belum expired saat diperiksa; screenshot belum direview visual.

## Batas/manual

- [ ] Runtime insert sekitar ragged/1kolom/12kolom/3spasi anchor; large menu scroll,
  keyboard traversal, active Find/Go-to physical focus, header-only insertion UI.
- [ ] Glyph/caret visuals, selection, IME/RTL/accessibility/HiDPI, desktop Arch /
  Hyprland /Wayland /portal /GPU. X11 tiny-skia bukan bukti target compositor.

Tidak ada insert/delete kolom atau header row, row movement, drag reorder, bulk rows,
auto-add row, multi-cell selection/draft session undo, inline typing/glyph mapping,
selection/scroll sync, incremental parse/virtualisasi atau benchmark speed/RAM.
Source reconstruction/history tetap O(n), gutter/LF canonical dan startup6b exit101
lama yang belum terdiagnosis tetap; gate ini bukan klaim memperbaiki startup.
