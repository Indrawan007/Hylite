# Stage 6b — Undo/Redo task langsung di Preview

**Catatan historis:** cakupan task-only di bawah berlaku pada 6b.
[Successor 6c](testing-stage6c.md) menambah heading-level edge tervalidasi,
tanpa membuka history Source umum.

Melanjutkan [checkbox task 6a](testing-stage6a.md). Ini bukan Undo seluruh dokumen
di Preview atau WYSIWYG umum. **Preview hanya dapat menyeberangi transisi task
tervalidasi yang tepat berada di puncak stack history.**

## Perilaku

- Di Preview, Ctrl+Z membatalkan task, Ctrl+Shift+Z atau Ctrl+Y mengulangnya.
  Tombol Undo/Redo toolbar memakai eligibility yang sama dan nonaktif jika tidak
  ada task edge di puncak arah terkait. Mode tetap Preview, tanpa fokus Source.
- Tidak mencari task lebih tua melewati paste, ketikan, formatter atau edit
  Source lain. Jika Source edit berada di puncak, history Preview berhenti.
  Source/Split masih dapat Undo/Redo seluruh history seperti sebelumnya.
- Source Undo dapat mengekspos task edge lebih tua, lalu Preview boleh mengakses
  edge itu. Redo berhenti lagi bila edge berikutnya adalah Source. Tidak ada
  stack task terpisah atau reinterpretasi delta teks sebagai task edit.
- Native typing/formatting di Preview tetap ditolak. `Message::Undo/Redo` Source
  tetap memakai guard lama; Preview mempunyai message history tersendiri.
- Prompt/Open, mode Source/Split, Find/Go to Line aktif menolak request history
  Preview. Save/Export biasa mengizinkan dan token tidak dikonsumsi. Ack save
  tetap mengacu pada checkpoint konten yang dikirim, bukan state UI saat balasan.

## Metadata history

`History` membungkus Record dalam Entry dengan tag `preview_task` pada **edge
transaksi**, bukan checkpoint. Hanya jalur toggle task yang sudah lolos model/
snapshot/token validation memasok `EditKind::PreviewTask`; jenis ini tetap
Atomic, tidak coalesce. Source `change`, paste, formatter, insert/deletion tetap
bukan preview task walau perubahan teksnya kebetulan sama satu byte marker.

Undo memindahkan tag bersama edge ke redo dan sebaliknya. Branch replacement,
trim, budget nol/oversized dan penggantian dokumen menghapus tag bersama record.
Tidak ada ID set/map paralel yang dapat tumbuh atau menyisakan izin stale.
Peek eligibility O(1), tidak mem-parse model/copy teks atau mencari ke dalam stack.
API peek/undo/redo tetap mengembalikan Record sehingga rekonstruksi caret memakai
jalur sebelumnya. Batas gabungan 200 entries / 8 MiB **teks** tetap sama;
metadata bool/padding, buffer dan model bukan bagian budget byte teks itu.

Restore melalui `Document::restore_history`: source/caret/checkpoint kembali,
revision meningkat, dirty mengikuti saved state, cache preview invalidated.
Task click target lama stale setelah history restore walau bytes identik lagi.
History tidak perlu cache model warm karena metadata berasal dari transaksi
sebelumnya yang telah tervalidasi, bukan dari parse ulang saat Undo.

## Keyboard dan toolbar

`preview_history_shortcut` terpisah dari shortcut global/Source. Hanya exact Ctrl
atau Ctrl+Shift untuk z/Z dan exact Ctrl untuk y/Y; AltGr/Alt/logo/bare/Shift+Y
bukan shortcut ini. Hanya event berstatus **Ignored** boleh menghasilkan message
PreviewHistory. Event Source/query yang Captured tidak diduplikasi subscription.
Controller tetap memeriksa mode dan eligibility saat menerima message.

Toolbar Source/Split mempertahankan command Undo/Redo lama; di Preview memakai
PreviewHistory dan guard yang sama dengan keyboard. Label/hit area Undo/Redo
tidak berubah; hint Preview menjelaskan shortcut dan task-only scope.

## Tes

22 Rust baru:

- **7 core History**: empty/Source-only, tag transfer, Source frontier, branch,
  trim/zero/oversized, no-op tidak retag Source, dan Source coalescing setelah task.
- **6 Document**: scoped Undo/Redo + Source fixture boundary, Source barrier,
  same-byte Source diff bukan task, metadata/save/dirty, branch/replace, stale
  task click sesudah history kembali ke bytes identik.
- **3 shortcut**: exact combinations/case, modifiers invalid, Captured tidak
  menghasilkan command kedua.
- **6 controller**: mode/cache/source, Source history refusal, guards/panels,
  Save/Export checkpoint behavior, Source Undo tidak double, branch/new doc.

Oracle/budget/coalescing lama dipertahankan. Tes yang menginspeksi record private
sekarang membaca `entry.record` alih-alih record langsung; expected count/byte/
caret tidak diubah atau dilonggarkan.

GUI menambah **9 clipboard checks**: fixture Unicode/X lain/code/terminal tab;
Preview checkbox; Ctrl+Z, Ctrl+Shift+Z, Undo lagi, Ctrl+Y, toolbar Undo dan Redo;
Undo ganda berhenti sebelum Source paste sementara typing/bold tetap ditolak;
Source Undo terakhir sekali saja kembali welcome clean. Tiap hasil diperiksa
byte-exact melalui Split, lalu kembali Preview dengan mode cycle saja. Semua
Copy `refocus=False`; tiga klik hanya checkbox dan toolbar produk, bukan Source.
Dua screenshot `task-history-preview.png` dan `task-history-source.png` bukan
pixel oracle. Helper Python mengunci expected bytes/urutan/shortcut/tanpa refocus.

## Status eksekusi

[CI 39737235985](https://github.com/Indrawan007/Hylite/actions/runs/39737235985)
**success** pada `0df378803bbe19672abb9391ae868306a82a4053`: **461 Cargo**,
**179 core strict** (subset Cargo), **22 Python**, **101 GUI X11** (87 clipboard,
12 title, 2 exit native normal), format/build debug/Clippy `-D warnings` lulus.
Job `119685695986` berjalan 2026-10-22 06:57:38–07:01:20 UTC. Artifact
`gui-x11-smoke` ID `12407691827`, 1.188.202 bytes, belum expired saat API diperiksa;
screenshot belum direview visual. [Bukti GUI terperinci](testing-gui-x11.md).

Percobaan [CI 39736591093](https://github.com/Indrawan007/Hylite/actions/runs/39736591093)
lulus format/Clippy/build, 179 core dan 461 Cargo. Probe history keyboard/toolbar
baru dan boundary Source sudah lewat, tetapi gate lifecycle gagal saat proses
kedua startup (exit 101). Root diagnostic berada di bagian annotation yang
terpotong: pelapor lama membuat chunk 6000 karakter sedangkan Checks API
menyimpan hanya 4096. Reporter sekarang membuat chunk 3000 (total tail tetap
24.000 karakter, maksimum 8 annotation), dengan regression test menjaga panic pada offset
4500 tidak hilang. Ini perbaikan diagnostik, bukan retry aplikasi, relaksasi
timeout atau pelonggaran pemeriksaan lifecycle. Run berikutnya di atas lulus semua gate, tetapi akar exit 101 pertama **belum
diketahui**, tidak diklaim diperbaiki oleh reporter atau diasumsikan sebagai
masalah fitur/infra tertentu. Tidak ada retry startup otomatis yang ditambahkan.

## Batas/manual — belum dinyatakan lulus

- [ ] Keyboard layout non-default, IME/RTL, aksesibilitas/touchpad.
- [ ] Visual disabled state/hint dan nested checkbox pada Light/Dark/HiDPI.
- [ ] Query focus runtime dan shortcut ketika panel/context berubah sangat cepat.
- [ ] Arch + Hyprland/Wayland, GPU dan portal file.

Tidak mengubah serializer, lockfile, dependensi, worker file, parser/provenance
token atau batas task kanonis 6a. Tidak memperlonggar native Preview editing,
expected bytes, timeout GUI, atau menambahkan hook tes aplikasi. Rekonstruksi
buffer tetap O(n); tidak ada benchmark latensi/RAM atau editable AST/glyph map.
