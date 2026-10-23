# Stage 7n — umpan balik batas history draft dan penguatan properti

Melanjutkan [keyboard history 7m](testing-stage7m.md). Setelah 7m, `Ctrl+Z` memindahkan
stack draft lokal tanpa terlihat apakah masih ada snapshot. Bila stack kosong,
gestur itu **mati tanpa penjelasan**: tidak ada teks, tidak ada perubahan, dan
pengguna tidak bisa membedakan "sudah di awal sesi draft" dari "aplikasi tidak
merespons". Panel sel juga punya satu ketidakkonsistenan kecil: mengetik
membersihkan notice lama, sedangkan panel paragraf tidak.

Stage7n menutup keduanya dan menambah penguatan properti untuk batas history:

- `Ctrl+Z` pada stack Undo yang kosong menampilkan
  `Nothing to undo in this draft; Source is unchanged.`; `Ctrl+Y`/`Ctrl+Shift+Z`
  pada stack Redo kosong menampilkan `Nothing to redo in this draft; Source is unchanged.`
- Restore yang ditolak karena record melebihi `MAX_BYTES` menampilkan
  `That draft snapshot is too large to restore; draft kept. Source unchanged.`
- Ketikan efektif berikutnya (termasuk paste kebesaran) membersihkan notice itu,
  sama seperti panel sel, sehingga pesan lama tidak menggantung.
- Restore yang berhasil tetap membersihkan notice dan meng-arms ulang validasi.

Tidak ada tombol, tinggi baris, koordinat, atau semantik `Apply` yang berubah; tidak
ada suite GUI baru karena isi footer tidak dapat diobservasi oleh harness clipboard
(mengikuti pola [Stage 7k](testing-stage7k.md): seluruh gate lama dijalankan ulang).

## Kontrak

- **Notice, bukan error.** Pesan memakai `navigation_notice`, bukan `error`:
  `Apply` tetap aktif dan validasinya tidak berubah. Prioritas tampilan footer tetap
  error → `navigation_notice` → teks default.
- **Tidak ada mutasi.** Gestur yang tidak bisa bergerak tidak menyentuh nilai draft,
  teks Source, snapshot, render key, caret, revisi, checkpoint, dirty flag, maupun
  kedua stack history. Test menegaskan stack dan `bytes` byte-identical sebelum dan
  sesudah penolakan.
- **Cakupan pesan terbatas.** Hanya gestur history lokal yang melaporkan; ketikan,
  batas ketik, dan balasan asinkron tetap senyap (aturan 7i).
- **Batasan yang tidak diubah.** `Ctrl+PageUp`/`Ctrl+PageDown` tetap **bukan** rute
  commit untuk panel satu baris (`text_input`), dan suite 7j mengunci perilaku itu
  sebagai oracle. Mengubahnya adalah keputusan produk terpisah, bukan bagian 7n.

## Penguatan properti (tanpa perubahan produk)

Test baru di `src/multiline_draft_history.rs` memakai RNG deterministik (LCG dengan
seed eksplisit, tanpa dependency baru) dan sebuah **reference model** yang menghitung
ulang total byte dari nol alih-alih memelihara counter seperti produksi, sehingga
drift akuntansi terdeteksi:

1. `generated_history_matches_an_independently_bookkept_reference` — 5 seed × 400
   operasi campuran `record`/`step` dengan ukuran teks 1 B–16 KiB (termasuk yang
   menyentuh batas 1 MiB), membandingkan hasil, `can_undo`/`can_redo`, isi kedua
   stack, dan `bytes` dengan model pada setiap langkah.
2. `eviction_always_takes_the_oldest_undo_then_the_farthest_redo` — Undo penuh (100),
   lalu 40 Undo (Redo terisi), lalu 60 record baru; urutan jendela yang dipertahankan
   harus sama persis dengan model, bukan hanya jumlahnya.
3. `refused_records_and_steps_leave_both_stacks_byte_identical` — record oversized,
   caret di tengah scalar UTF-8, dan caret di luar batas ditolak tanpa mengubah satu
   record pun.

Test controller baru:

- `empty_draft_history_gesture_reports_and_never_mutates_anything` — pesan untuk
  Undo/Redo kosong di kedua panel, pembersihan notice oleh ketikan dan oleh restore
  efektif, serta Source/stack Source yang tidak berubah.
- `generated_draft_history_messages_never_touch_source_or_its_history` — 6 putaran
  campuran ketikan valid/invalid dengan `DraftHistory` dan `ParagraphDraftHistory`;
  teks Source, snapshot, caret, dan stack Source tetap identik, dan hanya `Apply`
  eksplisit yang membuat satu edge yang bisa dibatalkan `PreviewHistory`.

**956 Cargo** = 951 sebelumnya + **5 Rust** (3 properti `multiline_draft_history`,
2 controller). **297 strict core** = 294 + 3. **69 Python** dan **23 suite GUI / 462
pemeriksaan** tidak berubah.

## Execution status

Local `cargo fmt --all -- --check`, **297 strict core**, **69 Python**,
`git diff --check` PASS. Verification lokal memakai toolchain 1.88.0 yang dipasang
ulang dari npm (`@rustbin/*`) karena sandbox di-reset; `cargo build` penuh tetap
tidak mungkin di sini (crates.io diblokir), jadi suite Cargo/Clipy diverifikasi CI.

## Manual / limits

- [ ] Apakah footer dua panel masih terbaca saat teks pesan ini bertambah panjang
  pada HiDPI atau font non-default.
- [ ] Perilaku `Ctrl+Z` saat komposisi IME aktif di `text_input` (belum diuji nyata).
- [ ] Arch/Hyprland/Wayland/portal/GPU. X11 tiny-skia bukan proof target.

Stage ini tidak menambah undo tak terbatas, tidak mengelompokkan ketikan, dan tidak
membuat stack draft bertahan melewati `Cancel`/`Apply`/perpindahan draft.
