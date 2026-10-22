# Stage 6g — duplikasi/hapus body row tabel root

Successor [alignment 6f](testing-stage6f.md). Ini operasi whole raw row, bukan
editing isi sel atau WYSIWYG umum.

## Kontrak produk dan batas konservatif

- Picker tabel yang sama kini berlabel **Table actions…**. Semua opsi alignment
  tetap pertama, urutan dan ukuran tetap; kemudian `Row N: Duplicate / Delete`.
  N dimulai 1 untuk body pertama. Tidak ada opsi untuk header atau delimiter.
- Hanya root projected table dengan delimiter kanonis 6f serta **seluruh body
  berupa physical line pipe-leading, indent 0–3 spasi dan LF-terminated** yang
  mendapat row actions. Jika ada body tanpa leading pipe, multiline/nonkanonis,
  atau EOF tanpa LF, row actions tidak diberikan; alignment lama dapat tetap
  tersedia. Nested/quoted/literal/fallback tidak mendapat row actions.
- Scope konservatif ini disengaja. Tidak menambahkan LF diam-diam ke EOF dan
  tidak mencoba menemukan baris dari display cells yang sudah dinormalisasi.
- Duplicate menyalin satu **whole physical line** persis, termasuk indent/pipes/
  raw inline/UTF-8/escaped pipe/entity/ragged cells/whitespace/LF, lalu menyisipkan
  sesudah baris terpilih. Caret pada awal duplikat (sebelum indent/pipe), collapse.
- Delete menghapus hanya line terpilih termasuk LF. Caret pada awal line berikut
  atau posisi bekas line terakhir. Header/delimiter dan semua byte di luar line
  tetap verbatim. Menghapus body terakhir mempertahankan tabel header-only.
- Batas lama **12 kolom / 128 row termasuk header / 512 sel** tetap. Duplicate
  yang melampaui salah satu cap dihilangkan dari menu dan ditolak di API tanpa
  prefix parsial, checkpoint atau penghapusan redo. Delete tetap diperbolehkan
  pada tabel valid yang tepat di cap. Tabel yang sudah fallback tidak diedit.
- Target: **full Snapshot + exact table range + body row index + total row count
  termasuk header + column count + whole physical line range**. Document menuntut
  exact membership pada **cache warm root Table**. Metadata hanya ada bila jumlah
  physical body lines sama dengan projected rows dan setiap projected row range
  berada di physical line yang bersesuaian, selain auth delimiter 6f.
- Lexical planner sendiri tidak membuktikan provenance. Cold/stale/forged/header/
  wrong index/count/columns/range ditolak tanpa mutasi. Target lama tetap stale
  setelah Undo ke byte identik atau replacement dokumen. Tidak rebase.
- Satu aksi satu Atomic **`PreviewTable`** trusted edge yang sudah digunakan 6f;
  tidak menambah stack izin atau jenis history. Tidak coalesce. Preview Undo/Redo
  tetap top-only eligible task/heading/table edge; Source edit tetap barrier.
  Budget history 200 entries / 8 MiB teks, trimming/branching tetap.
- Source mode/prompt/Open menolak command; Save/Export biasa boleh. Revision,
  cache/search, checkpoint/save ack, mode dan logical Source/query focus memakai
  jalur lama. Native/free text Preview tetap tidak diperbolehkan.

## Tes

**25 Rust baru**, total Cargo **581**, lulus pada CI di bawah:

- **8 portable planner**: exact raw copy/delete/caret/prefix/suffix, protected
  header/delimiter, last-row deletion, noncanonical/EOF/UTF-8/range rejection,
  full snapshot/index/count/columns/line, exact caps dan lexical fence counterexample.
- **8 Document**: raw-byte/history/caret/checkpoint; cold/forged auth; nested/code/
  HTML/non-pipe/EOF/fallback refusal; header-only result; row/cell caps dan redo;
  stale Undo/reset; ordinary Source barrier; saved checkpoint/branch/mixed alignment.
- **2 model + 2 adapter**: physical line metadata vs normalized cells; nested/
  EOF/non-pipe inert; alignment options tetap di depan, row labels/indices,
  cap Duplicate omission dan Delete availability; unsupported rows alignment-only.
- **5 controller**: Preview raw edits/history/mode; Source/prompt/Open refusal vs
  Save/Export; Split Find refresh/context; stale/native typing refusal; save ack.

Core **218** (210 lama + 8 planner) adalah **subset Cargo**, bukan tambahan tes
unik. Python **31** (29 lama + 2 row helpers) mengunci probe/bytes/klik/no-refocus/
exit dan exclusive suite dispatch. Private DISPLAY guard mencakup empat suite.

## Suite GUI privat keempat

Semua phase/urutan/oracle gate lama tetap: **116 regression + 9 task-batch +
9 table-alignment**. `--suite table-rows` menambah invokasi wrapper/Xvfb/app/XDG/
env/clipboard baru dan step workflow sendiri. Keempat runner tetap alarm **180s**,
step timeout **5 menit**, job budget **45 menit**. Tidak ada retry, aplikasi test
hook, Source click/refocus workaround atau pelemahan gate lama.

Expected row suite **9 checks** (8 exact Copy + 1 clean natural exit):

1. Fresh clean welcome.
2. Fixture UTF-8/raw inline/escaped pipe/entity/indent/dash/ragged row/terminal tab.
3. Preview Row 1 Duplicate: satu whole physical raw line tersalin persis.
4. Satu Preview Undo: semua source semula kembali.
5. Preview Redo: raw duplikat dan suffix persis.
6. Split Row 2 Delete lalu ketik X tanpa Source refocus: sentinel pada awal physical
   line berikutnya, bukan pada cell display atau query.
7. Dua Source Undo memisahkan native typing dan row Delete.
8. Undo Duplicate/paste kembali exact welcome clean tanpa extra history.
9. Client Close keluar alami 0, bukan kill sebagai oracle.

Empat klik hanya kontrol produk. Semua Copy `refocus=False`; helper juga mengunci
literal expected lines independen, bukan hanya memakai replacement expression.
Evidence di `gui-x11-smoke/table-rows/`, `rows-preview.png` / `rows-split.png` bukan
pixel/glyph oracle. Picker width168/font11/padding2/6 dan Source button24px tetap.

```sh
./scripts/test-gui-x11.sh --artifacts "$(mktemp -d)"
./scripts/test-gui-x11.sh --suite task-batch --artifacts "$(mktemp -d)"
./scripts/test-gui-x11.sh --suite table-alignment --artifacts "$(mktemp -d)"
./scripts/test-gui-x11.sh --suite table-rows --artifacts "$(mktemp -d)"
```

## Status eksekusi

Lokal: **218 strict core**, **31 Python**, format Rust dan `git diff --check`
lulus. [CI 39787971305](https://github.com/Indrawan007/Hylite/actions/runs/39787971305)
**success** pada `3590d56d6e5933d85b7ad74e868b5fc562a97ee0`. Job
`119856055204`, 22 Oktober 2026, 13:34:07–13:41:52 UTC: format, native build debug,
Clippy `-D warnings`, **581 Cargo**, **218 strict core** (subset Cargo), **31 Python**,
dan **116 regression + 9 task-batch + 9 table-alignment + 9 table-rows GUI** lulus.
Total **143 checks di empat suite** (126 clipboard, 12 title, 5 exit alami), bukan
satu suite dengan deadline diperbesar.

Regresi 13:38:24–13:41:06 UTC; batch 13:41:06–13:41:19; alignment
13:41:19–13:41:33; rows 13:41:33–13:41:46. Alarm180s/step5min masing-masing tetap.
Semua 12 body phase regresi serta body batch/alignment diperiksa sama secara AST
dengan Stage 6f; tidak mengubah koordinat/bytes/refocus/oracle gate lama.

Artifact API `12431131246`, `gui-x11-smoke`, 1.742.544 bytes, belum expired saat
diperiksa. Screenshot belum direview visual; bukan pixel/glyph/layout oracle atau
bukti Wayland/Hyprland. Kelulusan berdasarkan byte-exact clipboard, native input/
focus/caret/history dan lifecycle X11.

## Batas/manual

- [ ] Query-focus runtime, semua row/scroll menu/keyboard menu, cap controls visual.
- [ ] Light/Dark/HiDPI/accessibility, glyph/caret/scroll visual.
- [ ] Arch/Hyprland/Wayland, file portal, GPU, compositor shortcuts, IME/RTL.

Tidak ada arbitrary Preview body/cell text editing, AST serialization, glyph↔caret
atau selection/scroll sync, incremental parsing/virtualisasi maupun klaim speed/RAM.
Projection/reconstruction tetap O(n); LF canonical dan gutter ditunda tetap.
Startup 6b exit101 lama masih belum terdiagnosis; suite baru bukan klaim fix.
