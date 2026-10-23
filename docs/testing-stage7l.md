# Stage 7l — history draft lokal untuk panel satu baris (paragraf & sel tabel)

Melanjutkan [provenance sel tabel 7k](testing-stage7k.md). Stage7c memberi history
snapshot draft **lokal** untuk panel multiline; dua panel satu baris — paragraf root
(7a/7j) dan sel tabel (6h/6i/6p) — masih tidak punya Undo/Redo lokal sama sekali.
Salah ketik di panel itu hanya bisa dibatalkan dengan `Cancel` (kehilangan seluruh
draft) atau dengan meng-Apply nilai yang salah ke Source.

Stage7l menutup celah itu dengan **history draft lokal yang bounded** dan dua tombol
eksplisit `Undo draft`/`Redo draft` pada masing-masing panel:

- Panel paragraf satu baris: baris tombol `Apply`/`Cancel` tidak berubah;
  `Undo draft` (566,288) dan `Redo draft` (694,288) berada di baris navigasi 7j.
- Panel sel tabel: baris tombol `Apply`/`Cancel` tidak berubah; `Undo draft`
  (486,200) dan `Redo draft` (614,200) berada di baris navigasi sel.
- Lebar baris tombol sel tetap eksplisit 90/130/120/120 px, sehingga tinggi baris
  dan koordinat tidak bergeser.

Tidak ada shortcut keyboard baru: `text_input` Iced0.13 menangkap `Enter`, `Escape`,
`PageUp`, dan `PageDown` saat fokus, jadi undo/redo lokal **hanya** lewat tombol.
Tidak ada pengelompokan (grouping) berbasis waktu atau caret, tidak ada berbagi
history antar panel, tidak ada mutasi Source sebelum Apply, dan Apply tetap
mencatat **tepat satu** edge history Source.

## Kontrak

- **Bounded dan lokal.** Setiap sesi draft memiliki
  `multiline_draft_history::History` baru (100 record total Undo+Redo, 1 MiB teks
  mentah yang disimpan, ≤16 KiB per record). Record hanya `{text, caret}` — tidak
  ada snapshot, `state_id`, identitas file, operasi, atau status simpan.
- **Satu edit efektif = satu langkah.** `History::record(before, after)` menolak
  pasangan no-op, record dengan caret di luar batas UTF-8, dan record melebihi
  `MAX_BYTES`. Redo dibuang saat cabang baru dibuat; eviksi paling tua dari Undo
  lebih dulu, lalu Redo terjauh.
- **Restore hanya setelah prefiat.** `history_preview_paragraph`/
  `history_preview_cell` mengambil target lebih dulu, menolak bila
  `target.text.len() > MAX_BYTES`, lalu memanggil `History::step`. Nilai yang
  dipulihkan **divalidasi ulang**: paragraf memakai
  `Use one nonempty unindented line without control characters.`, sel memakai
  `Use one line, escape pipes as \|, and avoid an odd trailing backslash.`.
  Nilai invalid tetap dipulihkan sebagai teks mentah (seperti 7c) tetapi Apply
  tetap ditolak oleh validasi yang sama.
- **Batas, notice, dan error dipertahankan.** Notice `navigation_notice` dan error
  lama dibersihkan oleh restore efektif; restore no-op tidak menyentuh keduanya.
- **Source tidak tersentuh.** Undo/Redo draft tidak mengubah dokumen, snapshot,
  render key, caret Source, revisi, checkpoint, dirty flag, maupun stack
  Undo/Redo `PreviewHistory` Source. `Apply` tetap satu-satunya jalur ke Source dan
  tetap mencatat satu edge (`EditKind::PreviewParagraph`/`PreviewCell`), meskipun
  nilai yang di-Apply berasal dari restore lokal.
- **Sesi berakhir = stack hilang.** `Cancel`, Apply, atau penutupan no-op membuang
  seluruh stack lokal; membuka sesi berikutnya selalu mulai kosong.
- **Tombol dinonaktifkan tanpa state.** `can_undo`/`can_redo` diturunkan langsung
  dari stack, sehingga tombol disabled tidak pernah memindahkan fokus atau
  menghasilkan history palsu.

## Implementasi

- `src/multiline_draft_history.rs`: `History` kini `Clone + Debug` (dipakai dua
  panel) tetapi tetap tanpa identitas dokumen.
- `src/commands.rs`: `ParagraphDraftHistory(bool)` dan `CellDraftHistory(bool)` di
  samping `MultilineDraftHistory(bool)`.
- `src/app.rs`: dispatch dua pesan baru dan dua whitelist guard (panel draft dan
  `Source`/`Split`/`Preview`) diperluas sehingga history lokal tetap diblokir saat
  panel draft hidup atau saat guard lain aktif.
- `src/app/paragraph_edit.rs`: `record_of`, `history_preview_paragraph`, pemanggilan
  `history.record(before, after)` pada edit efektif, dan inisialisasi history baru
  di `open_preview_paragraph` serta `move_preview_paragraph`.
- `src/app/cell_edit.rs`: `record_of`, `history_preview_cell`, dan inisialisasi
  history di `open_preview_cell` serta `move_preview_cell`.
- `src/ui/paragraph_edit.rs` dan `src/ui/cell_edit.rs`: dua tombol baru per panel
  dengan `can_undo`/`can_redo`, tanpa mengubah tinggi baris atau lebar tombol.

**948 Cargo** = 943 sebelumnya + **5 Rust**: 3 controller (`src/app/tests.rs`) dan
2 UI (satu per panel).

Cakupan tes:

- `paragraph_draft_history_undo_redo_stay_local_and_a_new_edit_drops_redo`:
  Undo/Redo lokal mengembalikan teks dan caret draft sementara `buffer_text()`,
  `snapshot()`, `render_key()`, caret Source, dan stack Source tidak berubah; satu
  edit baru setelah Undo membuang cabang redo; `Apply` sesudah restore tetap
  meng-commit hanya teks draft yang terlihat, dan satu Undo Source mengembalikan
  seluruh dokumen;
- `paragraph_draft_history_restores_validity_and_ignores_oversized_or_absent_drafts`:
  Undo pada stack kosong bersifat no-op, restore membersihkan error validasi
  sebelumnya, nilai melebihi `MAX_BYTES` ditolak di batas input (nilai valid
  terakhir dipertahankan + error), dan pesan history setelah `Cancel` inert tanpa
  menyentuh history Source;
- `cell_draft_history_undo_restores_the_raw_cell_and_apply_commits_it`: alur yang
  sama untuk panel sel, dan Apply sesudah restore mencatat tepat satu edge Source;
- UI: `can_undo`/`can_redo` mengikuti stack, tombol disabled tidak menghasilkan
  pesan, history lokal tetap aktif saat draft ber-error, sedangkan navigasi dan
  `Apply` tetap mengikuti error tersebut.

## GUI

Suite ke-22 `draft-history-single` (independen, satu X11 display sendiri) menguji
kedua panel dalam **satu sesi Source**:

1. `Undo draft`/`Redo draft` paragraf memulihkan snapshot lokal berturut-turut
   (`payload` → `first` → `payload` → `first` → `payload`) tanpa satu byte Source
   berubah — diperiksa dengan refocus `Split → Source` lewat siklus mode;
2. dua snapshot paragraf lalu `Undo draft` + `Enter` meng-Apply nilai hasil restore,
   dan satu `ctrl+z` Source menghapus **hanya** edge itu;
3. seluruh dokumen diganti dengan fixture tabel **selagi Source masih fokus**
   (paste mentah, bukan `paste_from_clean` yang menuntut judul bersih);
4. alur yang sama untuk panel sel (Undo/Redo lokal, Apply satu edge, satu undo
   Source menghapus edge sel), lalu tiga `ctrl+z` Source berakhir pada dokumen
   `welcome` yang bersih dan close native yang normal.

Jumlah yang dipertahankan apa adanya: 24 `expect_copy` (semua `refocus: False`),
14 klik tombol, 11 `ctrl+shift+m`, 2 `Escape`, 2 `Return`, 4 `ctrl+z`, 7 `ctrl+a`,
7 `ctrl+v`, 2 `title_is_dirty`, 1 `request_close` + 1 `expect_native_exit`.
28 badan exercise lama tetap AST-identical `bb37223`; 21 gate lama dijalankan ulang
utuh tanpa perubahan bytes/koordinat/oracle/deadline/refocus.

Pemeriksaan suite baru dihitung dengan aturan yang sama seperti suite lama
(satu baris `PASS` per `expect_copy`, per `wait`, per `title_is_dirty`, dan per
`expect_native_exit`): **31 pemeriksaan**, sehingga totalnya **22 suite /
450 pemeriksaan** (419 + 31).

## Execution status

Local `cargo fmt --all -- --check`, **294 strict core**, **67 Python**
(2 tes baru untuk suite ini), dan `git diff --check` PASS.

[CI 39915156642](https://github.com/Indrawan007/Hylite/actions/runs/39915156642),
`d6ecb70`, job120286966415 **SUCCESS**, 2026-10-23 **07:01:48–07:15:01 UTC** /
**15:01:48–15:15:01 Makassar**. Format, strict core subset, Clippy
`-D warnings`, build, **948 Cargo**, seluruh **22 suite** lulus (termasuk
`Smoke test single-line draft history`), dan upload artifact lulus.

Artifact `12484788441`, **3.329.395 bytes**, expires 2026-10-30T07:13:41Z; digest
SHA256 yang dilaporkan API: `bb80e4a53e3a5cc0161dd365d1f0d5bef71b4969eb15ba4fd2f2d1447f7e6de2`
(zip artefak sendiri tidak dapat diunduh dari sandbox ini).

Commit dokumentasi ini sendiri juga lulus CI
[39916771054](https://github.com/Indrawan007/Hylite/actions/runs/39916771054)
(`601ba8d`, perubahan hanya pada `*.md`).

Percobaan CI sebelumnya gagal pada **suite GUI**, bukan pada gate: (1) copy
dilakukan saat fokus masih di Preview/Split sehingga balasan clipboard berupa
sentinels `hylite-copy-not-acknowledged-…`; (2) `paste_from_clean(table)` dipanggil
ketika judul dokumen memang masih kotor. Perbaikannya adalah refocus Source lewat
siklus mode penuh dan pemakaian helper paste mentah di dalam sesi Source — tidak
ada byte, koordinat, oracle, atau deadline yang dilonggarkan.

## Manual / limits

- [ ] Apakah dua tombol tambahan tetap terbaca pada HiDPI, font non-default, atau
  jendela sangat sempit.
- [ ] Arch/Hyprland/Wayland/portal/IME/RTL/GPU. X11 tiny-skia bukan proof target.

History ini lokal dan terbatas: bukan undo tak terbatas, bukan undo global, tidak
mengelompokkan ketikan, dan tidak menyimpan seleksi arah/anchor maupun scroll.
