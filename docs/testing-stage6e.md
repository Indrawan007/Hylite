# Stage 6e — batch centang task yang didukung Preview

Successor [Setext 6d](testing-stage6d.md). Ini batch checkbox, bukan WYSIWYG umum.

## Kontrak produk dan otorisasi

- Toolbar **Check tasks / Clear tasks** tersedia di Preview/Split. Semua task
  kanonis dalam model Preview yang didukung ikut ditargetkan, termasuk nested/
  quoted/ordered task. Bukan seluruh pola `[ ]` dalam source: code/HTML/literal,
  unsupported marker `[	]` dan konten fallback yang tidak dimodelkan sebagai task
  tidak diubah. Source mode, prompt dan Open menolak command seperti single task.
- Check mengubah `[ ]` ke `[x]`; `[X]`/`[x]` yang sudah checked tetap persis.
  Clear mengubah kedua kasus checked ke `[ ]`. Byte selain marker middle byte
  yang perlu berubah dipertahankan, termasuk raw inline, Unicode dan terminal tab.
- Satu batch adalah **satu transaksi Atomic**/checkpoint/revision dan satu Undo
  jika budget history memungkinkan,
  bukan loop commit single-task. Replacement dapat melintasi banyak blok tetapi
  byte di antaranya disalin verbatim. Caret sesudah middle byte marker terakhir
  yang berubah (sebelum `]`), selection collapse. Tidak ada serialisasi AST.
- `BatchTarget` memuat full snapshot, desired state dan **daftar kanonis lengkap**
  dalam urutan model/source, termasuk anggota yang no-op. Document menuntut
  exact equality dengan daftar dari model warm saat ini. Omission, duplikasi,
  reorder, forged item/code/marker, outer/member snapshot stale dan cold ditolak
  seluruhnya. Tidak rebase atau menerima subset seolah-olah "all tasks".
- Planner memvalidasi setiap single Target, source order/non-overlap marker,
  expected state dan snapshot sebelum mengembalikan perubahan. Planner lexical
  saja tidak bisa membuktikan provenance atau kelengkapan; itu tugas Document.
- **Maksimal 1024 task kanonis** per batch. Jika model melampaui cap, kedua tombol
  disabled dan command ditolak tanpa prefix parsial. No-op/empty/unsupported-only
  juga disabled; forged no-op tidak menghapus redo atau membuat checkpoint.
- Menggunakan `PreviewTask` Atomic/non-coalescing yang sama dengan single checkbox.
  Preview Undo/Redo tetap top-only eligible task/heading edge, tidak melompati
  Source edit. Source/Split tetap history umum; cap history 200 entries/8 MiB
  teks, trim/branch/reset dan Ignored-only shortcuts tidak berubah.
- Save/Export biasa boleh; token dan saved-snapshot dirty semantics tetap.
  Revision/cache/search terbarui, mode tetap. Split memakai logical Source/query
  focus; Preview tidak memfokuskan Source. Native Preview typing tetap ditolak.

## Tes

20 Rust baru (total Cargo 530, lulus pada CI di bawah):

- **7 portable planner**: check/clear exact bytes/case/Unicode, empty/no-op,
  outer/member snapshot, ordering/duplicate/malformed/late mismatch, exact cap
  dan oversized reject-all, serta lexical-only counterexample.
- **8 Document**: supported nested tasks vs literals/unsupported, complete-list
  auth, cold/stale/replace, no-op/redo, cap tanpa prefix parsial, Source frontier/
  branch, saved checkpoint dan mixed single/batch/heading history.
- **5 controller**: button eligibility/mode/cap/no-op, prompt/Open/Save/Export,
  stale/general typing/one-step history, Split Find context, saved ack/Undo.

Tidak mengubah expected bytes atau oracle tes lama. Metadata history tetap
terikat record; tidak ada stack/set izin baru.

## Dua suite GUI terisolasi, bukan relaksasi deadline

Suite regresi default masih mengeksekusi **seluruh 116 pemeriksaan lama** dalam
urutan yang sama, dengan global alarm **180 detik**, step timeout 5 menit dan
lifecycle natural exit lama. `exercise_suite` default regression; helper baru
mengunci daftar dispatch lengkap dan menolak nama suite yang tidak dikenal.

Probe batch memakai `--suite task-batch` di **invokasi wrapper/Xvfb terpisah**,
XDG/env/app/clipboard temporer baru dan alarm 180 detik yang sama. Workflow
menambah step tersendiri (5 menit); job budget lama 45 menit tidak diubah. Tidak
memecah atau mengurangi suite regresi agar tampak lulus, tidak menambah retry,
hook aplikasi, deadline besar pada regresi atau Source refocus workaround.

Suite batch expected **9 checks**: 8 Copy byte-exact dan 1 clean client Close
exit alami 0. Copy: welcome baru, fixture mixed `[ ]`/`[X]`/quoted/unsupported tab/
code literal/Unicode/terminal tab; Check; Undo; Redo + click Check no-op; Split
Clear + sentinel X pada caret middle-byte end; dua Undo memisahkan Source typing
vs Clear; Undo Check/paste ke welcome clean tanpa extra history. Tiga klik hanya
kontrol produk; setelah initial Source setup, semua Copy `refocus=False`.

Artifact lama tetap di root `gui-x11-smoke`, evidence batch di subfolder
`task-batch/` agar tidak menimpa summary/log/screenshot regresi. Summary mencatat
nama suite. `batch-preview.png`/`batch-split.png` bukan pixel oracle. Tiga Python
baru mengunci probe batch, full regression dispatch, dan private-display guard
kedua suite sebelum subprocess dimulai.

```sh
# Fresh evidence directory untuk tiap invokasi. Kedua suite wajib dijalankan.
./scripts/test-gui-x11.sh --artifacts "$(mktemp -d)"
./scripts/test-gui-x11.sh --suite task-batch --artifacts "$(mktemp -d)"
```

## Status eksekusi

[CI 39771530257](https://github.com/Indrawan007/Hylite/actions/runs/39771530257)
**success** pada `3fb4199beffc62afca5fb7c5f40899bc2cbbb0aa`. Job
`119799991102`, 2026-10-22 11:33:56–11:38:05 UTC: **530 Cargo**, **201 strict core**
(subset Cargo), **27 Python**, **116 regression + 9 task-batch GUI**, format/build
debug/Clippy `-D warnings` lulus. Total 125 checks di **dua** suite (110 clipboard,
12 title, 3 exit alami), bukan satu suite tunggal dengan deadline lebih besar.
Regresi 11:35:08–11:37:48 UTC; batch 11:37:48–11:38:01 UTC, masing-masing alarm
180s dan step timeout 5 menit. Artifact API `12422742801`, `gui-x11-smoke`,
1.520.409 bytes, belum expired saat diperiksa. Screenshot belum direview visual;
ini bukan pixel/layout oracle atau verifikasi Wayland/Hyprland.

Percobaan [CI 39770915857](https://github.com/Indrawan007/Hylite/actions/runs/39770915857)
lulus 530 Cargo/build/Clippy, tetapi regresi GUI gagal pada probe block-source:
click y=151 tidak menghasilkan Source Copy. Tombol baru fixed 112 px memakai
font default 16 dan padding horizontal bawaan 20 px; label Check tasks dapat
wrap dan menambah tinggi editing bar. Font label produk menjadi explicit 14
agar muat. Run di atas lulus seluruh probe lama, termasuk click block source,
heading/task/source focus, tanpa mengubah koordinat/copy/refocus/timeout atau
expected bytes probe lama. Suite batch kini juga terbukti pada runtime nyata,
bukan hanya mock Python. [Detail GUI](testing-gui-x11.md).

## Batas/manual

- [ ] Batch query-focus runtime, disabled-state/tooltip/Light/Dark/HiDPI.
- [ ] Accessibility, IME/RTL, compositor shortcuts.
- [ ] Arch/Hyprland/Wayland, GPU/portal, scroll/caret visual.

Tidak ada body editing, AST serialization, glyph/caret mapping, selection/scroll
sync, general Preview Undo atau benchmark speed/RAM. Full projection/buffer
reconstruction tetap O(n); cap batch bukan batas ukuran dokumen atau RAM total.
Startup kedua 6b exit 101 lama tetap belum terdiagnosis, bukan diklaim diperbaiki
oleh batch atau pemisahan suite baru.
