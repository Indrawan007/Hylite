# Stage 7m — keyboard Undo/Redo draft untuk panel satu baris (Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z)

Melanjutkan [history draft lokal 7l](testing-stage7l.md). Stage7l memberi dua panel
satu baris (`text_input` paragraf root dan sel tabel) tombol `Undo draft`/`Redo draft`
tetapi **tidak** ada jalur keyboard: selama panel draft terbuka, `Ctrl+Z` tidak
melakukan apa pun (history Source memang diblokir), sehingga pengguna harus
mengangkat tangan ke tombol.

Stage7m menutup celah itu **tanpa widget baru dan tanpa mengubah satu byte pun**:

- `Ctrl+Z` = `Undo draft`, `Ctrl+Y` dan `Ctrl+Shift+Z` = `Redo draft` pada panel
  paragraf satu baris **dan** panel sel tabel.
- Hanya `Ctrl` tepat (atau `Ctrl+Shift` untuk redo). AltGr (`Ctrl+Alt`), Logo,
  `Shift+Z` tanpa Ctrl, dan `Shift+Y` ditolak supaya seleksi/komposisi karakter tidak
  pernah memicu history.
- Tanpa panel draft terbuka, gestur yang sama tetap menjadi history
  Source/Preview seperti sebelumnya.
- Footer kedua panel kini menyebut shortcut-nya. Baris `Apply`/`Cancel`, baris
  navigasi, tinggi baris, dan lebar tombol lama tidak berubah.

Yang secara eksplisit **tidak** dikerjakan: tidak ada shortcut commit baru
(`Enter` tetap `on_submit` Apply), tidak ada pengelompokan ketikan berbasis waktu,
tidak ada berbagi history antar panel, tidak ada undo global, dan tidak ada
perubahan pada semantik `Apply`.

## Kontrak

- **Rute netral, keputusan di controller.** `commands::draft_history_shortcut`
  hanya mem-parse tombol menjadi `Message::DraftHistory(redo)`; ia tidak tahu panel
  mana yang terbuka. `Hylite::draft_history` yang memutuskan: panel paragraf →
  history paragraf, panel sel → history sel, tidak ada panel → `preview_undo_redo`.
- **Status event diabaikan untuk rute draft.** `text_input` yang fokus melaporkan
  `Ctrl+Z` sebagai *captured* walaupun ia tidak punya undo stack, jadi rute draft
  tidak boleh menuntut `Status::Ignored`. Sebaliknya `preview_history_shortcut`
  (dipakai binding `text_editor` multiline) tetap menuntut `Status::Ignored`, dan
  pesan netral hanya jatuh ke `preview_undo_redo` yang bersyarat mode `Preview`.
  Konsekuensinya tidak ada undo ganda: di `Source`/`Split`, `Ctrl+Z` tetap
  dijalankan widget Source yang menangkap event, sedangkan fallback Preview no-op
  di mode itu.
- **Satu panel, satu stack.** `draft_history_panel()` hanya mengembalikan salah
  satu panel; dua panel satu baris tidak pernah hidup bersamaan dan panel draft
  multiline punya binding sendiri (pesan netral diblokir guard multiline).
- **Snapshot tetap milik 7c/7l.** `multiline_draft_history::History` dipakai ulang:
  100 record / 1 MiB / ≤16 KiB per record, satu edit efektif = satu langkah, cabang
  baru membuang redo. Tidak ada state baru, file, thread, atau dependency.
- **Restore = nilai draft saja.** Undo/Redo draft tidak menyentuh dokumen, snapshot,
  render key, caret Source, revisi, checkpoint, dirty flag, maupun stack Source;
  validasi draft dihitung ulang sehingga error hilang saat nilai valid kembali.
  `Apply` tetap satu-satunya jalur ke Source dan tetap mencatat satu edge.

## Implementasi

- `src/commands.rs`: `history_keys` (parser murni Ctrl/Ctrl+Shift),
  `draft_history_shortcut`, `Message::DraftHistory(bool)`, dan
  `DraftHistoryPanel`. `preview_history_shortcut` dipertahankan sebagai pembungkus
  bergerbang status untuk binding multiline.
- `src/app.rs`: `draft_history(redo)` + `draft_history_panel()`, dispatch
  `Message::DraftHistory`, dan penambahan pesan itu ke whitelist guard panel
  paragraf serta panel sel.
- `src/ui/paragraph_edit.rs` / `src/ui/cell_edit.rs`: teks footer menyebut
  `Ctrl+Z/Ctrl+Y = Undo/Redo draft`; affordance, tombol, tinggi, dan lebar tetap.

**951 Cargo** = 948 sebelumnya + **3 Rust**: 1 `commands` (parser + gerbang status,
termasuk bahwa `Ctrl+Shift+Y` bukan redo) dan 2 controller (`src/app/tests.rs`:
routing hanya ke satu panel, dan fallback ke `preview_undo_redo` yang tetap
Preview-only serta tidak menyentuh Source saat draft terbuka). **69 Python** = 67 +
2 helper suite baru.

## GUI

Suite ke-23 `draft-history-keys` (independen, satu X11 display sendiri) menguji
kedua panel dalam satu sesi Source:

1. `Ctrl+Z` mengembalikan snapshot lokal paragraf, `Ctrl+Y` mengulanginya,
   `Ctrl+Shift+Z` mengulang setelah Undo, dan `Ctrl+Z` kedua pada stack kosong
   tidak melakukan apa pun — semuanya dibuktikan lewat isi clipboard draft;
2. `Ctrl+Shift+M` ke Source membuktikan tidak ada satu byte Source yang berubah;
3. fixture tabel mengganti seluruh dokumen dari dalam Source, lalu panel sel
   diuji dengan `Ctrl+Z` dan `Ctrl+Shift+Z` yang sama;
4. satu `Ctrl+Z` Source menghapus edge penggantian tabel, `Ctrl+Z` berikutnya
   kembali ke `welcome` bersih, lalu close native normal.

Jumlah yang dipertahankan apa adanya: 17 `expect_copy` (semua `refocus: False`),
3 klik, 5 `ctrl+shift+m`, 7 `ctrl+z`, 1 `ctrl+y`, 2 `ctrl+shift+z`, 2 `Escape`,
3 `ctrl+a`, 3 `ctrl+v`, 1 `paste_from_clean`, 2 `title_is_dirty`, 1
`request_close` + 1 `expect_native_exit`. 28 badan exercise lama tetap
AST-identical `bb37223`; 22 gate lama dijalankan ulang utuh tanpa perubahan
bytes/koordinat/oracle/deadline/refocus.

## Execution status

Local `cargo fmt --all -- --check`, **294 strict core**, **69 Python**,
`git diff --check` PASS.

[CI 39922474388](https://github.com/Indrawan007/Hylite/actions/runs/39922474388),
`d1031c3`, job120310748817 **SUCCESS**, 2026-10-23 **08:07:46–08:19:46 UTC** /
**16:07:46–16:19:46 Makassar**. Format, strict core subset, Clippy
`-D warnings`, build, **951 Cargo**, seluruh **23 suite** (**462 pemeriksaan**:
426 clipboard — 307 Source + 119 draft input; 12 title konfirmasi; 24 exit native
normal) lulus, dan upload artifact lulus.

Artifact `12488168397`, **3.334.806 bytes**, expires 2026-10-30T08:19:41Z; digest
SHA256 yang dilaporkan API: `745dfeffea59a0ea523f063977816d1b806d8716bed981f65638e894b8f6c6ec`
(zip artefak tidak dapat diunduh dari sandbox ini).

Commit dokumentasi ini sendiri juga lulus CI
[39924092281](https://github.com/Indrawan007/Hylite/actions/runs/39924092281)
(`7fcf454`, hanya `*.md`).

Percobaan CI sebelumnya gagal pada **kompilasi**, bukan pada gate: (1) rute draft
dibuat dengan closure yang menangkap state padahal `event::listen_with` hanya
menerima *fn pointer* → diganti pesan netral + routing di controller; (2) parameter
status menjadi tidak terpakai setelah penyederhanaan; (3) suite GUI gagal karena
rute masih menuntut `Status::Ignored` sementara `text_input` fokus melaporkan
`Ctrl+Z` sebagai *captured*. Semua perbaikan dilakukan pada kode, tanpa melonggarkan
bytes, koordinat, oracle, atau deadline.

## Manual / limits

- [ ] Perilaku shortcut saat compositor Hyprland sudah memakai `Ctrl+Z`/`Ctrl+Y`
  global (di X11 virtual tidak ada klaim binding compositor).
- [ ] `Ctrl+Z` pada keyboard layout non-Latin/IME, dan perilaku saat `text_input`
  memakai metode input native.
- [ ] Arch/Hyprland/Wayland/portal/GPU. X11 tiny-skia bukan proof target.

History tetap lokal dan terbatas: satu ketikan = satu langkah (tanpa grouping), dan
stack hilang saat `Cancel`, `Apply`, atau perpindahan draft.
