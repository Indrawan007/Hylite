# Stage 6f — alignment kolom tabel GFM root

Successor [batch task 6e](testing-stage6e.md). Kontrol delimiter terbatas, bukan
editing teks sel atau WYSIWYG umum.

## Kontrak produk

- **Column alignment…** pada action bar tabel Preview/Split menawarkan
  `Column N: Default / Left / Center / Right`, tanpa pilihan yang sudah aktif.
  Default berarti tanpa colon. Hanya tabel root yang diproyeksikan dan mempunyai
  delimiter kanonis terverifikasi; tabel nested/quoted, code/HTML/literal,
  nonkanonis dan fallback oversized tidak memperoleh kontrol alignment.
- Cap tabel lama tetap **12 kolom / 128 row termasuk header / 512 sel**. Bukan
  menaikkan cap atau mengedit prefix tabel yang terpotong.
- Hanya token delimiter kolom terpilih berubah: tambah/hapus colon di tepi run
  dash yang ada. Panjang dash, indent (0–3 spasi), whitespace/tab di antara sel,
  pipes, header/body raw inline, Unicode, escaped pipe, entity, ragged row dan
  terminal tab tetap verbatim. Tidak ada normalisasi/serialisasi AST.
- Target menyertakan **full Snapshot + exact table range + column index/count +
  delimiter marker range + expected alignment**. Document mengharuskan anggota
  `Kind::Table` root pada **cache warm saat ini**, dengan marker/alignment persis.
  Planner lexical sendiri tidak membuktikan provenance. Model baru memproyeksikan
  marker jika hasil lexical sama dengan alignment dari pulldown-cmark.
- Cold/stale/forged/wrong column/count/marker/expected alignment dan no-op ditolak
  tanpa mutasi, revision, checkpoint atau penghapusan redo. Tidak rebase target.
- Satu pilihan efektif adalah **satu Atomic `PreviewTable` history edge** yang
  tidak coalesce. Preview Undo/Redo hanya eligible edge paling atas, kini meliputi
  task/heading/table, dan tidak melewati Source edit. Source/Split tetap history
  umum. Budget 200 entries / 8 MiB teks history, branching/trimming tetap.
- Caret collapse sesudah token delimiter baru, sebelum whitespace/pipe berikutnya.
  Cache/search/revision terbarui, mode tetap, logical Source/query focus dipulihkan
  di Split; Preview tidak memfokuskan Source. Source mode, prompt dan Open menolak
  command. Save/Export biasa diizinkan, dengan saved-snapshot semantics lama.
- Picker width 168, font 11, padding 2/6 di bar yang sama; tombol Source tetap
  tinggi 24 px. Tidak menambah bar baru atau mengubah koordinat probe regresi lama.

## Tes

26 tes Rust baru (total Cargo **556**, lulus pada CI di bawah):

- **8 portable planner**: semua pasangan alignment, exact raw bytes/dash count,
  optional edge pipes/indent/tab/single dash, malformed colon/kolom/range/UTF-8,
  snapshot/marker/expected state/no-op, serta counterexample
  lexical-only yang tetap memerlukan auth model.
- **1 portable history**: `PreviewTable` Atomic/eligible, Undo/Redo/Source barrier.
- **8 Document**: exact token/caret/checkpoint/UndoRedo, warm/snapshot/forged auth,
  nested/literal/Setext/fallback refusal, no-op/redo, stale Undo/reset, Source
  frontier, saved checkpoint/branch dan independent column transactions.
- **2 model + 2 renderer adapter**: root marker ranges dan parser agreement,
  nested/noncanonical/fallback inert, stamped choices dan no-op omission.
- **5 controller**: mode/raw bytes/checkpoint/history; Source/prompt/Open refusal
  vs Save/Export; Split Find context; stale/native typing refusal; async save ack.

Core mandiri **210** (201 lama + 8 planner + 1 history) adalah **subset Cargo**,
bukan tambahan 210 tes unik. Python **29** (27 lama + 2 alignment helper) mengunci
expected bytes/klik/shortcut/no-refocus/exit dan exclusive suite dispatch. Guard
private-display diperluas ke ketiga suite sebelum subprocess dimulai.

## Suite GUI privat ketiga

Default `regression` tetap **116** checks lama dalam urutan yang sama; `task-batch`
tetap **9**. `--suite table-alignment` memakai wrapper/Xvfb/app/XDG/env/clipboard
baru dan step workflow sendiri. Ketiganya tetap alarm **180 detik**, step timeout
5 menit, job budget 45 menit; tidak ada retry, runtime hook atau refocus workaround.

Alignment mengharapkan **9 checks** (8 byte-exact Copy + 1 clean natural exit):

1. Fresh welcome clean.
2. Fixture raw UTF-8/header/pipes/indent/dash/entity/escaped pipe/ragged row/tab.
3. Preview pilih Column 1 Center: hanya token pertama menjadi `:---:`.
4. Preview Undo: semua byte awal kembali.
5. Preview Redo: hasil Center persis.
6. Split pilih Column 2 Left, ketik X tanpa klik Source: sentinel sesudah `:--`.
7. Dua native Source Undo memisahkan typing X dan perubahan kolom kedua.
8. Undo kolom pertama dan paste kembali welcome clean, tanpa extra history.
9. Client Close keluar alami 0; bukan kill sebagai oracle.

Empat klik hanya kontrol produk; semua Copy `refocus=False`. Artefak suite di
`gui-x11-smoke/table-alignment/`, dengan `alignment-preview.png` dan
`alignment-split.png`. Screenshot bukan pixel/visual alignment oracle.

```sh
./scripts/test-gui-x11.sh --artifacts "$(mktemp -d)"
./scripts/test-gui-x11.sh --suite task-batch --artifacts "$(mktemp -d)"
./scripts/test-gui-x11.sh --suite table-alignment --artifacts "$(mktemp -d)"
```

## Status eksekusi

Lokal: format Rust, **210 strict core**, **29 Python**, dan `git diff --check`
lulus. [CI pertama 39784221190](https://github.com/Indrawan007/Hylite/actions/runs/39784221190)
pada `7ed8fb01b07e2bae54479cf6988cdf14d3f88f52` lulus build debug/Clippy strict,
**556 Cargo**, **116 regresi + 9 batch GUI**, tetapi suite alignment gagal di Copy
keenam: oracle `replace("--:")` juga mengenai suffix token Center `:---:` kolom
pertama. Actual bytes aplikasi sudah benar: kolom kedua menjadi `:--X` setelah
pilih Left lalu mengetik X tanpa refocus. Oracle baru memakai delimiter dengan
edge pipes/spacing spesifik kolom kedua; helper menambah assertion literal line
independen. Tidak mengubah produk, klik/timeout/refocus atau oracle suite lama.

[CI 39784983010](https://github.com/Indrawan007/Hylite/actions/runs/39784983010)
**success** pada `894227f3d863a396bc2f05a8fccbb5d1a8571fd5`, termasuk dua fixture
negatif tab-leading delimiter/fallback dan oracle yang diperbaiki. Job
`119845680831`, 2026-10-22 13:13:43–13:20:38 UTC: **556 Cargo**, **210 strict core**
(subset Cargo), **29 Python**, format/build debug/Clippy `-D warnings` dan
**116 regresi + 9 task-batch + 9 table-alignment GUI** lulus. Total **134 checks**
di tiga suite (118 clipboard, 12 title, 4 exit alami), bukan satu deadline besar.
Regresi 13:17:24–13:20:07 UTC; batch 13:20:07–13:20:20; alignment
13:20:20–13:20:34. Masing-masing alarm 180s / step 5 menit tetap.

Artifact API `12428428377`, `gui-x11-smoke`, 1.668.936 bytes, belum expired saat
diperiksa. Screenshot belum direview visual; bukan pixel/layout/alignment glyph
oracle atau bukti Wayland/Hyprland. Kelulusan berdasarkan byte-exact clipboard,
shortcut/fokus, history dan lifecycle native X11.

## Batas/manual

- [ ] Query-focus runtime alignment, semua kolom/semua arah menu/keyboard menu.
- [ ] Alignment glyph/cell visual, disabled controls, Light/Dark, HiDPI/accessibility.
- [ ] Arch/Hyprland/Wayland, portal file dialog, IME/RTL, compositor/GPU.

Tidak ada editing teks bebas sel/body Preview, selection/glyph↔caret mapping,
selection/scroll sync, parsing incremental/virtualisasi atau klaim performa.
Full projection/reconstruction tetap O(n), LF canonical dan gutter tetap ditunda.
Startup kedua 6b exit 101 lama tetap belum terdiagnosis; penambahan suite bukan
klaim perbaikan startup tersebut.
